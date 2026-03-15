use std::{env, fs};
use crate::cli::color_text::{GREEN, BLUE, BOLD, RESET};

pub struct Prompt {
    pub user: String,
    pub host: String,
    pub home: String,
}

impl Prompt {
    pub fn new() -> Self {
        let user = env::var("SUDO_USER").unwrap_or_else(|_| env::var("USER").unwrap_or_default());
        let home = if user == "root" { "/root".to_string() } else { format!("/home/{}", user) };
        let host = fs::read_to_string("/proc/sys/kernel/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "fedora".into());

        Self { user, host, home }
    }

    pub fn build(&self) -> String {
        let curr_path = env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(&self.home, "~");
        
        format!("{BOLD}{GREEN}melisa@{}{RESET}:{BLUE}{}{RESET}> ", self.host, curr_path)
    }
}