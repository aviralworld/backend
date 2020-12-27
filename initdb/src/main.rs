use std::env;

use movine::Movine;
use postgres::{Client, NoTls};

fn main() {
    let connection_string =
        env::var("BACKEND_DB_CONNECTION_STRING").expect("read BACKEND_DB_CONNECTION_STRING");
    let client = Client::connect(&connection_string, NoTls)
        .expect("create postgres::Client from BACKEND_DB_CONNECTION_STRING");
    let mut movine = Movine::new(client);
    movine.set_migration_dir("../migrations");

    if movine.status().is_err() {
        movine.initialize().expect("initialize movine");
    }

    movine.up().expect("run movine migrations");
}
