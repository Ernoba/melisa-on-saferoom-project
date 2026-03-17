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
pub async fn is_ssh_session() -> bool {
    // Metode 1: Cek Environment Variables (Cepat)
    // Saat login via SSH, variabel ini otomatis ada.
    if env::var("SSH_CLIENT").is_ok() || env::var("SSH_TTY").is_ok() || env::var("SSH_CONNECTION").is_ok() {
        return true;
    }

    // Metode 2: Verifikasi via perintah 'who -m' (Lebih Akurat)
    // 'who -m' akan menampilkan detail terminal yang sedang digunakan.
    // Jika ada alamat IP/hostname dalam kurung di akhir, itu adalah SSH.
    let output = Command::new("who")
        .arg("-m")
        .output()
        .await; // <--- WAJIB AWAIT

    if let Ok(out) = output {
        let status = String::from_utf8_lossy(&out.stdout);
        // Sesi SSH biasanya terlihat seperti: "user pts/0 2026-03-17 (192.168.1.5)"
        // Sesi lokal biasanya terlihat seperti: "user tty1 2026-03-17"
        if status.contains('(') && status.contains(')') {
            return true;
        }
    }

    false
}