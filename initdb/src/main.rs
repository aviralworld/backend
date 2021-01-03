use std::env;
use std::thread::sleep;
use std::time::Duration;

use movine::Movine;
use postgres::{Client, NoTls};

fn main() {
    let connection_string =
        env::var("BACKEND_DB_CONNECTION_STRING").expect("read BACKEND_DB_CONNECTION_STRING");

    let keep_trying = env::var("BACKEND_INITDB_KEEP_TRYING").unwrap_or(String::new()) == "1";
    let ms_between_attempts: u32 = env::var("BACKEND_INITDB_DELAY")
        .expect("retrieve BACKEND_INITDB_DELAY")
        .parse()
        .expect("parse BACKEND_INITDB_DELAY as u32");
    let delay = Duration::from_millis(ms_between_attempts as u64);

    let mut attempt = 1;

    loop {
        let result = Client::connect(&connection_string, NoTls);

        if let Ok(client) = result {
            let mut movine = Movine::new(client);
            movine.set_migration_dir("../migrations");

            if movine.status().is_err() {
                movine.initialize().expect("initialize movine");
            }

            movine.up().expect("run movine migrations");
            break;
        }

        if keep_trying {
            attempt += 1;
            println!(
                "Sleeping {} milliseconds before attempt #{}...",
                ms_between_attempts, attempt
            );
            sleep(delay);
        } else {
            break;
        }
    }
}
