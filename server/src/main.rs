use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use warp::Filter;

use backend::audio;
use backend::config::{get_ffprobe, get_variable};
use backend::db::PgDb;
use backend::environment::{Config, Environment};
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;
use futures::future::FutureExt;
use log::{info, initialize_logger};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let logger = initialize_logger();

    let store = Arc::new(S3Store::from_env().expect("initialize S3 store from environment"));

    fs::create_dir_all(env::temp_dir()).expect("ensure temporary directory exists");

    let main_port: u16 = get_variable("BACKEND_PORT")
        .parse()
        .expect("parse BACKEND_PORT as u16");
    let admin_port: u16 = get_variable("BACKEND_ADMIN_PORT")
        .parse()
        .expect("parse BACKEND_ADMIN_PORT as u16");

    info!(logger, "Starting..."; "main_port" => main_port, "admin_port" => admin_port);
    let logger = Arc::new(logger);

    let ffprobe_path = get_ffprobe(env::var("BACKEND_FFPROBE_PATH").ok());
    let checker = Arc::new(audio::make_wrapper(logger.clone(), ffprobe_path));

    info!(logger, "Creating database pool...");
    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
    let pool = sqlx::Pool::connect(&connection_string)
        .await
        .expect("create database pool from BACKEND_DB_CONNECTION_STRING");
    let db = Arc::new(PgDb::new(pool));

    let urls = Arc::new(Urls::new(
        get_variable("BACKEND_BASE_URL"),
        get_variable("BACKEND_RECORDINGS_PATH"),
    ));

    let config = Config::new(
        get_variable("BACKEND_TOKENS_PER_RECORDING")
            .parse()
            .expect("parse BACKEND_TOKENS_PER_RECORDING as u8"),
    );
    let environment = Environment::new(logger.clone(), db, urls, store, checker, config);

    let (termination_sender, mut termination_receiver) = mpsc::channel::<()>(1);

    let terminate =
        Arc::new(move || {
            let termination_sender = termination_sender.clone();

            async move {
            let termination_sender = termination_sender.clone();
                termination_sender.send(()).await.unwrap();
            }
            .boxed()
        });

    let should_terminate = async move {
        termination_receiver.recv().await;
    }
    .shared();

    let ctrlc = {
        let should_terminate = should_terminate.clone();
        let terminate = terminate.clone();

        let signal = tokio::signal::ctrl_c();

        async move {
            let terminate = terminate.clone();

            tokio::select! {
                _ = should_terminate => {},
                _ = signal => {
                    terminate();
                }
            }
        }
    };

    let main_server = {
        let should_terminate = should_terminate.clone();

        let logger2 = logger.clone();

        let formats_route = routes::make_formats_route(environment.clone());
        let ages_list_route = routes::make_ages_list_route(environment.clone());
        let categories_list_route = routes::make_categories_list_route(environment.clone());
        let genders_list_route = routes::make_genders_list_route(environment.clone());
        let count_route = routes::make_count_route(environment.clone());
        let upload_route = routes::make_upload_route(environment.clone());
        let children_route = routes::make_children_route(environment.clone());
        let delete_route = routes::make_delete_route(environment.clone());
        let retrieve_route = routes::make_retrieve_route(environment.clone());
        let lookup_key_route = routes::make_lookup_key_route(environment.clone());
        let random_route = routes::make_random_route(environment.clone());
        let token_route = routes::make_token_route(environment.clone());

        let routes = formats_route
            .or(ages_list_route)
            .or(categories_list_route)
            .or(genders_list_route)
            .or(count_route)
            .or(upload_route)
            .or(children_route)
            .or(delete_route)
            .or(random_route)
            .or(retrieve_route)
            .or(lookup_key_route)
            .or(token_route)
            .recover(move |r| routes::format_rejection(logger2.clone(), r));

        let (_, main_server) =
            warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], main_port), async {
                should_terminate.await;
            });

        main_server
    };

    let admin_server = {
        let should_terminate = should_terminate.clone();
        let terminate = terminate.clone();

        let routes = routes::admin::make_healthz_route(environment.clone()).or(
            routes::admin::make_termination_route(environment.clone(), terminate),
        );

        let (_, admin_server) =
            warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], admin_port), async {
                should_terminate.await;
            });

        admin_server
    };

    tokio::join!(ctrlc, main_server, admin_server);

    info!(logger, "Exiting gracefully...");

    Ok(())
}
