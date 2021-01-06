//! A helper program to initialize the database for testing.

use std::convert::Infallible;
use std::env;
use std::error::Error;

use movine::Movine;
use postgres::{Client, NoTls};
use serde::Serialize;
use tokio::task;
use warp::http::StatusCode;
use warp::reply::{json, with_status, Json, WithStatus};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let main_port: u16 = env::var("INITDB_PORT")
        .expect("read INITDB_PORT")
        .parse()
        .expect("parse INITDB_PORT");
    let admin_port: u16 = env::var("INITDB_ADMIN_PORT")
        .expect("read INITDB_ADMIN_PORT")
        .parse()
        .expect("parse INITDB_ADMIN_PORT");

    let initialize_db_route = warp::post().and_then(initialize_db);
    let main_server = warp::serve(initialize_db_route).run(([0, 0, 0, 0], main_port));

    let health_check_route = warp::get()
        .and(warp::path("healthz"))
        .and_then(health_check);
    let admin_server = warp::serve(health_check_route).run(([0, 0, 0, 0], admin_port));

    futures::future::join(main_server, admin_server).await;

    Ok(())
}

async fn initialize_db() -> Result<WithStatus<Json>, warp::reject::Rejection> {
    use warp::reject::custom;

    task::block_in_place(|| {
        let connection_string = env::var("BACKEND_DB_CONNECTION_STRING").map_err(|_| {
            custom(Failure(
                "could not read BACKEND_DB_CONNECTION_STRING".to_string(),
            ))
        })?;

        let result = Client::connect(&connection_string, NoTls);

        match result {
            Ok(client) => {
                let mut movine = Movine::new(client);
                movine.set_migration_dir("./migrations");

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
    })
    .map_err(|_| custom(Failure("failed to join blocking task".to_string())))
}

async fn health_check() -> Result<Json, Infallible> {
    Ok(json(
        &(HealthCheck {
            revision: info::REVISION,
            timestamp: info::BUILD_TIMESTAMP,
            version: info::VERSION,
        }),
    ))
}

#[derive(Serialize)]
struct HealthCheck<'a> {
    revision: Option<&'a str>,
    timestamp: Option<&'a str>,
    version: &'a str,
}

#[derive(Debug)]
struct Failure(String);

impl warp::reject::Reject for Failure {}
