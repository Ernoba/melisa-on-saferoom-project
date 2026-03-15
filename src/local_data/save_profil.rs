use serde::{Serialize, Deserialize};
use std::fs;
use crate::cli::color_text::{GREEN, RED, BLUE, RESET};

#[derive(Serialize, Deserialize, Debug)]
pub struct Profil {
    pub username: String,
    pub password: String,
}

impl Profil {
    pub fn load_from_file() -> Option<Self> {
        let path = "data/profil.toml";

        if let Ok(contents) = fs::read_to_string(path) {
            toml::from_str(&contents).ok()
        } else {
            None
        }
    }

    pub fn save_to_file(&self) {
        let folder = "data";
        let file_path = format!("{}/profil.toml", folder);

        if let Err(e) = fs::create_dir_all(folder) {
            eprintln!("  {}error:{} Failed to initialize directory '{}': {}", RED, RESET, folder, e);
            return;
        }

        match toml::to_string(self) {
            Ok(contents) => {
                if let Err(e) = fs::write(&file_path, contents) {
                    eprintln!("  {}error:{} Could not write to {}: {}", RED, RESET, file_path, e);
                } else {
                    println!();
                    println!("  {}•{} Configuration stored in {}{}{}", BLUE, RESET, GREEN, file_path, RESET);
                }
            }
            Err(e) => {
                eprintln!("  {}error:{} Serialization failed: {}", RED, RESET, e);
            }
        }
    }
}