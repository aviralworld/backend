use std::env;

/// Returns the value of the named environment variable if it exists or panics.
pub fn get_variable(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("must define {} environment variable", name))
}
