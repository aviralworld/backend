use std::env;
use std::error::Error;

use movine::Movine;
use postgres::{Client, NoTls};
use warp::http::StatusCode;
use warp::reply::{json, with_status, Json, WithStatus};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let port: u16 = env::var("INITDB_PORT")
        .expect("read INITDB_PORT")
        .parse()
        .expect("parse INITDB_PORT");
    let route = warp::post().and_then(initialize_db);

    warp::serve(route).run(([0, 0, 0, 0], port)).await;

    Ok(())
}

async fn initialize_db() -> Result<WithStatus<Json>, warp::reject::Rejection> {
    use warp::reject::custom;

    let connection_string =
        env::var("BACKEND_DB_CONNECTION_STRING").expect("read BACKEND_DB_CONNECTION_STRING");

    let result = Client::connect(&connection_string, NoTls);

    match result {
        Ok(client) => {
            let mut movine = Movine::new(client);
            movine.set_migration_dir("../migrations");

            if movine.status().is_err() {
                movine
                    .initialize()
                    .map_err(|_| custom(Failure("failed to initialize movine".to_string())))?;
            }

            movine
                .up()
                .map_err(|_| custom(Failure("failed to run migrations".to_string())))?;

            Ok(with_status(json(&()), StatusCode::NO_CONTENT))
        }
        Err(e) => Err(custom(Failure(e.to_string()))),
    }
}

#[derive(Debug)]
struct Failure(String);

impl warp::reject::Reject for Failure {}
