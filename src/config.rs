use std::sync::{Arc, OnceLock};

pub struct Config {
    pub username: String,
    pub password: String,
}

pub static GLOBAL_CONFIG: OnceLock<Arc<Config>> = OnceLock::new();

impl Config {
    pub fn global() -> Arc<Self> {
        GLOBAL_CONFIG.get()
            .cloned()
            .expect("Global config not initialized")
    }
}