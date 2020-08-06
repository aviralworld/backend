use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use slog::{debug, error, Logger};
use url::Url;
use uuid::Uuid;
use warp::filters::multipart::{form, FormData, Part};
use warp::http::StatusCode;
use warp::reject;
use warp::reply::{json, with_status, Json, Reply, WithStatus};
use warp::Filter;

use crate::db::Db;
use crate::errors::BackendError;
use crate::io::parse_upload;
use crate::recording::UploadMetadata;
use crate::store::Store;

// create, delete, update, retrieve, count

#[derive(Deserialize, Serialize)]
struct StorageResponse {
    status: Response,
    key: Option<String>,
}

#[derive(Deserialize, Serialize)]
enum Response {
    Ok,
    Error,
}

/// The maximum form data size to accept. This should be enforced by the HTTP gateway, so on the Rust side it’s set to an unreasonably large number.
const MAX_CONTENT_LENGTH: u64 = 2 * 1024 * 1024 * 1024;

// TODO accept environment as single `Environment` struct (causes all sorts of reference and lifetime and pointers issues)
pub fn make_upload_route<'a, O: 'a>(
    logger: Arc<Logger>,
    db: Arc<impl Db + Sync + Send + 'a>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>> + 'a>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError> + Send + Sync + 'a>,
) -> impl warp::Filter<Extract = (impl Reply,), Error = reject::Rejection> + Clone + 'a {
    let store = store.clone();
    let db = db.clone();
    let logger1 = logger.clone();
    let logger2 = logger.clone();
    let checker = checker.clone();

    // TODO this should stream the body from the request, but warp
    // doesn't support that yet
    warp::path("recordings")
        .and(warp::post())
        .and(form().max_length(MAX_CONTENT_LENGTH))
        .and_then(
            // this can be simplified once async closures are stabilized (rust/rust-lang#62290)
            move |content: FormData| -> BoxFuture<Result<WithStatus<Json>, reject::Rejection>> {
                process_upload(
                    logger1.clone(),
                    db.clone(),
                    store.clone(),
                    checker.clone(),
                    content,
                )
                .boxed()
            },
        )
        .recover(move |r| format_rejection(logger2.clone(), r))
}

async fn process_upload<O>(
    logger: Arc<Logger>,
    db: Arc<impl Db>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError>>,
    content: FormData,
) -> Result<WithStatus<Json>, reject::Rejection> {
    debug!(logger, "Parsing submission...");
    let upload = parse_upload(content).await?;
    debug!(logger, "Verifying audio contents...");
    let verified_audio = verify_audio(logger.clone(), checker, upload.audio).await?;

    debug!(logger, "Writing metadata to database...");
    let id = save_recording_metadata(logger.clone(), db.clone(), upload.metadata).await?;
    let id_as_str = format!("{}", id);
    let logger = Arc::new(logger.new(slog::o!("id" => id_as_str.clone())));

    let log_error = |e: BackendError| {
        error!(logger, "failed to save"; "id" => &id_as_str, "error" => format!("{:?}", e));
        reject::custom(e)
    };

    debug!(logger, "Saving recording to store...");
    save_upload_audio(logger.clone(), store.clone(), &id, verified_audio)
        .await
        .map_err(&log_error)?;

    debug!(logger, "Updating recording URL...");
    update_recording_url(logger.clone(), db.clone(), store.clone(), &id)
        .await
        .map_err(&log_error)?;

    debug!(logger, "Sending response...");
    let response = StorageResponse {
        status: Response::Ok,
        key: Some(id_as_str.clone()),
    };

    Ok(with_status(json(&response), StatusCode::OK))
}

async fn verify_audio(
    _logger: Arc<Logger>,
    checker: Arc<impl Fn(&[u8]) -> Result<(), BackendError>>,
    audio: Part,
) -> Result<Vec<u8>, BackendError> {
    use crate::io;

    let audio_data = io::part_as_vec(audio)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;

    checker(&audio_data)?;

    Ok(audio_data)
}

async fn save_recording_metadata(
    _logger: Arc<Logger>,
    db: Arc<impl Db>,
    metadata: Part,
) -> Result<Uuid, reject::Rejection> {
    use crate::io;

    let raw_metadata = io::part_as_vec(metadata)
        .await
        .map_err(|_| BackendError::MalformedFormSubmission)?;
    let metadata: UploadMetadata = serde_json::from_slice(&raw_metadata)
        .map_err(|e| BackendError::MalformedUploadMetadata(e))?;

    // TODO handle the name or ID not being unique
    let new_recording = db.insert(metadata).await?;
    let id = new_recording.id();

    Ok(*id)
}

async fn save_upload_audio<O>(
    _logger: Arc<Logger>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    key: &Uuid,
    upload: Vec<u8>,
) -> Result<(), BackendError> {
    store.save(key, upload).await?;

    Ok(())
}

async fn update_recording_url<O>(
    _logger: Arc<Logger>,
    db: Arc<impl Db>,
    store: Arc<impl Store<Output = O, Raw = Vec<u8>>>,
    key: &Uuid,
) -> Result<Url, BackendError> {
    let url = store
        .get_url(&key)
        .map_err(|e| BackendError::FailedToGenerateUrl { source: e })?;

    db.update_url(key, &url).await?;

    Ok(url)
}

async fn format_rejection(
    logger: Arc<Logger>,
    rej: reject::Rejection,
) -> Result<WithStatus<Json>, reject::Rejection> {
    if let Some(e) = rej.find::<BackendError>() {
        error!(logger, "Backend error"; "error" => format!("{:?}", e));
        let response = StorageResponse {
            status: Response::Error,
            key: None,
        };

        return Ok(with_status(json(&response), status_code_for(e)));
    }

    Err(rej)
}

fn status_code_for(e: &BackendError) -> StatusCode {
    use BackendError::*;

    match e {
        BadRequest | TooManyStreams(..) => StatusCode::BAD_REQUEST,
        WrongMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        PartsMissing => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Once;

    use once_cell::sync::OnceCell;
    use serde::Deserialize;
    use slog::{self, o, Logger};

    use crate::db::Db;
    use crate::errors;
    use crate::store::mock::MockStore;

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Reply {
        status: String,
        key: Option<String>,
    }

    static SLOG_SCOPE_GUARD: OnceCell<slog_scope::GlobalLoggerGuard> = OnceCell::new();

    #[tokio::test]
    async fn uploading_works() {
        read_config();
        initialize_global_logger();

        static BOUNDARY: &str = "thisisaboundary1234";

        let content_type = multipart_content_type(&BOUNDARY);

        let store = MockStore::new("ogg");

        let logger = slog_scope::logger().new(o!("test" => "uploading_works"));
        let logger_arc = Arc::new(logger.clone());

        let checker = make_wrapper_for_test(logger_arc.clone());
        let db = make_db().await;

        let filter = super::make_upload_route(
            logger_arc.clone(),
            Arc::new(db),
            Arc::new(store),
            Arc::new(checker),
        );

        let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let base_path = Path::new(&cargo_dir);
        let file_path = base_path.join("tests").join("opus_file.ogg");

        let body = fs::read("tests/simple_metadata.json").expect("read simple_metadata.json");

        let response = upload_file(&file_path, &content_type, BOUNDARY.as_bytes(), &body)
            .reply(&filter)
            .await;

        let status = response.status();
        let body = String::from_utf8_lossy(response.body()).into_owned();

        assert!(status.is_success());

        let deserialized: Reply = serde_json::from_str(&body).expect("parse response as JSON");
        assert_eq!(deserialized.status, "Ok", "response status must be okay");
        assert!(
            deserialized.key.unwrap() != "",
            "response must provide non-blank key"
        );
    }

    #[tokio::test]
    async fn bad_requests_fail() {
        use bytes::Bytes;
        use warp::http::StatusCode;

        fn assert_failed(
            response: warp::http::Response<Bytes>,
            expected_status: u16,
            verify_error_type: &dyn Fn(StatusCode) -> bool,
        ) {
            let status = response.status();
            assert!(verify_error_type(status));
            assert_eq!(status.as_u16(), expected_status);
        }

        read_config();
        initialize_global_logger();

        let store = MockStore::new("ogg");

        let logger = slog_scope::logger();
        let logger_arc = Arc::new(logger);

        let checker = make_wrapper_for_test(logger_arc.clone());
        let db = make_db().await;

        let filter = super::make_upload_route(
            logger_arc.clone(),
            Arc::new(db),
            Arc::new(store),
            Arc::new(checker),
        );

        {
            let response = warp::test::request()
                .path("/recordings/")
                .method("POST")
                .header("content-type", "text/plain")
                .header("content-length", 0)
                .reply(&filter)
                .await;

            assert_failed(response, 400, &|s: StatusCode| s.is_client_error());
        }
    }

    const NEWLINE: &[u8] = "\r\n".as_bytes();
    const METADATA_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"metadata\"\r\n\r\n".as_bytes();
    const AUDIO_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/ogg\r\n\r\n"
            .as_bytes();
    const BOUNDARY_LEADER: &[u8] = &[b'-', b'-'];

    fn initialize_global_logger() {
        SLOG_SCOPE_GUARD.get_or_init(|| slog_envlogger::init().expect("initialize slog-envlogger"));
    }

    fn read_config() {
        static INITIALIZED_CONFIG: Once = Once::new();

        INITIALIZED_CONFIG.call_once(|| {
            dotenv::dotenv().expect("read .env");
        });
    }

    fn upload_file(
        path: impl AsRef<Path>,
        content_type: &str,
        boundary: &[u8],
        metadata: &[u8],
    ) -> warp::test::RequestBuilder {
        let data =
            fs::read(path.as_ref()).expect(&format!("read file {:?}", path.as_ref().display()));
        let body = make_multipart_body(boundary, metadata, &data);

        warp::test::request()
            .path("/recordings/")
            .method("POST")
            .header("content-type", content_type)
            .header("content-length", body.len())
            .body(body)
    }

    fn make_wrapper_for_test(
        logger: Arc<Logger>,
    ) -> impl Fn(&[u8]) -> Result<(), errors::BackendError> {
        use crate::audio;

        audio::make_wrapper(
            logger.clone(),
            env::var("BACKEND_FFPROBE_PATH").ok(),
            env::var("BACKEND_MEDIA_CODEC")
                .expect("must define BACKEND_MEDIA_CODEC environment variable"),
            env::var("BACKEND_MEDIA_FORMAT")
                .expect("must define BACKEND_MEDIA_FORMAT environment variable"),
        )
    }

    async fn make_db() -> impl Db {
        use sqlx::Pool;
        use tokio::task;

        use crate::config::get_variable;
        use crate::db::PgDb;

        let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
        let pool = Pool::new(&connection_string)
            .await
            .expect("create PgPool from BACKEND_DB_CONNECTION_STRING");

        static INITIALIZED_DB: Once = Once::new();

        task::spawn_blocking(move || {
            INITIALIZED_DB.call_once(|| {
                let connection_string = connection_string.clone();

                if env::var("BACKEND_TEST_INITIALIZE_DB").unwrap_or("0".to_owned()) == "1" {
                    initialize_db_for_test(&connection_string);
                }
            });
        })
        .await
        .expect("must spawn blocking task");

        PgDb::new(pool)
    }

    fn initialize_db_for_test(connection_string: &str) {
        use movine::Movine;
        // it would make more sense to use `tokio-postgres`, which is
        // inherently async and which `postgres` is a sync wrapper
        // around, but `movine` expects this
        use postgres::{Client, NoTls};

        let mut client = Client::connect(&connection_string, NoTls)
            .expect("create postgres::Client from BACKEND_DB_CONNECTION_STRING");
        let mut movine = Movine::new(&mut client);

        if movine.status().is_err() {
            movine.initialize().expect("initialize movine");
        }

        movine.up().expect("run movine migrations");

        let sql = fs::read_to_string("tests/data.sql").expect("read SQL file");
        client.simple_query(&sql).expect("execute SQL file");
    }

    fn make_multipart_body(boundary: &[u8], metadata: &[u8], content: &[u8]) -> Vec<u8> {
        let boundary = boundary_with_leader(boundary);
        let boundary = boundary.as_slice();

        let mut parts = vec![
            boundary,
            NEWLINE,
            METADATA_HEADER,
            metadata,
            NEWLINE,
            boundary,
            NEWLINE,
        ];

        parts.push(AUDIO_HEADER);
        parts.push(&content);
        parts.push(NEWLINE);
        parts.push(boundary);
        parts.push("--".as_bytes());
        parts.push(NEWLINE);

        parts.concat()
    }

    fn boundary_with_leader(boundary: &[u8]) -> Vec<u8> {
        let parts = &[BOUNDARY_LEADER, boundary];
        parts.concat()
    }

    fn multipart_content_type(boundary: &str) -> String {
        format!("multipart/form-data; boundary={}", boundary)
    }
}
