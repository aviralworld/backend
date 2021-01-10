//! A helper program to initialize the database for testing.

use std::env;

use movine::Movine;
use postgres::{Client, NoTls};

use log::{debug, initialize_logger};

fn main() {
    dotenv::dotenv().ok();

    let logger = initialize_logger();
    let connection_string = env::var("BACKEND_DB_CONNECTION_STRING")
        .expect("could not read BACKEND_DB_CONNECTION_STRING");

    debug!(logger, "Connecting to database...");

    let client = Client::connect(&connection_string, NoTls).expect("could not connect to database");

    let mut movine = Movine::new(client);
    movine.set_migration_dir("./migrations");

    if movine.status().is_err() {
        debug!(logger, "Initializing movine...");
        movine.initialize().expect("failed to initialize movine")
    }

    debug!(logger, "Running migrations...");
    movine.up().expect("failed to run migrations");

    debug!(logger, "Completed initialization.");
}
