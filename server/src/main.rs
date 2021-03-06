use std::env;
use std::error::Error;
use std::fs;
use std::sync::Arc;

use futures::future::{BoxFuture, FutureExt};
use tokio::sync::mpsc;
use warp::Filter;

use backend::audio;
use backend::config::{get_ffprobe, get_variable};
use backend::db::PgDb;
use backend::environment::{Config, Environment};
use backend::routes;
use backend::store::S3Store;
use backend::urls::Urls;
use log::{info, initialize_logger, Logger};

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

    let terminate = Arc::new(move || {
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
                    terminate().await;
                }
            }
        }
    };

    let main_server = start_main_server(
        logger.clone(),
        main_port,
        environment.clone(),
        should_terminate.clone(),
    );

    let admin_server = start_admin_server(
        logger.clone(),
        admin_port,
        environment.clone(),
        should_terminate.clone(),
        terminate.clone(),
    );

    tokio::join!(ctrlc, main_server, admin_server);

    info!(logger, "Exiting gracefully...");

    Ok(())
}

fn start_main_server<O: Clone + Send + Sync + 'static>(
    logger: Arc<Logger>,
    port: u16,
    environment: Environment<O>,
    should_terminate: futures::future::Shared<
        impl warp::Future<Output = ()> + Send + Sync + 'static,
    >,
) -> impl warp::Future<Output = ()> + 'static {
    use routes as r;

    let logger2 = logger.clone();

    let prefix = warp::path(environment.urls.recordings_path.clone());

    let mut routes = vec![
        r::make_formats_route(environment.clone()),
        r::make_ages_list_route(environment.clone()),
        r::make_categories_list_route(environment.clone()),
        r::make_genders_list_route(environment.clone()),
        r::make_count_route(environment.clone()),
        r::make_upload_route(environment.clone()),
        r::make_children_route(environment.clone()),
        r::make_delete_route(environment.clone()),
        r::make_retrieve_route(environment.clone()),
        r::make_random_route(environment.clone()),
        r::make_token_route(environment.clone()),
        r::make_lookup_route(environment.clone()),
        r::make_availability_route(environment),
    ];

    let first = routes.pop().expect("get first route");

    let routes = routes
        .into_iter()
        .fold(first, |e, r| e.or(r).unify().boxed())
        .recover(move |r| routes::format_rejection(logger2.clone(), r));

    let (_, main_server) =
        warp::serve(prefix.and(routes)).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            should_terminate.await;
        });

    main_server
}

fn start_admin_server<O: Clone + Send + Sync + 'static>(
    _logger: Arc<Logger>,
    port: u16,
    environment: Environment<O>,
    should_terminate: futures::future::Shared<
        impl warp::Future<Output = ()> + Send + Sync + 'static,
    >,
    terminate: Arc<dyn Fn() -> BoxFuture<'static, ()> + Send + Sync + 'static>,
) -> impl warp::Future<Output = ()> + 'static {
    let terminate = terminate.clone();

    let routes = routes::admin::make_healthz_route(environment.clone()).or(
        routes::admin::make_termination_route(environment, terminate),
    );

    let (_, admin_server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            should_terminate.await;
        });

    admin_server
}
