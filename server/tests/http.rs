use std::env;
use std::fs;
use std::path::Path;

use lazy_static::lazy_static;
use serde::Deserialize;
use url::Url;
use warp::http::StatusCode;

use backend::config::get_variable;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreationResponse {
    message: Option<String>,
    id: Option<String>,
    tokens: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetrievalResponse {
    id: String,
    url: String,
    mime_type: RelatedLabel,
    created_at: i64,
    updated_at: i64,
    category: RelatedLabel,
    parent: Option<String>,
    name: String,
    age: Option<RelatedLabel>,
    gender: Option<RelatedLabel>,
    location: Option<String>,
    occupation: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RandomResponse {
    recordings: Vec<RandomRecording>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct TokenResponse {
    id: String,
    parent_id: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RandomRecording {
    id: String,
    name: String,
    location: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RelatedLabel(i16, String, Option<String>);

const BOUNDARY: &str = "thisisaboundary1234";

lazy_static! {
    static ref TOKENS_PER_RECORDING: u8 = get_variable("BACKEND_TOKENS_PER_RECORDING").parse().expect("parse BACKEND_TOKENS_PER_RECORDING");
}

#[tokio::test]
async fn api_works() {
    dotenv::dotenv().ok();

    prepare_db().await;

    test_formats();
    test_ages();
    test_categories();
    test_genders();

    test_non_existent_recording().await;

    let content_type = multipart_content_type(&BOUNDARY);

    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let base_path = Path::new(&cargo_dir);
    let file_path = base_path.join("tests").join("opus_file.ogg");

    let (id, tokens) = test_upload(&file_path, &content_type);
    test_duplicate_upload(&file_path, &content_type);

    let children: serde_json::Value = serde_json::from_reader(
        fs::File::open("tests/simple_metadata_children.json")
            .expect("open simple_metadata_children.json"),
    )
    .expect("parse simple_metadata_children.json");

    let results = test_uploading_children(&file_path, &content_type, &id, tokens, children);

    let id_to_delete = results[2].0.to_owned();
    test_deletion(
        &id_to_delete,
        &id,
        &results
            .iter()
            .map(|(id, _)| id.to_owned())
            .collect::<Vec<_>>(),
    );

    test_count();

    test_random();

    let (id, tokens) = results[0].to_owned();
    test_token(tokens[0].to_owned(), id);
}

fn test_formats() {
    let response =
        reqwest::blocking::get(url_to(Some("formats".to_string()))).expect("get /formats");

    let formats =
        serde_json::from_str::<Vec<String>>(&response.text().expect("get response body as string"))
            .expect("parse response as Vec<String>");

    assert_eq!(formats, vec!["audio/ogg; codec=opus", "audio/ogg"]);
}

fn test_ages() {
    lazy_static! {
        static ref AGES: Vec<RelatedLabel> = {
            vec![
                RelatedLabel(1, String::from("Age 1"), None),
                RelatedLabel(2, String::from("Age B"), None),
                RelatedLabel(3, String::from("Age three"), None),
                RelatedLabel(4, String::from("Fooled ya! This is Age 2"), None),
            ]
        };
    }

    let response = reqwest::blocking::get(url_to(Some("ages".to_string()))).expect("get /ages");

    assert_eq!(response.status(), 200);

    let ages = serde_json::from_str::<Vec<RelatedLabel>>(
        &response.text().expect("get response body as string"),
    )
    .expect("parse response as Vec<RelatedLabel>");

    assert_eq!(ages, *AGES);
}

fn test_categories() {
    lazy_static! {
        static ref CATEGORIES: Vec<RelatedLabel> = {
            vec![
                RelatedLabel(6, String::from("This is a category"), None),
                RelatedLabel(2, String::from("Some other category"), None),
                RelatedLabel(
                    7,
                    "This category has
  some newlines
and spaces in it"
                        .to_owned(),
                    None,
                ),
                RelatedLabel(
                    3,
                    String::from("यह हिन्दी है ।"),
                    Some(String::from("This is a description")),
                ),
                RelatedLabel(4, String::from("Ceci n’est pas une catégorie"), None),
                RelatedLabel(1, String::from("یہ بھی ہے"), None),
            ]
        };
    }

    let response =
        reqwest::blocking::get(url_to(Some("categories".to_string()))).expect("get /categories");

    assert_eq!(response.status(), 200);

    let categories =
        serde_json::from_str::<Vec<RelatedLabel>>(&response.text().expect("get response body"))
            .expect("parse response as Vec<RelatedLabel>");

    assert_eq!(categories, *CATEGORIES);
}

fn test_genders() {
    lazy_static! {
        static ref GENDERS: Vec<RelatedLabel> = {
            vec![
                RelatedLabel(1, String::from("One of the genders"), None),
                RelatedLabel(2, String::from("Some other genders"), None),
                RelatedLabel(3, String::from("No gender specified"), None),
                RelatedLabel(50, String::from("None of the above"), None),
            ]
        };
    }

    let response =
        reqwest::blocking::get(url_to(Some("genders".to_string()))).expect("get /genders");

    assert_eq!(response.status(), 200);

    let genders =
        serde_json::from_str::<Vec<RelatedLabel>>(&response.text().expect("get response body"))
            .expect("parse response as Vec<RelatedLabel>");

    assert_eq!(genders, *GENDERS);
}

fn test_upload(
    file_path: impl AsRef<Path>,
    content_type: impl AsRef<str>,
) -> (String, Vec<String>) {
    let bytes = fs::read("tests/simple_metadata.json").expect("read simple_metadata.json");

    let response = upload_file(
        file_path.as_ref(),
        content_type.as_ref(),
        BOUNDARY.as_bytes(),
        &bytes,
    );

    assert_eq!(response.status(), StatusCode::CREATED);

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
    assert_eq!(segments[0], get_variable("BACKEND_RECORDINGS_PATH"));
    assert_eq!(segments.len(), 2);

    let response = serde_json::from_str::<CreationResponse>(
        &response.text().expect("get response body as string"),
    )
    .expect("parse response as JSON");

    let id = response.id.expect("get ID from response");

    assert_ne!(id, "", "response must provide non-blank key");

    let tokens = response.tokens.unwrap();

    assert_eq!(
        tokens.len(),
        *TOKENS_PER_RECORDING as usize,
        "response must provide the correct number of tokens"
    );

    (id, tokens)
}

fn test_duplicate_upload(file_path: impl AsRef<Path>, content_type: impl AsRef<str>) {
    // ensure the token cannot be reused
    {
        let bytes = fs::read("tests/simple_metadata_with_same_token.json")
            .expect("read simple_metadata_with_same_token.json");

        let response = upload_file(
            &file_path,
            content_type.as_ref(),
            BOUNDARY.as_bytes(),
            &bytes,
        );

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let deserialized: CreationResponse =
            serde_json::from_str(&response.text().expect("get response body as string"))
                .expect("parse response as JSON");
        assert!(
            deserialized.id.is_none(),
            "error response must not include key"
        );
        assert!(
            deserialized.message.unwrap().starts_with("invalid token"),
            "error response must mention invalid token"
        );
    }

    // ensure the name cannot be reused
    {
        let bytes =
            fs::read("tests/duplicate_metadata.json").expect("read duplicate_metadata.json");

        let response = upload_file(
            &file_path,
            content_type.as_ref(),
            BOUNDARY.as_bytes(),
            &bytes,
        );

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let deserialized: CreationResponse =
            serde_json::from_str(&response.text().expect("get response body as string"))
                .expect("parse response as JSON");
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
}

fn test_uploading_children(
    file_path: impl AsRef<Path>,
    content_type: impl AsRef<str>,
    parent: &str,
    mut tokens: Vec<String>,
    mut children: serde_json::Value,
) -> Vec<(String, Vec<String>)> {
    let mut results = vec![];

    for mut child in children
        .as_array_mut()
        .expect("get array from simple_metadata_children.json")
    {
        let result = test_uploading_child(
            file_path.as_ref(),
            content_type.as_ref(),
            tokens.pop().unwrap(),
            &mut child,
        );

        if let Some((id, tokens)) = result {
            results.push((id, tokens));
        }
    }

    {
        let path = format!("id/{id}/children/", id = parent);
        let response = reqwest::blocking::get(url_to(Some(path.to_string())))
            .expect(&format!("get {path}", path = path));

        assert_eq!(response.status(), StatusCode::OK);

        let returned_ids =
            parse_children_ids(&response.bytes().expect("get response body as bytes"));
        assert_eq!(
            results.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
            returned_ids
        );
    }

    results
}

fn test_uploading_child(
    file_path: impl AsRef<Path>,
    content_type: impl AsRef<str>,
    token: String,
    child: &mut serde_json::Value,
) -> Option<(String, Vec<String>)> {
    let object = child.as_object_mut().expect("get child as object");
    object.insert("token".to_owned(), serde_json::json!(token));
    let bytes = serde_json::to_vec(&object).expect("serialize edited child as JSON");

    let response = upload_file(
        file_path.as_ref(),
        content_type.as_ref(),
        BOUNDARY.as_bytes(),
        &bytes,
    );

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = serde_json::from_str::<CreationResponse>(
        &response.text().expect("get response body as string"),
    )
    .expect("parse response as JSON");

    let id = response.id.unwrap();
    let tokens = response.tokens.unwrap();

    Some((id, tokens))
}

fn test_deletion(id_to_delete: &str, parent: &str, ids: &[String]) {
    let path = format!("id/{id}/", id = id_to_delete);
    let response = reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get /{}", path));

    assert_eq!(response.status(), StatusCode::OK);

    let recording: RetrievalResponse =
        serde_json::from_slice(&response.bytes().expect("get response body as string"))
            .expect("deserialize retrieved recording");
    verify_recording_data(&recording, id_to_delete, parent);

    // can't hard-code a test for the URL since it can vary based on the environment
    let recording_url = &recording.url;

    {
        let response = reqwest::blocking::get(recording_url)
            .expect("verify recording exists in store before deleting");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .expect("get content-type header")
                .to_str()
                .expect("convert content-type header to string"),
            "audio/ogg; codec=opus"
        );
    }

    let client = reqwest::blocking::Client::new();
    let path = format!("id/{id}/", id = id_to_delete);
    let response = client
        .request(reqwest::Method::DELETE, url_to(Some(path.clone())))
        .send()
        .expect(&format!("get {}", path));

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let path = format!("id/{id}/", id = id_to_delete);
    let response =
        reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get /{}", path));
    assert_eq!(response.status(), StatusCode::GONE);

    let response =
        reqwest::blocking::get(recording_url).expect("make request for deleted recording to store");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let path = format!("id/{id}/children", id = parent);
    let response =
        reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get /{}", path));

    assert_eq!(response.status(), StatusCode::OK);
    let returned_ids = parse_children_ids(&response.bytes().expect("get response body as string"));
    assert_eq!(
        ids.iter()
            .cloned()
            .filter(|i| i != id_to_delete)
            .collect::<Vec<_>>(),
        returned_ids
    );
}

fn test_count() {
    let response = reqwest::blocking::get(url_to(Some("count".to_string()))).expect("get /count");
    let count = response
        .text()
        .expect("get response body as string")
        .parse::<i64>()
        .expect("parse count response as i64");

    assert_eq!(count, 5);
}

fn test_random() {
    use std::collections::HashSet;

    let response =
        reqwest::blocking::get(url_to(Some("random/10".to_string()))).expect("get /random/10");

    assert_eq!(response.status(), 200);

    let parsed: RandomResponse =
        serde_json::from_slice(&response.bytes().expect("get response body as bytes"))
            .expect("deserialize retrieved recording");
    let recordings = parsed
        .recordings
        .into_iter()
        .map(|r| r.id)
        .collect::<HashSet<_>>();

    assert_eq!(recordings.len(), 5);
}

fn test_token(token_id: String, parent_id: String) {
    use uuid::Uuid;

    {
        let uuid = Uuid::new_v4();
        let path = format!("token/{}/", uuid);
        let response =
            reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get {}", path));
        assert_eq!(response.status(), 404);
    }

    {
        let path = format!("token/{}/", token_id);
        let response =
            reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get {}", path));
        assert_eq!(response.status(), 200);

        let parsed: TokenResponse =
            serde_json::from_slice(&response.bytes().expect("get response body as bytes"))
                .expect("deserialize token response");

        assert_eq!(parsed.parent_id, parent_id);
    }
}

#[tokio::test]
async fn bad_uploads_fail() {
    {
        let response = reqwest::blocking::Client::new()
            .request(reqwest::Method::POST, url_to(None))
            .header("content-type", "text/plain")
            .header("content-length", 0)
            .send()
            .expect("make request");

        // should fail because of `content-type`
        let status = response.status();
        assert!(status.is_client_error());
        assert_eq!(status.as_u16(), 400);
    }
}

async fn test_non_existent_recording() {
    use uuid::Uuid;

    let path = format!("id/{id}", id = Uuid::new_v4());
    let response =
        reqwest::blocking::get(url_to(Some(path.clone()))).expect(&format!("get {}", path));

    assert_eq!(response.status(), StatusCode::NOT_FOUND)
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

fn upload_file(
    path: impl AsRef<Path>,
    content_type: &str,
    boundary: &[u8],
    metadata: &[u8],
) -> reqwest::blocking::Response {
    let data = fs::read(path.as_ref())
        .unwrap_or_else(|_| panic!("read file {:?}", path.as_ref().display()));
    let body = make_multipart_body(boundary, metadata, &data);

    reqwest::blocking::Client::new()
        .request(reqwest::Method::POST, url_to(None))
        .header("content-type", content_type)
        .header("content-length", body.len())
        .body(body)
        .send()
        .expect(&format!("upload {:?}", path.as_ref().display()))
}

fn verify_recording_data(recording: &RetrievalResponse, id: &str, parent_id: &str) {
    assert_eq!(recording.id, id);
    // the URL is tested for validity separately

    assert_eq!(recording.mime_type.1, "audio/ogg; codec=opus");

    // serde has already verified that the times are i64s, i.e. valid as Unix timestamps
    assert_eq!(
        recording.category,
        RelatedLabel(1, "یہ بھی ہے".to_owned(), None)
    );
    assert_eq!(recording.parent, Some(parent_id.to_owned()));
    assert_eq!(recording.name, "Another \r\nname");
    assert_eq!(recording.age, None);
    assert_eq!(
        recording.gender,
        Some(RelatedLabel(2, "Some other genders".to_owned(), None))
    );
    assert_eq!(recording.location, None);
    assert_eq!(recording.occupation, Some("something".to_owned()));
}

fn url_to(path: Option<String>) -> Url {
    lazy_static! {
        static ref BASE_URL: Url = {
            let raw = get_variable("BACKEND_SERVER_URL");
            Url::parse(&raw).expect("parse BACKEND_SERVER_URL as URL")
        };

        static ref BASE_PATH: String = format!("{}/", get_variable("BACKEND_RECORDINGS_PATH"));
    }

    let base = BASE_URL
        .join(&BASE_PATH)
        .expect("join BASE_URL with BASE_PATH");

    match path {
        Some(p) => base
            .join(&p)
            .expect(&format!("must join {} to {}", BASE_URL.as_str(), p)),
        _ => base,
    }
}

async fn prepare_db() {
    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");

    if env::var("BACKEND_TEST_INITIALIZE_DB").unwrap_or_else(|_| "0".to_owned()) == "1" {
        tokio::task::spawn_blocking(move || initialize_db_for_test(&connection_string)).await.expect("initialize DB");
    }
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

    movine.set_migration_dir("../migrations");
    movine.set_strict(true);

    if movine.status().is_err() {
        movine.initialize().expect("initialize movine");
    }

    movine.up().expect("run movine migrations");

    let sql = fs::read_to_string("tests/data.sql").expect("read SQL file");
    client.simple_query(&sql).expect("execute SQL file");
}

fn make_multipart_body(boundary: &[u8], metadata: &[u8], content: &[u8]) -> Vec<u8> {
    const NEWLINE: &[u8] = b"\r\n";
    const METADATA_HEADER: &[u8] = b"Content-Disposition: form-data; name=\"metadata\"\r\n\r\n";
    const AUDIO_HEADER: &[u8] =
        b"Content-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/ogg\r\n\r\n";

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
    parts.push(b"--");
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
