use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::core::root_check::{ensure_admin, check_if_admin};
use crate::cli::color_text::{BOLD, GREEN, RED, RESET, YELLOW};

pub enum UserRole {
    Admin,
    Regular,
}

// --- USER MANAGEMENT ---

/// Provisions a new user specifically for the MELISA environment.
/// Assigns the custom shell and configures appropriate sudo privileges.
///
/// `audit` tidak mengubah perilaku fungsi ini secara signifikan karena
/// `passwd` harus interaktif. Flag diteruskan ke `configure_sudoers` untuk
/// konsistensi API.
pub async fn add_melisa_user(username: &str, audit: bool) {
    if !ensure_admin().await {
        return;
    }
    println!("\n{}--- Provisioning New MELISA User: {} ---{}", BOLD, username, RESET);

    println!("{}Select Access Level for {}:{}", BOLD, username, RESET);
    println!("  1) Administrator (Full Management: Users, Projects, & LXC)");
    println!("  2) Standard User (Project & LXC Management Only)");
    print!("Enter choice (1/2): ");
    let _ = io::stdout().flush().await;

    let mut choice = String::new();
    let stdin = std::io::stdin();
    let _ = stdin.read_line(&mut choice);

    let role = match choice.trim() {
        "1" => UserRole::Admin,
        _ => UserRole::Regular,
    };

    let status = if audit {
        println!("[AUDIT] Running: useradd -m -s /usr/local/bin/melisa {}", username);
        Command::new("sudo")
            .args(&["useradd", "-m", "-s", "/usr/local/bin/melisa", username])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    } else {
        Command::new("sudo")
            .args(&["useradd", "-m", "-s", "/usr/local/bin/melisa", username])
            .status()
            .await
    };

    match status {
        Ok(s) if s.success() => {
            println!("{}[SUCCESS]{} User account '{}' successfully created.", GREEN, RESET, username);

            let folder_path = format!("/home/{}", username);
            let _ = Command::new("sudo")
                .args(&["chmod", "700", &folder_path])
                .status()
                .await;

            if set_user_password(username).await {
                configure_sudoers(username, role, audit).await;
            }
        }
        _ => {
            eprintln!("{}[ERROR]{} Failed to create user. The username might already exist.", RED, RESET);
        }
    }
}

/// Triggers the interactive passwd prompt to set or update a user's password.
/// Selalu interaktif — tidak terpengaruh flag audit karena memerlukan input pengguna.
pub async fn set_user_password(username: &str) -> bool {
    println!("{}[ACTION]{} Please set the authentication password for {}:", YELLOW, RESET, username);

    // passwd harus selalu interaktif (inherit stdio) agar pengguna bisa mengetik password.
    let status = Command::new("sudo")
        .args(&["passwd", username])
        .status()
        .await;

    match status {
        Ok(s) if s.success() => {
            println!("{}[SUCCESS]{} Password successfully updated for {}.", GREEN, RESET, username);
            true
        }
        _ => {
            eprintln!("{}[ERROR]{} Failed to update the password.", RED, RESET);
            false
        }
    }
}

/// Completely removes a user, terminates their processes, and cleans up their home directory.
///
/// Ketika `audit = true`, output dari pkill dan userdel diteruskan ke terminal.
pub async fn delete_melisa_user(username: &str, audit: bool) {
    if !ensure_admin().await {
        return;
    }
    println!("\n{}--- Initiating Deletion for User: {} ---{}", BOLD, username, RESET);

    // 1. Terminate all active processes owned by the user
    println!("{}[INFO]{} Terminating all active processes for user '{}'...", YELLOW, RESET, username);

    if audit {
        println!("[AUDIT] Running: pkill -u {}", username);
        let _ = Command::new("sudo")
            .args(&["pkill", "-u", username])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await;
    } else {
        let _ = Command::new("sudo")
            .args(&["pkill", "-u", username])
            .status()
            .await;
    }

    // 2. Delete the user and forcefully remove their home directory
    if audit {
        println!("[AUDIT] Running: userdel -r -f {}", username);
    }

    let status_del = if audit {
        Command::new("sudo")
            .args(&["userdel", "-r", "-f", username])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    } else {
        Command::new("sudo")
            .args(&["userdel", "-r", "-f", username])
            .status()
            .await
    };

    // 3. Remove the user's specific sudoers configuration file
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);
    let status_rm = Command::new("sudo")
        .args(&["rm", "-f", &sudoers_path])
        .status()
        .await;

    match (status_del, status_rm) {
        (Ok(s1), Ok(s2)) if s1.success() && s2.success() => {
            println!(
                "{}[SUCCESS]{} User '{}' and all associated permissions have been completely removed.",
                GREEN, RESET, username
            );
        }
        _ => {
            eprintln!(
                "{}[WARNING]{} Deletion process encountered anomalies (User or files might already be removed).",
                RED, RESET
            );
        }
    }
}

/// Generates and deploys a custom sudoers file for the user,
/// defining their exact system privileges.
///
/// Ketika `audit = true`, isi sudoers yang akan ditulis dicetak ke terminal.
async fn configure_sudoers(username: &str, role: UserRole, audit: bool) {
    let mut commands = vec![
        "/usr/bin/lxc-*", "/bin/lxc-*",
        "/usr/sbin/lxc-*", "/sbin/lxc-*",
        "/usr/share/lxc/templates/lxc-download *",
        "/usr/bin/git *", "/bin/git *",
        "/usr/local/bin/melisa *",
        "/usr/bin/mkdir -p *", "/bin/mkdir -p *",
        "/usr/bin/rm -f *", "/bin/rm -f *",
        "/usr/bin/bash -c *", "/bin/bash -c *",
        "/usr/bin/tee *", "/bin/tee *",
        "/usr/bin/chattr *", "/bin/chattr *",
    ];

    match role {
        UserRole::Admin => {
            commands.extend(vec![
                "/usr/sbin/useradd *", "/sbin/useradd *",
                "/usr/sbin/userdel *", "/sbin/userdel *",
                "/usr/bin/passwd *", "/bin/passwd *",
                "/usr/bin/pkill *", "/bin/pkill *",
                "/usr/bin/grep *", "/bin/grep *",
                "/usr/bin/lxc-info *", "/bin/lxc-info *",
                "/usr/bin/ls /etc/sudoers.d/", "/bin/ls /etc/sudoers.d/",
                "/usr/bin/rm -f /etc/sudoers.d/melisa_*",
                "/bin/rm -f /etc/sudoers.d/melisa_*",
                "/usr/bin/tee /etc/sudoers.d/melisa_*",
                "/bin/tee /etc/sudoers.d/melisa_*",
                "/usr/bin/chmod *", "/bin/chmod *", "/usr/sbin/chmod *", "/sbin/chmod *",
                "/usr/bin/chown *", "/bin/chown *", "/usr/sbin/chown *", "/sbin/chown *",
                "/usr/bin/mkdir *", "/bin/mkdir *",
            ]);
        }
        UserRole::Regular => {
            // Standard users only retain the base commands defined above.
        }
    }

    let sudoers_rule = format!("{} ALL=(ALL) NOPASSWD: {}\n", username, commands.join(", "));
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);

    if audit {
        println!("[AUDIT] Writing sudoers rule to {}:", sudoers_path);
        println!("{}", sudoers_rule.trim());
    }

    let child_process = Command::new("sudo")
        .args(&["tee", &sudoers_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn();

    match child_process {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                if let Err(e) = stdin.write_all(sudoers_rule.as_bytes()).await {
                    eprintln!("{}[ERROR]{} Failed to write to standard input of tee: {}", RED, RESET, e);
                    return;
                }
            }
            let _ = child.wait().await;

            let _ = Command::new("sudo")
                .args(&["chmod", "0440", &sudoers_path])
                .status()
                .await;
            println!("{}[SUCCESS]{} Privilege configuration deployed successfully.", GREEN, RESET);
        }
        Err(e) => eprintln!(
            "{}[FATAL]{} Failed to spawn tee command to deploy sudoers file: {}",
            RED, RESET, e
        ),
    }
}

/// Scans the system for users operating under the MELISA shell
/// and checks for orphaned configurations.
pub async fn list_melisa_users() {
    if !ensure_admin().await {
        return;
    }
    println!("\n{}--- Registered MELISA Accounts ---{}", BOLD, RESET);

    let passwd_out = Command::new("sudo")
        .args(&["grep", "/usr/local/bin/melisa", "/etc/passwd"])
        .output()
        .await;

    let mut existing_users = Vec::new();

    if let Ok(out) = passwd_out {
        let result = String::from_utf8_lossy(&out.stdout);
        for line in result.lines() {
            if let Some(user) = line.split(':').next() {
                existing_users.push(user.to_string());
                let tag = if check_if_admin(user).await {
                    format!("{}[ADMINISTRATOR]{}", GREEN, RESET)
                } else {
                    format!("{}[STANDARD USER]{}", YELLOW, RESET)
                };
                println!("  > {:<20} {}", user, tag);
            }
        }
    }

    println!("\n{}--- Scanning for Orphaned Sudoers Configurations ---{}", BOLD, RESET);

    let sudoers_files = Command::new("sudo")
        .args(&["ls", "/etc/sudoers.d/"])
        .output()
        .await;

    match sudoers_files {
        Ok(out) if out.status.success() => {
            let files = String::from_utf8_lossy(&out.stdout);
            let mut found_trash = false;

            for file in files.lines() {
                if file.starts_with("melisa_") {
                    let user_from_file = file.replace("melisa_", "");
                    if !existing_users.contains(&user_from_file) {
                        println!(
                            "  {}! Orphan Detected:{} {} (User account no longer exists)",
                            RED, RESET, file
                        );
                        found_trash = true;
                    }
                }
            }
            if !found_trash {
                println!(
                    "  {}No orphaned configurations found. System state is clean.{}",
                    GREEN, RESET
                );
            }
        }
        _ => println!(
            "{}[ERROR]{} Failed to access the /etc/sudoers.d/ directory.",
            RED, RESET
        ),
    }
}

/// Updates the access control level for an existing system user by re-deploying sudoers.
pub async fn upgrade_user(username: &str, audit: bool) {
    if !ensure_admin().await {
        return;
    }

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    let header = format!("\n{}--- Modifying Access Privileges for: {} ---{}\n", BOLD, username, RESET);
    let _ = stdout.write_all(header.as_bytes()).await;

    let check_user = Command::new("id").arg(username).output().await;
    if let Ok(out) = check_user {
        if !out.status.success() {
            let err = format!("{}[ERROR]{} Target user '{}' not found.\n", RED, RESET, username);
            let _ = stdout.write_all(err.as_bytes()).await;
            return;
        }
    }

    let menu = format!(
        "Select Target Role for {}:\n  1) Administrator (Full Access)\n  2) Standard User (LXC & Project Only)\n",
        username
    );
    let _ = stdout.write_all(menu.as_bytes()).await;

    let _ = stdout.write_all(b"Enter choice (1/2): ").await;
    let _ = stdout.flush().await;

    let mut choice = String::new();
    if let Err(e) = reader.read_line(&mut choice).await {
        eprintln!("{}[ERROR]{} Failed to read input: {}", RED, RESET, e);
        return;
    }

    let role = match choice.trim() {
        "1" => UserRole::Admin,
        _ => UserRole::Regular,
    };

    configure_sudoers(username, role, audit).await;

    let success_msg = format!("{}[DONE]{} Privileges for '{}' updated successfully.\n", GREEN, RESET, username);
    let _ = stdout.write_all(success_msg.as_bytes()).await;
    let _ = stdout.flush().await;
}

/// Purges any sudoers files left behind by manually deleted user accounts.
pub async fn clean_orphaned_sudoers() {
    if !ensure_admin().await {
        return;
    }
    println!("\n{}--- Executing Orphaned Configuration Cleanup ---{}", BOLD, RESET);

    let passwd_out = Command::new("sudo")
        .args(&["grep", "/usr/local/bin/melisa", "/etc/passwd"])
        .output()
        .await;

    if let Ok(out) = passwd_out {
        let result = String::from_utf8_lossy(&out.stdout);
        let existing_users: Vec<&str> = result
            .lines()
            .map(|l| l.split(':').next().unwrap_or(""))
            .collect();

        let files_out = Command::new("sudo")
            .args(&["ls", "/etc/sudoers.d/"])
            .output()
            .await;

        if let Ok(f_out) = files_out {
            let files = String::from_utf8_lossy(&f_out.stdout);
            for file in files.lines() {
                if file.starts_with("melisa_") {
                    let user_name = file.replace("melisa_", "");
                    if !existing_users.contains(&user_name.as_str()) {
                        println!(
                            "{}[PURGING]{} Removing orphaned configuration file: {}",
                            YELLOW, RESET, file
                        );
                        let _ = Command::new("sudo")
                            .args(&["rm", "-f", &format!("/etc/sudoers.d/{}", file)])
                            .status()
                            .await;
                    }
                }
            }
        }
    }
    println!("{}[DONE]{} System garbage collection completed successfully.", GREEN, RESET);
}