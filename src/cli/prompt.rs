use std::env;
use rustyline::Editor;
use rustyline::history::FileHistory;
use tokio::fs;
use std::io::ErrorKind;
use crate::cli::helper::MelisaHelper;
use crate::cli::color_text::{GREEN, RED, YELLOW, BLUE, BOLD, RESET};

pub struct Prompt {
    pub user: String,
    pub home: String,
}

impl Prompt {
    pub fn new() -> Self {
        // Ambil nama user dari environment SSH/System
        let user = env::var("SUDO_USER")
            .or_else(|_| env::var("USER"))
            .or_else(|_| env::var("LOGNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        // Internal Melisa tetap mengacu ke /root
        let home = "/root".to_string(); 

        Self { user, home }
    }

    pub fn build(&self) -> String {
        let curr_path = env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(&self.home, "~");
        
        // Output: melisa@afira:~> atau melisa@saferoom:~>
        format!("{BOLD}{GREEN}melisa@{}{RESET}:{BLUE}{}{RESET}> ", self.user, curr_path)
    }
}

pub async fn reset_history(rl: &mut Editor<MelisaHelper, FileHistory>, history_path: &str) {
    // 1. Bersihkan history di RAM dulu (Operasi atomik di level aplikasi)
    let _ = rl.clear_history();

    // 2. Coba hapus file tanpa nge-check metadata dulu (Menghindari TOCTOU race condition)
    // Langsung tembak hapus, kalau error baru kita tangani.
    match fs::remove_file(history_path).await {
        Ok(_) => {
            println!("{GREEN}[SUCCESS]{RESET} History file has been physically deleted.");
        }
        Err(e) => {
            match e.kind() {
                // Jika file emang sudah tidak ada (mungkin dihapus user lain duluan), itu OK.
                ErrorKind::NotFound => {
                    println!("{YELLOW}[INFO]{RESET} History file already gone or doesn't exist.");
                }
                // Jika ada masalah izin akses (Permission Denied)
                ErrorKind::PermissionDenied => {
                    eprintln!("{RED}[ERROR]{RESET} Cannot delete history: Permission denied.");
                }
                // Masalah tak terduga lainnya (misal file corrupt atau locked oleh process lain)
                _ => {
                    eprintln!("{RED}[ERROR]{RESET} Unexpected error while resetting history: {}", e);
                }
            }
        }
    }

    // 3. Pastikan state Editor sinkron dengan disk (buat file kosong baru jika perlu)
    // Ini penting agar instance MELISA ini tidak error saat mencoba save_history nanti.
    if let Err(e) = rl.save_history(history_path) {
        eprintln!("{YELLOW}[WARN]{RESET} Failed to re-initialize empty history file: {}", e);
    }

    println!("{GREEN}[DONE]{RESET} Reset process finished.");
}