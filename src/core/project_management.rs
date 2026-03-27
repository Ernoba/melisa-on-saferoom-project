use tokio::process::Command;
use std::process::Stdio;
use std::path::Path;
use tokio::fs;

use crate::core::root_check::admin_check;
use crate::cli::color_text::{RED, GREEN, BLUE, YELLOW, BOLD, RESET};

pub const PROJECTS_MASTER: &str = "/opt/melisa/projects";

/// Utility — check if a user already has a local project workspace (`.git` folder).
async fn check_user_in_project(username: &str, project_name: &str) -> bool {
    let git_path = Path::new("/home")
        .join(username)
        .join(project_name)
        .join(".git");
    git_path.exists()
}

/// Initializes a new master bare repository for a project.
/// This acts as the central source of truth for all users collaborating on the project.
///
/// Ketika `audit = true`, output dari perintah `git init` dan `chmod` diteruskan ke terminal.
pub async fn new_project(project_name: &str, audit: bool) {
    let master_path = format!("{}/{}", PROJECTS_MASTER, project_name);

    if let Err(e) = fs::create_dir_all(&master_path).await {
        eprintln!("{}[FATAL]{} Failed to create master directory structure: {}", RED, RESET, e);
        return;
    }

    // 1. Initialize Bare Repository with Shared Group mode
    let init_status = if audit {
        println!("[AUDIT] Running: git init --bare --shared=group {}", master_path);
        Command::new("git")
            .args(&["init", "--bare", "--shared=group", &master_path])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    } else {
        Command::new("git")
            .args(&["init", "--bare", "--shared=group", &master_path])
            .status()
            .await
    };

    if let Ok(s) = init_status {
        if !s.success() {
            eprintln!("{}[ERROR]{} Git bare repository initialization failed.", RED, RESET);
            return;
        }
    }

    // Register safe directory
    let _ = Command::new("git")
        .args(&["config", "--system", "--add", "safe.directory", &master_path])
        .status()
        .await;

    // Permission & Group Security
    let _ = Command::new("chown")
        .args(&["-R", "root:melisa", &master_path])
        .status()
        .await;
    let _ = Command::new("chmod")
        .args(&["-R", "2775", &master_path])
        .status()
        .await;

    let _ = Command::new("git")
        .args(&["-C", &master_path, "config", "core.sharedRepository", "group"])
        .status()
        .await;

    // 2. Setup Post-Receive Hook
    let hook_path = format!("{}/hooks/post-receive", master_path);
    let hook_content = format!("#!/bin/bash\nsudo melisa --update-all {}", project_name);

    match fs::write(&hook_path, hook_content).await {
        Ok(_) => {
            let _ = Command::new("chmod").args(&["+x", &hook_path]).status().await;
            println!(
                "{}[SUCCESS]{} Master Git repository '{}' initialized and security protocols applied.",
                GREEN, RESET, project_name
            );
        }
        Err(e) => eprintln!("{}[ERROR]{} Failed to write post-receive hook: {}", RED, RESET, e),
    }
}

/// Invites specific users to a project by cloning the master repository into their home directories.
///
/// Ketika `audit = true`, output git clone diteruskan ke terminal.
pub async fn invite(project_name: &str, invited_users: &[&str], audit: bool) {
    let master_path = format!("{}/{}", PROJECTS_MASTER, project_name);

    for username in invited_users {
        let user_project_path = format!("/home/{}/{}", username, project_name);

        let _ = Command::new("rm").args(&["-rf", &user_project_path]).status().await;

        let _ = Command::new("sudo")
            .args(&[
                "-u", username, "git", "config", "--global",
                "--add", "safe.directory", &master_path,
            ])
            .status()
            .await;

        if audit {
            println!(
                "[AUDIT] Running: git clone {} {} (as {})",
                master_path, user_project_path, username
            );
        }

        let clone_status = if audit {
            Command::new("sudo")
                .args(&["-u", username, "git", "clone", &master_path, &user_project_path])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
        } else {
            Command::new("sudo")
                .args(&["-u", username, "git", "clone", &master_path, &user_project_path])
                .status()
                .await
        };

        match clone_status {
            Ok(s) if s.success() => {
                let _ = Command::new("chown")
                    .args(&["-R", &format!("{}:{}", username, username), &user_project_path])
                    .status()
                    .await;
                println!(
                    "{}[INVITED]{} User workspace for '{}' successfully provisioned.",
                    GREEN, RESET, username
                );
            }
            _ => {
                // Fallback: master repo is empty, initialize manually.
                let _ = Command::new("sudo")
                    .args(&["-u", username, "mkdir", "-p", &user_project_path])
                    .status()
                    .await;
                let _ = Command::new("sudo")
                    .args(&["-u", username, "git", "-C", &user_project_path, "init"])
                    .status()
                    .await;
                let _ = Command::new("sudo")
                    .args(&[
                        "-u", username, "git", "-C", &user_project_path,
                        "remote", "add", "origin", &master_path,
                    ])
                    .status()
                    .await;

                let _ = Command::new("chown")
                    .args(&["-R", &format!("{}:{}", username, username), &user_project_path])
                    .status()
                    .await;

                println!(
                    "{}[WARNING]{} Master repository is empty. Workspace for '{}' initialized manually.",
                    YELLOW, RESET, username
                );
            }
        }
    }
}

/// Automatically commits and pushes a user's local changes to the master repository.
///
/// Ketika `audit = true`, output git add / commit / push diteruskan ke terminal.
pub async fn pull(username: &str, project_name: &str, audit: bool) -> bool {
    if !check_user_in_project(username, project_name).await {
        eprintln!(
            "{}[ERROR]{} User '{}' does not have a workspace for project '{}'.",
            RED, RESET, username, project_name
        );
        eprintln!(
            "{}[TIP]{} Run: melisa --invite {} {}",
            YELLOW, RESET, project_name, username
        );
        return false;
    }

    let user_path = format!("/home/{}/{}", username, project_name);

    let branch_out = Command::new("sudo")
        .args(&["-u", username, "git", "-C", &user_path, "branch", "--show-current"])
        .output()
        .await;

    let branch = String::from_utf8_lossy(
        &branch_out.as_ref().map(|o| o.stdout.clone()).unwrap_or_default(),
    )
    .trim()
    .to_string();
    let branch = if branch.is_empty() { "master".to_string() } else { branch };

    println!(
        "{}[INFO]{} Pulling from '{}' workspace into master (Branch: {})...",
        BLUE, RESET, username, branch
    );

    // Stage all changes
    let _ = Command::new("sudo")
        .args(&["-u", username, "git", "-C", &user_path, "add", "."])
        .status()
        .await;

    // Commit
    if audit {
        println!("[AUDIT] Running: git commit --allow-empty (as {})", username);
        let _ = Command::new("sudo")
            .args(&[
                "-u", username, "git", "-C", &user_path,
                "commit", "-m", "Admin force-pull: executed by MELISA", "--allow-empty",
            ])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await;
    } else {
        let _ = Command::new("sudo")
            .args(&[
                "-u", username, "git", "-C", &user_path,
                "commit", "-m", "Admin force-pull: executed by MELISA", "--allow-empty",
            ])
            .status()
            .await;
    }

    // Push to master
    if audit {
        println!("[AUDIT] Running: git push origin {} (as {})", branch, username);
    }

    let push_status = if audit {
        Command::new("sudo")
            .args(&["-u", username, "git", "-C", &user_path, "push", "origin", &branch])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    } else {
        Command::new("sudo")
            .args(&["-u", username, "git", "-C", &user_path, "push", "origin", &branch])
            .status()
            .await
    };

    match push_status {
        Ok(s) if s.success() => {
            println!(
                "{}[SUCCESS]{} Workspace '{}@{}' successfully pulled into master.",
                GREEN, RESET, username, project_name
            );
            true
        }
        _ => {
            eprintln!(
                "{}[ERROR]{} Failed to push '{}' workspace to master. Possible divergence.",
                RED, RESET, username
            );
            eprintln!(
                "{}[TIP]{} Consider: melisa --update {} --force (to reset their workspace first)",
                YELLOW, RESET, project_name
            );
            false
        }
    }
}

/// Displays an overview of all projects.
/// Admins see the root master projects; standard users see their local cloned workspaces.
pub async fn list_projects(home: &str) {
    let is_admin = admin_check().await;
    println!("\n{}--- MELISA PROJECT DASHBOARD ---{}", BOLD, RESET);

    if is_admin {
        let output = Command::new("ls").args(&["-1", PROJECTS_MASTER]).output().await;

        match output {
            Ok(out) if out.status.success() => {
                let list = String::from_utf8_lossy(&out.stdout);
                if list.trim().is_empty() {
                    println!("  {}No Master Projects have been established yet.{}", YELLOW, RESET);
                } else {
                    println!("{}Master Repositories (Root Infrastructure):{}", BOLD, RESET);
                    for project in list.lines() {
                        println!("  {} [MASTER] {}{}", GREEN, project, RESET);
                    }
                }
            }
            _ => eprintln!(
                "{}[ERROR]{} Denied or failed access to the master projects directory.",
                RED, RESET
            ),
        }
    } else {
        let output = Command::new("ls").args(&["-F", home]).output().await;

        if let Ok(out) = output {
            let list = String::from_utf8_lossy(&out.stdout);
            let mut found = false;

            println!("{}Active Workspace Assignments:{}", BOLD, RESET);
            for entry in list.lines() {
                if entry.ends_with('/') && entry != "data/" {
                    println!("  {} [WORKSPACE] {}{}", BLUE, entry.trim_end_matches('/'), RESET);
                    found = true;
                }
            }

            if !found {
                println!(
                    "  {}You have not been assigned to any active projects.{}",
                    YELLOW, RESET
                );
            }
        }
    }
}

/// Completely obliterates a project from the master directory and from all users' local workspaces.
pub async fn delete_project(master_path: String, project_name: &str) {
    println!(
        "{}[WARNING]{} Initiating total wipe sequence for project '{}'...",
        YELLOW, RESET, project_name
    );

    let _ = Command::new("rm").args(&["-rf", &master_path]).status().await;

    let passwd_out = Command::new("grep")
        .args(&["/usr/local/bin/melisa", "/etc/passwd"])
        .output()
        .await;

    if let Ok(out) = passwd_out {
        let result = String::from_utf8_lossy(&out.stdout);
        for line in result.lines() {
            if let Some(username) = line.split(':').next() {
                let user_project_path = format!("/home/{}/{}", username, project_name);

                if Path::new(&user_project_path).exists() {
                    let _ = Command::new("rm")
                        .args(&["-rf", &user_project_path])
                        .status()
                        .await;
                    println!("  {}[DELETED]{} Workspace removed for user '{}'.", YELLOW, RESET, username);
                }
            }
        }
        println!(
            "{}[SUCCESS]{} Project '{}' completely eradicated from the server infrastructure.",
            GREEN, RESET, project_name
        );
    } else {
        eprintln!(
            "{}[ERROR]{} Failed to retrieve user list during deletion sequence.",
            RED, RESET
        );
    }
}

/// Revokes project access for specific users by deleting their local workspace clones.
pub async fn out_user(targets: &[&str], project_name: &str) {
    for username in targets {
        let user_project_path = format!("/home/{}/{}", username, project_name);
        let status = Command::new("rm").args(&["-rf", &user_project_path]).status().await;

        match status {
            Ok(s) if s.success() => {
                println!(
                    "{}[REVOKED]{} User '{}' has been successfully removed from project '{}'.",
                    YELLOW, RESET, username, project_name
                );
            }
            _ => eprintln!(
                "{}[ERROR]{} Failed to purge project workspace for user '{}'.",
                RED, RESET, username
            ),
        }
    }
}

/// Forcefully syncs a user's local workspace with the latest state of the master repository.
/// Typically triggered by the post-receive hook.
///
/// Ketika `audit = true`, output dari setiap perintah git diteruskan ke terminal.
pub async fn update_project(username: &str, project_name: &str, force: bool, audit: bool) {
    if username.contains('/') || username.contains("..")
        || project_name.contains('/') || project_name.contains("..")
    {
        eprintln!("{}[ERROR]{} Invalid characters detected in input. Sync aborted.", RED, RESET);
        return;
    }

    let base_path = Path::new("/home").join(username).join(project_name);
    let user_path = base_path.to_str().unwrap_or_default().to_string();
    let git_path = base_path.join(".git");

    if !git_path.exists() {
        eprintln!(
            "{}[ERROR]{} Target path '{}' is not a valid Git repository. Sync aborted.",
            RED, RESET, user_path
        );
        return;
    }

    let branch_out = Command::new("sudo")
        .args(&["-u", username, "git", "-C", &user_path, "branch", "--show-current"])
        .output()
        .await;

    let mut branch = String::from_utf8_lossy(
        &branch_out.as_ref().map(|o| o.stdout.clone()).unwrap_or_default(),
    )
    .trim()
    .to_string();
    if branch.is_empty() {
        branch = "master".to_string();
    }

    if force {
        println!(
            "{}[SYNC/FORCE]{} Hard reset for '{}@{}' (Branch: {})...",
            YELLOW, RESET, project_name, username, branch
        );

        // Fix ownership
        let _ = Command::new("sudo")
            .args(&["chown", "-R", &format!("{}:{}", username, username), &user_path])
            .status()
            .await;

        // Clean untracked files
        if audit {
            println!("[AUDIT] Running: git clean -fd (as {})", username);
            let _ = Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "clean", "-fd"])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await;
        } else {
            let _ = Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "clean", "-fd"])
                .status()
                .await;
        }

        // Fetch latest
        if audit {
            println!("[AUDIT] Running: git fetch origin (as {})", username);
            let _ = Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "fetch", "origin"])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await;
        } else {
            let _ = Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "fetch", "origin"])
                .status()
                .await;
        }

        // Hard reset
        if audit {
            println!(
                "[AUDIT] Running: git reset --hard origin/{} (as {})",
                branch, username
            );
        }

        let status = if audit {
            Command::new("sudo")
                .args(&[
                    "-u", username, "git", "-C", &user_path,
                    "reset", "--hard", &format!("origin/{}", branch),
                ])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
        } else {
            Command::new("sudo")
                .args(&[
                    "-u", username, "git", "-C", &user_path,
                    "reset", "--hard", &format!("origin/{}", branch),
                ])
                .status()
                .await
        };

        match status {
            Ok(s) if s.success() => {
                println!(
                    "{}[SUCCESS]{} '{}' (user: {}) forcefully synchronized to master state.",
                    GREEN, RESET, project_name, username
                );
            }
            _ => eprintln!(
                "{}[ERROR]{} Force sync failed for user '{}' project '{}'.",
                RED, RESET, username, project_name
            ),
        }
    } else {
        println!(
            "{}[SYNC/SAFE]{} Safe update for '{}@{}' (Branch: {})...",
            BLUE, RESET, project_name, username, branch
        );

        // Fetch without changing workspace
        if audit {
            println!("[AUDIT] Running: git fetch origin (as {})", username);
        }

        let fetch_status = if audit {
            Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "fetch", "origin"])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
        } else {
            Command::new("sudo")
                .args(&["-u", username, "git", "-C", &user_path, "fetch", "origin"])
                .status()
                .await
        };

        if fetch_status.as_ref().map(|s| !s.success()).unwrap_or(true) {
            eprintln!(
                "{}[ERROR]{} Failed to fetch from master for user '{}'. Check network/repo.",
                RED, RESET, username
            );
            return;
        }

        // Fast-forward merge
        if audit {
            println!(
                "[AUDIT] Running: git merge --ff-only origin/{} (as {})",
                branch, username
            );
        }

        let merge_status = if audit {
            Command::new("sudo")
                .args(&[
                    "-u", username, "git", "-C", &user_path,
                    "merge", "--ff-only", &format!("origin/{}", branch),
                ])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .await
        } else {
            Command::new("sudo")
                .args(&[
                    "-u", username, "git", "-C", &user_path,
                    "merge", "--ff-only", &format!("origin/{}", branch),
                ])
                .status()
                .await
        };

        match merge_status {
            Ok(s) if s.success() => {
                println!(
                    "{}[SUCCESS]{} '{}' (user: {}) safely updated to latest master.",
                    GREEN, RESET, project_name, username
                );
            }
            Ok(_) => {
                println!(
                    "{}[INFO]{} Cannot fast-forward '{}' for user '{}'.",
                    YELLOW, RESET, project_name, username
                );
                println!("{}[TIP]{} Local branch has diverged from master.", YELLOW, RESET);
                println!(
                    "{}[TIP]{} Use 'melisa --update {} --force' to discard local changes,",
                    YELLOW, RESET, project_name
                );
                println!(
                    "{}[TIP]{} or resolve manually: ssh to server → cd ~/{} → git status",
                    YELLOW, RESET, project_name
                );
            }
            Err(e) => {
                eprintln!("{}[ERROR]{} Merge command failed: {}", RED, RESET, e);
            }
        }
    }
}

/// Triggers a hard update across ALL users assigned to a specific project.
/// This is the master command executed by the Git post-receive hook.
pub async fn update_all_users(project_name: &str, audit: bool) {
    let output = Command::new("grep")
        .args(&["/usr/local/bin/melisa", "/etc/passwd"])
        .output()
        .await;

    if let Ok(out) = output {
        let result = String::from_utf8_lossy(&out.stdout);
        for line in result.lines() {
            if let Some(username) = line.split(':').next() {
                let user_project_path = format!("/home/{}/{}", username, project_name);

                if Path::new(&user_project_path).exists() {
                    update_project(username, project_name, true, audit).await;
                }
            }
        }
    }
}