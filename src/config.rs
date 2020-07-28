use std::env;

/// Returns the value of the named environment variable if it exists or panics.
pub fn get_variable(name: &str) -> String {
    env::var(name).expect(&format!("must define {} environment variable", name))
}
