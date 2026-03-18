use tokio::process::Command; // Diubah ke tokio
use std::env;
use crate::cli::color_text::{RED, RESET};

pub fn check_root() -> bool {
    // Pastikan sudah menambahkan 'libc = "0.2"' di Cargo.toml
    unsafe { libc::geteuid() == 0 }
}

pub async fn check_if_admin(username: &str) -> bool {
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);
    
    let check_admin = Command::new("sudo")
        .arg("-n") // <--- KUNCI SAKTINYA DI SINI (Non-interactive)
        .args(&["/usr/bin/grep", "-qs", "useradd", &sudoers_path])
        .status()
        .await; // <--- WAJIB AWAIT

    match check_admin {
        // Jika sukses (0), berarti dia Admin dan punya izin NOPASSWD
        Ok(s) if s.success() => true,
        // Jika gagal (karena nggak ada izin atau perlu password), langsung anggap bukan Admin
        _ => false, 
    }
}

// Fungsi untuk mengecek apakah user yang sedang menjalankan aplikasi adalah Admin
pub async fn ensure_admin() -> bool {
    if !admin_check().await {
        println!("{}[ERROR] Permission not allowed. This function is for admin only.{}", RED, RESET);
        return false;
    }
    true
}

pub async fn admin_check() -> bool {
    let current_user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    check_if_admin(&current_user).await
}

// chek apakah user ssh atau host
// Di src/core/root_check.rs
pub async fn is_ssh_session() -> bool {
    // 1. Cek Environment Variables (Pastikan isinya tidak kosong)
    if !env::var("SSH_CLIENT").unwrap_or_default().is_empty() || 
       !env::var("SSH_TTY").unwrap_or_default().is_empty() || 
       !env::var("SSH_CONNECTION").unwrap_or_default().is_empty() {
        return true;
    }

    // 2. Verifikasi via 'who -m'
    let output = Command::new("who")
        .arg("-m")
        .output()
        .await;

    if let Ok(out) = output {
        let status = String::from_utf8_lossy(&out.stdout);
        // PERBAIKAN: Hanya anggap SSH jika ada kurung DAN bukan merupakan display lokal (:0)
        // Di src/core/root_check.rs, ubah bagian ini:
            if status.contains('(') && status.contains(')') && !status.contains("(:") {
                return true; // Hanya anggap SSH jika bukan display lokal (:0)
            }
        }

    false
}