use std::env;
use rustyline::Editor;
use rustyline::history::FileHistory;
use tokio::fs;
use std::io::ErrorKind;

use crate::cli::helper::MelisaHelper;
use crate::cli::color_text::{GREEN, RED, YELLOW, BLUE, BOLD, RESET};

/// Dynamically constructs the interactive terminal prompt string.
/// Adapts to the execution context (Standard User vs. Sudo/Root).
pub struct Prompt {
    pub user: String,
    pub home: String,
}

impl Prompt {
    /// Initializes a new Prompt instance by resolving the executing user's environment.
    pub fn new() -> Self {
        // Resolve the actual user identity, prioritizing SUDO_USER to prevent 
        // the prompt from falsely claiming "root" when an admin escalates privileges.
        let user = env::var("SUDO_USER")
            .or_else(|_| env::var("USER"))
            .or_else(|_| env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        // Resolve the home directory to enable path abbreviation (e.g., replacing /home/user with ~)
        let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());

        Self { user, home }
    }

    /// Compiles and formats the final prompt string injected into the Rustyline editor.
    pub fn build(&self) -> String {
        // Retrieve the current working directory and abbreviate the home path to '~'
        let curr_path = env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(&self.home, "~");
        
        // Output Format: melisa@username:~/current/path> 
        format!("{BOLD}{GREEN}melisa@{}{RESET}:{BLUE}{}{RESET}> ", self.user, curr_path)
    }
}

/// Safely purges the command history both from the active memory buffer and physical storage.
pub async fn reset_history(rl: &mut Editor<MelisaHelper, FileHistory>, history_path: &str) {
    // 1. Purge the in-memory history buffer first (Atomic application-level operation)
    let _ = rl.clear_history();

    // 2. Attempt physical deletion without prior metadata checks.
    // This strictly prevents TOCTOU (Time-Of-Check to Time-Of-Use) race conditions.
    match fs::remove_file(history_path).await {
        Ok(_) => {
            println!("{}[SUCCESS]{} Local history file has been permanently deleted.", GREEN, RESET);
        }
        Err(e) => {
            match e.kind() {
                // Ignore if the file is already gone (e.g., deleted manually by the user)
                ErrorKind::NotFound => {
                    println!("{}[INFO]{} History file does not exist or was already removed.", YELLOW, RESET);
                }
                // Handle restricted access contexts gracefully
                ErrorKind::PermissionDenied => {
                    eprintln!("{}[ERROR]{} Cannot delete history: Permission denied. Escalation required.", RED, RESET);
                }
                // Catch-all for unexpected I/O anomalies (e.g., locked file, corrupted filesystem)
                _ => {
                    eprintln!("{}[ERROR]{} Unexpected I/O error during history reset: {}", RED, RESET, e);
                }
            }
        }
    }

    // 3. Re-initialize a clean, empty history file to synchronize the Editor state.
    // This prevents Rustyline from crashing upon exit when it attempts to save a new session.
    if let Err(e) = rl.save_history(history_path) {
        eprintln!("{}[WARNING]{} Failed to re-initialize an empty history file: {}", YELLOW, RESET, e);
    } else {
        // --- ENTERPRISE SECURITY PATCH ---
        // Command history files can contain sensitive parameters.
        // We must enforce strict 0600 permissions (Owner Read/Write ONLY) on the newly created file.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Note: We use std::fs here because modifying permissions immediately after 
            // a synchronous rustyline save_history call guarantees file existence.
            if let Ok(mut perms) = std::fs::metadata(history_path).map(|m| m.permissions()) {
                perms.set_mode(0o600);
                let _ = std::fs::set_permissions(history_path, perms);
            }
        }
    }

    println!("{}[DONE]{} Command history purge sequence completed successfully.", GREEN, RESET);
}