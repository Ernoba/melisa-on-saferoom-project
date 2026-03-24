use tokio::process::Command;
use std::env;
use crate::cli::color_text::{RED, RESET};

/// Verifies if the current process is executing with root (UID 0) privileges.
/// Wrapped in a Unix configuration attribute to ensure safe cross-platform compilation.
#[cfg(target_family = "unix")]
pub fn check_root() -> bool {
    // Utilizing libc to directly query the Effective User ID (EUID) at the OS level
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(target_family = "unix"))]
pub fn check_root() -> bool {
    false // Fallback safety for non-Unix compilation targets
}

/// Verifies if a specific user has Administrative privileges within the MELISA ecosystem.
/// This is determined by checking their specific sudoers file for elevated commands.
pub async fn check_if_admin(username: &str) -> bool {
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);
    
    // Execute grep non-interactively via sudo (-n) to search for admin-only commands (e.g., 'useradd').
    // The '-n' flag is the secret weapon: it prevents the command from hanging and waiting
    // for a password prompt if the user does NOT have NOPASSWD privileges.
    let check_admin = Command::new("sudo")
        .arg("-n") 
        .args(&["/usr/bin/grep", "-qs", "useradd", &sudoers_path])
        .status()
        .await;

    match check_admin {
        // Exit code 0 means grep found the string AND sudo executed without requiring a password.
        Ok(s) if s.success() => true,
        // Any failure (file not found, pattern not found, or sudo requires a password) means Non-Admin.
        _ => false, 
    }
}

/// Primary gatekeeper function. Validates admin privileges and logs an error if denied.
pub async fn ensure_admin() -> bool {
    if !admin_check().await {
        println!("{}[ERROR] Access Denied. This operation requires Administrative privileges.{}", RED, RESET);
        return false;
    }
    true
}

/// Resolves the current executing user and checks their administrative status.
pub async fn admin_check() -> bool {
    // If the process is natively running as root, immediately grant admin status.
    if check_root() {
        return true;
    }

    // Prioritize SUDO_USER to detect the real human user behind a sudo elevation
    let current_user = env::var("SUDO_USER")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
        
    check_if_admin(&current_user).await
}

/// Detects if the current terminal session is being accessed remotely via SSH.
/// This is critical for blocking certain host-level initialization commands from remote execution.
pub async fn is_ssh_session() -> bool {
    // 1. Primary Check: Evaluate standard SSH environment variables
    let ssh_client = env::var("SSH_CLIENT").unwrap_or_default();
    let ssh_tty = env::var("SSH_TTY").unwrap_or_default();
    let ssh_connection = env::var("SSH_CONNECTION").unwrap_or_default();

    if !ssh_client.is_empty() || !ssh_tty.is_empty() || !ssh_connection.is_empty() {
        return true;
    }

    // 2. Secondary Check: Verify via the 'who -m' command
    // This catches edge cases where environment variables might be dropped during sudo escalation
    let output = Command::new("who")
        .arg("-m")
        .output()
        .await;

    if let Ok(out) = output {
        let status = String::from_utf8_lossy(&out.stdout);
        
        // Analyze the output format. Remote sessions typically append the origin IP in parentheses.
        // We explicitly exclude local display sessions (e.g., "(:0)") which also use parentheses.
        if status.contains('(') && status.contains(')') && !status.contains("(:") {
            return true; 
        }
    }

    false
}