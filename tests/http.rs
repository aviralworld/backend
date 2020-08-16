use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Once;

use once_cell::sync::OnceCell;
use serde::Deserialize;
use slog::{self, o, Logger};
use url::Url;
use warp::http::StatusCode;

use backend::config::get_variable;
use backend::db::Db;
use backend::errors;
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreationResponse {
    message: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetrievalResponse {
    id: String,
    url: String,
    created_at: i64,
    updated_at: i64,
    category: RelatedLabel,
    unlisted: bool,
    parent: Option<String>,
    name: String,
    age: Option<RelatedLabel>,
    gender: Option<RelatedLabel>,
    location: Option<String>,
    occupation: Option<String>,
}

impl RetrievalResponse {
    fn into_comparable(self) -> ComparableRetrievalResponse {
        ComparableRetrievalResponse {
            id: self.id,
            url: self.url,
            created_at: self.created_at,
            category: self.category,
            unlisted: self.unlisted,
            parent: self.parent,
            name: self.name,
            age: self.age,
            gender: self.gender,
            location: self.location,
            occupation: self.occupation,
        }
    }
}

// this is RetrievalResponse minus updated_at
#[derive(Debug, PartialEq)]
struct ComparableRetrievalResponse {
    id: String,
    url: String,
    created_at: i64,
    category: RelatedLabel,
    unlisted: bool,
    parent: Option<String>,
    name: String,
    age: Option<RelatedLabel>,
    gender: Option<RelatedLabel>,
    location: Option<String>,
    occupation: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RelatedLabel(i8, String);

static SLOG_SCOPE_GUARD: OnceCell<slog_scope::GlobalLoggerGuard> = OnceCell::new();

const BOUNDARY: &str = "thisisaboundary1234";

#[tokio::test]
async fn api_works() {
    test_non_existent_recording().await;

    let content_type = multipart_content_type(&BOUNDARY);

    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let base_path = Path::new(&cargo_dir);
    let file_path = base_path.join("tests").join("opus_file.ogg");

    let id = test_upload(&file_path, &content_type).await;
    test_duplicate_upload(&file_path, &content_type).await;

    let children: serde_json::Value = serde_json::from_reader(
        fs::File::open("tests/simple_metadata_children.json")
            .expect("open simple_metadata_children.json"),
    )
    .expect("parse simple_metadata_children.json");

    let ids = test_uploading_children(&file_path, &content_type, &id, children).await;

    let id_to_delete = ids.iter().skip(1).next().expect("get second child ID");
    test_deletion(id_to_delete, &id, &ids).await;

    test_count().await;

    test_updating(ids.iter().last().expect("get last child ID")).await;
}

async fn test_upload(file_path: impl AsRef<Path>, content_type: impl AsRef<str>) -> String {
    let bytes = fs::read("tests/simple_metadata.json").expect("read simple_metadata.json");

    let response = upload_file(
        file_path.as_ref(),
        content_type.as_ref(),
        BOUNDARY.as_bytes(),
        &bytes,
    )
    .reply(&make_upload_filter("test_upload").await)
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = String::from_utf8_lossy(response.body()).into_owned();

    let headers = response.headers();

    let location = Url::parse(
        headers
            .get("location")
            .expect("get location header")
            .to_str()
            .expect("convert location header to string"),
    )
    .expect("parse location header");
    assert_eq!(location.domain(), Some("www.example.com"));
    let segments = location
        .path_segments()
        .expect("get location path segments")
        .collect::<Vec<_>>();
    assert_eq!(segments[0], "recs");
    assert_eq!(segments.len(), 2);

    let id = serde_json::from_str::<CreationResponse>(&body)
        .expect("parse response as JSON")
        .id
        .expect("get ID from response");

    assert_ne!(id, "", "response must provide non-blank key");

    id
}

async fn test_duplicate_upload(file_path: impl AsRef<Path>, content_type: impl AsRef<str>) {
    let filter = make_upload_filter("check_duplicate_upload").await;

    // ensure the same name cannot be reused
    let bytes = fs::read("tests/duplicate_metadata.json").expect("read duplicate_metadata.json");

    let response = upload_file(
        &file_path,
        content_type.as_ref(),
        BOUNDARY.as_bytes(),
        &bytes,
    )
    .reply(&filter)
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = String::from_utf8_lossy(response.body()).into_owned();

    let deserialized: CreationResponse =
        serde_json::from_str(&body).expect("parse response as JSON");
    assert!(
        deserialized.id.is_none(),
        "error response must not include key"
    );
    assert_eq!(
        deserialized.message,
        Some("name already exists in database".to_owned()),
        "error response must mention name already exists in database"
    );
}

async fn test_uploading_children(
    file_path: impl AsRef<Path>,
    content_type: impl AsRef<str>,
    parent: &str,
    mut children: serde_json::Value,
) -> Vec<String> {
    let mut ids = vec![];

    for mut child in children
        .as_array_mut()
        .expect("get array from simple_metadata_children.json")
    {
        let child_id = test_uploading_child(
            file_path.as_ref(),
            content_type.as_ref(),
            parent,
            &mut child,
        )
        .await;

        for child_id in child_id {
            ids.push(child_id);
        }
    }

    let children_filter = make_children_filter("api_works").await;

    {
        let request = warp::test::request()
            .path(&format!("/recs/id/{id}/children/", id = parent))
            .method("GET")
            .reply(&children_filter)
            .await;
        assert_eq!(request.status(), StatusCode::OK);
        let returned_ids = parse_children_ids(request.body());
        assert_eq!(ids, returned_ids);
    }

    ids
}

async fn test_uploading_child(
    file_path: impl AsRef<Path>,
    content_type: impl AsRef<str>,
    id: &str,
    child: &mut serde_json::Value,
) -> Option<String> {
    let filter = make_upload_filter("test_uploading_child").await;
    let object = child.as_object_mut().expect("get child as object");
    let unlisted = object["unlisted"].as_bool().unwrap_or(false);
    object.insert("parent_id".to_owned(), serde_json::json!(id));
    let bytes = serde_json::to_vec(&object).expect("serialize edited child as JSON");

    let response = upload_file(
        file_path.as_ref(),
        content_type.as_ref(),
        BOUNDARY.as_bytes(),
        &bytes,
    )
    .reply(&filter)
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = String::from_utf8_lossy(response.body()).into_owned();

    let id = serde_json::from_str::<CreationResponse>(&body)
        .expect("parse response as JSON")
        .id
        .unwrap();

    if unlisted {
        None
    } else {
        Some(id)
    }
}

async fn test_deletion(id_to_delete: &str, parent: &str, ids: &[String]) {
    let retrieve_filter = make_retrieve_filter("api_works").await;
    let request = warp::test::request()
        .path(&format!("/recs/id/{id}/", id = id_to_delete))
        .method("GET")
        .reply(&retrieve_filter)
        .await;
    assert_eq!(request.status(), StatusCode::OK);
    let recording: RetrievalResponse =
        serde_json::from_slice(request.body()).expect("deserialize retrieved recording");
    verify_recording_data(&recording, id_to_delete, parent);

    // can't hard-code a test for the URL since it can vary based on the environment
    let recording_url = &recording.url;

    {
        let response = reqwest::get(recording_url)
            .await
            .expect("verify recording exists in store before deleting");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let delete_filter = make_delete_filter("api_works").await;
    let request = warp::test::request()
        .path(&format!("/recs/id/{id}/", id = id_to_delete))
        .method("DELETE")
        .reply(&delete_filter)
        .await;
    assert_eq!(request.status(), StatusCode::NO_CONTENT);

    let request = warp::test::request()
        .path(&format!("/recs/id/{id}/", id = id_to_delete))
        .method("GET")
        .reply(&retrieve_filter)
        .await;
    assert_eq!(request.status(), StatusCode::GONE);

    let response = reqwest::get(recording_url)
        .await
        .expect("make request for deleted recording to store");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let request = warp::test::request()
        .path(&format!("/recs/id/{id}/children/", id = parent))
        .method("GET")
        .reply(&make_children_filter("test_deletion").await)
        .await;
    assert_eq!(request.status(), StatusCode::OK);
    let returned_ids = parse_children_ids(request.body());
    assert_eq!(
        ids.iter()
            .cloned()
            .filter(|i| i != id_to_delete)
            .collect::<Vec<_>>(),
        returned_ids
    );
}

async fn test_count() {
    let count_filter = make_count_filter("test_updating").await;
    let response = warp::test::request()
        .path(&format!("/recs/count"))
        .method("GET")
        .reply(&count_filter)
        .await;
    let count = String::from_utf8_lossy(response.body()).parse::<i64>().expect("parse count response as i64");

    assert_eq!(count, 4);
}

async fn test_updating(id: &str) {
    // can be simplified once async closures are stabilized (rust-lang/rust#62290)
    async fn retrieve(id: &str) -> RetrievalResponse {
        let retrieve_filter = make_retrieve_filter("test_updating").await;
        let response = warp::test::request()
            .path(&format!("/recs/id/{id}/", id = id))
            .method("GET")
            .reply(&retrieve_filter)
            .await;

        serde_json::from_slice(response.body()).expect("deserialize retrieved recording")
    }

    let before = retrieve(id).await.into_comparable();

    let response = warp::test::request()
        .path(&format!("/recs/id/{id}/hide/", id = id))
        .method("POST")
        .reply(&make_hide_filter("uploading works").await)
        .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let after = retrieve(id).await.into_comparable();

    assert_eq!(
        after,
        ComparableRetrievalResponse {
            unlisted: true,
            ..before
        }
    );
}

#[tokio::test]
async fn bad_uploads_fail() {
    use bytes::Bytes;

    fn assert_failed(
        response: warp::http::Response<Bytes>,
        expected_status: u16,
        verify_error_type: &dyn Fn(StatusCode) -> bool,
    ) {
        let status = response.status();
        assert!(verify_error_type(status));
        assert_eq!(status.as_u16(), expected_status);
    }

    let filter = make_upload_filter("bad_uploads_fail").await;

    {
        // should fail because of `content-type`
        let response = warp::test::request()
            .path("/recs/")
            .method("POST")
            .header("content-type", "text/plain")
            .header("content-length", 0)
            .reply(&filter)
            .await;

        assert_failed(response, 400, &|s: StatusCode| s.is_client_error());
    }
}

async fn test_non_existent_recording() {
    use uuid::Uuid;

    let retrieve_filter = make_retrieve_filter("api_works").await;
    let request = warp::test::request()
        .path(&format!("/recs/id/{id}/", id = Uuid::new_v4()))
        .method("GET")
        .reply(&retrieve_filter)
        .await;

    assert_eq!(request.status(), StatusCode::NOT_FOUND)
}

async fn make_upload_filter<'a>(
    test_name: impl Into<String>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, checker, urls) = make_environment(test_name.into()).await;

    routes::make_upload_route(
        logger_arc.clone(),
        db,
        Arc::new(make_store()),
        checker,
        urls,
    )
}

async fn make_delete_filter<'a>(
    test_name: impl Into<String>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, _, urls) = make_environment(test_name.into()).await;

    routes::make_delete_route(logger_arc.clone(), db, Arc::new(make_store()), urls)
}

async fn make_children_filter<'a>(
    test_name: impl Into<String>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, _, urls) = make_environment(test_name.into()).await;

    routes::make_children_route(logger_arc.clone(), db, urls)
}

async fn make_retrieve_filter<'a>(
    test_name: impl Into<String>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, _, urls) = make_environment(test_name.into()).await;

    routes::make_retrieve_route(logger_arc.clone(), db, urls)
}

async fn make_count_filter<'a>(test_name: impl Into<String>) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, _, urls) = make_environment(test_name.into()).await;

    routes::make_count_route(logger_arc.clone(), db, urls)
}

async fn make_hide_filter<'a>(
    test_name: impl Into<String>,
) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::reject::Rejection> + 'a {
    let (logger_arc, db, _, urls) = make_environment(test_name.into()).await;

    routes::make_hide_route(logger_arc.clone(), db, urls)
}

fn make_store() -> S3Store {
    S3Store::from_env().expect("initialize S3 store")
}

fn parse_children_ids(body: &[u8]) -> Vec<String> {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ChildrenResponse {
        parent: String,
        children: Vec<Child>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Child {
        id: String,
        name: String,
    };

    let response: ChildrenResponse = serde_json::from_slice(body).expect("parse children response");

    response
        .children
        .into_iter()
        .map(|Child { id, .. }| id)
        .collect::<Vec<_>>()
}

async fn make_environment(
    test_name: String,
) -> (
    Arc<Logger>,
    Arc<impl Db>,
    Arc<impl Fn(&[u8]) -> Result<(), errors::BackendError>>,
    Arc<Urls>,
) {
    read_config();
    initialize_global_logger();

    let logger = slog_scope::logger().new(o!("test" => test_name));
    let logger_arc = Arc::new(logger);

    let checker = make_wrapper_for_test(logger_arc.clone());
    let db = make_db().await;

    (
        logger_arc.clone(),
        Arc::new(db),
        Arc::new(checker),
        Arc::new(Urls::new("https://www.example.com/", "recs")),
    )
}

fn initialize_global_logger() {
    SLOG_SCOPE_GUARD.get_or_init(|| slog_envlogger::init().expect("initialize slog-envlogger"));
}

fn read_config() {
    static INITIALIZED_CONFIG: Once = Once::new();

    INITIALIZED_CONFIG.call_once(|| {
        dotenv::dotenv().ok();
    });
}

fn upload_file(
    path: impl AsRef<Path>,
    content_type: &str,
    boundary: &[u8],
    metadata: &[u8],
) -> warp::test::RequestBuilder {
    let data = fs::read(path.as_ref()).expect(&format!("read file {:?}", path.as_ref().display()));
    let body = make_multipart_body(boundary, metadata, &data);

    warp::test::request()
        .path("/recs/")
        .method("POST")
        .header("content-type", content_type)
        .header("content-length", body.len())
        .body(body)
}

fn make_wrapper_for_test(
    logger: Arc<Logger>,
) -> impl Fn(&[u8]) -> Result<(), errors::BackendError> {
    use backend::audio;

    audio::make_wrapper(
        logger.clone(),
        env::var("BACKEND_FFPROBE_PATH").ok(),
        env::var("BACKEND_MEDIA_CODEC")
            .expect("must define BACKEND_MEDIA_CODEC environment variable"),
        env::var("BACKEND_MEDIA_FORMAT")
            .expect("must define BACKEND_MEDIA_FORMAT environment variable"),
    )
}

fn verify_recording_data(recording: &RetrievalResponse, id: &str, parent_id: &str) {
    assert_eq!(recording.id, id);
    // the URL is tested for validity separately
    // serde has already verified that the times are i64s, i.e. valid as Unix timestamps
    assert_eq!(
        recording.category,
        RelatedLabel(1, "This is a category".to_owned())
    );
    assert_eq!(recording.unlisted, false);
    assert_eq!(recording.parent, Some(parent_id.to_owned()));
    assert_eq!(recording.name, "Another \r\nname");
    assert_eq!(recording.age, None);
    assert_eq!(
        recording.gender,
        Some(RelatedLabel(2, "Some other genders".to_owned()))
    );
    assert_eq!(recording.location, None);
    assert_eq!(recording.occupation, Some("something".to_owned()));
}

async fn make_db() -> impl Db {
    use sqlx::Pool;
    use tokio::task;

    use backend::db::PgDb;

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
    const NEWLINE: &[u8] = "\r\n".as_bytes();
    const METADATA_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"metadata\"\r\n\r\n".as_bytes();
    const AUDIO_HEADER: &[u8] =
        "Content-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/ogg\r\n\r\n"
            .as_bytes();

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
    const BOUNDARY_LEADER: &[u8] = &[b'-', b'-'];

    let parts = &[BOUNDARY_LEADER, boundary];
    parts.concat()
}

fn multipart_content_type(boundary: &str) -> String {
    format!("multipart/form-data; boundary={}", boundary)
}
