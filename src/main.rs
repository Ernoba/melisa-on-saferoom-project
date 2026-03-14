mod config;
mod client;
mod local_data;

use std::io::{self, Write};
use std::sync::Arc;
use crate::config::{Config, GLOBAL_CONFIG};

fn main() {
    let mut input_username = String::new();
    let mut input_password = String::new();

    print!("Enter username: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input_username).unwrap();
    print!("Enter password: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input_password).unwrap();

    let initial_input = Config {
        username: input_username.trim().to_string(),
        password: input_password.trim().to_string(),
    };

    GLOBAL_CONFIG.set(Arc::new(initial_input)).ok().expect("Failed to set global config");
}
