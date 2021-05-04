use std::error::Error;

use dotenv::dotenv;
use log::{debug, info, initialize_logger};
use structopt::StructOpt;
use uuid::Uuid;

use backend::config::get_variable;
use backend::db::PgDb;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "generate-tokens",
    about = "Generate and print tokens for the given recordings"
)]
struct Opt {
    /// The recording IDs to generate tokens for
    #[structopt(parse(try_from_str = Uuid::parse_str))]
    ids: Vec<Uuid>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let opt = Opt::from_args();

    let logger = initialize_logger();

    let connection_string = get_variable("BACKEND_DB_CONNECTION_STRING");
    let pool = sqlx::Pool::connect(&connection_string)
        .await
        .expect("create database pool from BACKEND_DB_CONNECTION_STRING");
    let db = PgDb::new(pool);

    let ids = opt.ids;
    let tokens_per_recording: u8 = get_variable("BACKEND_TOKENS_PER_RECORDING")
        .parse()
        .expect("parse BACKEND_TOKENS_PER_RECORDING as u8");

    info!(
        logger,
        "Generating {} tokens each for {:?}...", tokens_per_recording, &ids
    );

    let mut tokens = vec![];

    for id in &ids {
        let logger = logger.new(log::o!("id" => format!("{}", id)));
        info!(logger, "Generating tokens for recording {}...", id);

        for number in 1..=tokens_per_recording {
            use backend::db::Db;

            info!(logger, "Generating token #{}...", number);

            let token = db.create_token(id).await.expect("create token");
            debug!(logger, "Generated token #{}: {}", number, token);
            tokens.push(token);
        }
    }

    info!(
        logger,
        "Generated tokens: {}",
        tokens
            .into_iter()
            .map(|t| format!("{}", t))
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(())
}
