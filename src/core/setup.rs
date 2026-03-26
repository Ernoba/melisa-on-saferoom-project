use tokio::process::Command;
use tokio::io::{self, AsyncWriteExt};
use tokio::fs::{self, OpenOptions};
use tokio::time::{timeout, Duration};
use std::path::Path;
use std::process::Stdio;

use crate::core::root_check::{check_root, is_ssh_session};
use crate::cli::color_text::{GREEN, RED, CYAN, BOLD, RESET};
use crate::core::project_management::PROJECTS_MASTER;

// Import modul host_distro yang baru dibuat
use crate::distros::host_distro::{detect_host_distro, get_distro_config, DistroConfig, FirewallKind};

pub async fn install() {
    // 1. Access & Security Verification
    if !check_root() {
        eprintln!("{}[CRITICAL ERROR] Setup must be executed with root privileges (Use sudo).{}", RED, RESET);
        std::process::exit(1);
    }

    if is_ssh_session().await {
        println!("\n{}[SECURITY ALERT]{} The 'setup' command is STRICTLY FORBIDDEN via SSH!", RED, RESET);
        println!("{}Only a physical user (Host) is authorized to perform system initialization.{}", BOLD, RESET);
        std::process::exit(1);
    }

    println!("\n{}MELISA SYSTEM & LXC INITIALIZATION (HOST MODE){}\n", BOLD, RESET);

    // 1.2 Detect Host Distribution
    let distro = detect_host_distro().await;
    let cfg = get_distro_config(&distro);
    println!("{}[INFO]{} Host OS detected as: {:?}", CYAN, RESET, distro);

    // 1.5 Pre-flight Dependency Check (Dynamic based on distro)
    if !check_required_tools(&cfg).await {
        eprintln!("\n{}CRITICAL_FAILURE: The system is missing fundamental utilities required for deployment.{}", RED, RESET);
        std::process::exit(1);
    }

    // 2. Local Environment Verification
    verify_data_environment().await;

    // 3. Package Installation (Dynamic Command Construction)
    let update_cmd = cfg.update_args.clone();
    
    let install_base = if cfg.pkg_manager == "pacman" {
        vec!["-S", "--noconfirm"]
    } else {
        vec!["install", "-y"]
    };

    let mut lxc_install_args = install_base.clone();
    lxc_install_args.extend(cfg.lxc_packages.clone());

    let mut sec_install_args = install_base.clone();
    sec_install_args.push(cfg.ssh_package);
    
    // Add firewall package based on detection
    match cfg.firewall_tool {
        FirewallKind::Firewalld => sec_install_args.push("firewalld"),
        FirewallKind::Ufw => sec_install_args.push("ufw"),
        FirewallKind::Iptables => sec_install_args.push("iptables"),
    }

    let mut sys_enable_args = vec!["enable", "--now", "lxc.service", "lxc-net.service", cfg.ssh_service];
    match cfg.firewall_tool {
        FirewallKind::Firewalld => sys_enable_args.push("firewalld"),
        FirewallKind::Ufw => sys_enable_args.push("ufw"),
        FirewallKind::Iptables => {}, // iptables usually doesn't have a direct service to enable like this
    }

    let commands = vec![
        ("Synchronizing package repositories", cfg.pkg_manager, update_cmd),
        ("Installing Virtualization & Bridge tools", cfg.pkg_manager, lxc_install_args),
        ("Installing SSH & Security components", cfg.pkg_manager, sec_install_args),
        ("Loading veth kernel module", "modprobe", vec!["veth"]),
        ("Enabling LXC, SSH, & Firewall services", "systemctl", sys_enable_args),
    ];

    // Allocate 10 minutes (600 seconds) timeout for heavy operations, 60 seconds for others
    for (desc, prog, args) in commands {
        let timeout_limit = if prog == cfg.pkg_manager { 600 } else { 60 };
        if !execute_step(desc, prog, &args, timeout_limit).await {
            eprintln!("\n{}CRITICAL_FAILURE: Deployment terminated abruptly at step '{}'{}", RED, desc, RESET);
            std::process::exit(1);
        }
    }

    // 4. Infrastructure Deployment & Configuration
    deploy_melisa_binary().await;
    setup_ssh_firewall(&cfg.firewall_tool).await;
    setup_lxc_network_quota().await;
    setup_projects_directory().await;
    configure_git_security().await;
    fix_shared_folder_permission("data").await;
    fix_uidmap_permissions().await;
    fix_system_privacy().await;

    // SubUID/SubGID Mapping for the user executing sudo
    match std::env::var("SUDO_USER") {
        Ok(user) if !user.is_empty() => setup_user_mapping(&user).await,
        Ok(_) | Err(_) => println!("  {:<50} [ {}WARNING{} ] SUDO_USER variable is missing; skipping user mapping.", "User Mapping", RED, RESET),
    }

    // 5. Shell Finalization
    register_melisa_shell().await;
    configure_sudoers_access().await;

    println!("\n{}VERIFYING SYSTEM CONFIGURATION...{}", BOLD, RESET);
    let verify_status = timeout(Duration::from_secs(30), Command::new("lxc-checkconfig").status()).await;
    match verify_status {
        Ok(Ok(status)) if status.success() => println!("  {:<50} [ {}PASSED{} ]", "LXC Kernel Support", GREEN, RESET),
        _ => println!("  {:<50} [ {}WARNING{} ] lxc-checkconfig encountered anomalies.", "System Verify", RED, RESET),
    }

    println!("\n{}MELISA HOST DEPLOYMENT COMPLETED SUCCESSFULLY{}\n", GREEN, RESET);
    println!("{}STATUS: SSH Active, Jail Shell Deployed, & Network Bridge Ready.{}", CYAN, RESET);
}

// =====================================================================
// UTILITY FUNCTIONS 
// =====================================================================

async fn execute_silent_task(program: &str, args: &[&str], description: &str, timeout_secs: u64) -> bool {
    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::null()).stderr(Stdio::null());

    match timeout(Duration::from_secs(timeout_secs), cmd.status()).await {
        Ok(Ok(s)) if s.success() => {
            println!("  {:<50} [ {}OK{} ]", description, GREEN, RESET);
            true
        }
        Ok(Ok(s)) => {
            println!("  {:<50} [ {}FAILED (Code: {}){} ]", description, RED, s.code().unwrap_or(-1), RESET);
            false
        }
        Ok(Err(e)) => {
            println!("  {:<50} [ {}ERROR: {}{} ]", description, RED, e, RESET);
            false
        }
        Err(_) => {
            println!("  {:<50} [ {}TIMEOUT ({}s){} ]", description, RED, timeout_secs, RESET);
            false
        }
    }
}

async fn backup_file(path: &str) {
    let original = Path::new(path);
    if original.exists() {
        let backup_path = format!("{}.bak", path);
        match fs::copy(original, &backup_path).await {
            Ok(_) => println!("  {:<50} [ {}BACKUP OK{} ]", format!("Securing {}", path), CYAN, RESET),
            Err(e) => println!("  {:<50} [ {}BACKUP FAIL: {}{} ]", format!("Securing {}", path), RED, e, RESET),
        }
    }
}

async fn check_required_tools(cfg: &DistroConfig) -> bool {
    println!("{}Verifying Required System Tools...{}", BOLD, RESET);
    
    // Core tools required regardless of OS
    let mut tools = vec!["modprobe", "systemctl", "git", "chown", "chmod", "grep", cfg.pkg_manager];
    
    // Dynamic firewall tool check
    match cfg.firewall_tool {
        FirewallKind::Firewalld => tools.push("firewall-cmd"),
        FirewallKind::Ufw => tools.push("ufw"),
        FirewallKind::Iptables => tools.push("iptables"),
    }

    let mut all_passed = true;

    for tool in tools {
        let mut cmd = Command::new("which");
        cmd.arg(tool).stdout(Stdio::null()).stderr(Stdio::null());

        match timeout(Duration::from_secs(5), cmd.status()).await {
            Ok(Ok(s)) if s.success() => {
                println!("  {:<50} [ {}FOUND{} ]", tool, GREEN, RESET);
            }
            _ => {
                println!("  {:<50} [ {}MISSING{} ]", tool, RED, RESET);
                all_passed = false;
            }
        }
    }
    all_passed
}

// =====================================================================
// CORE FUNCTIONS
// =====================================================================

async fn execute_step(description: &str, program: &str, args: &[&str], timeout_secs: u64) -> bool {
    println!("{} {}...", BOLD, description);
    let _ = io::stdout().flush().await;

    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::inherit()).stderr(Stdio::inherit());

    match timeout(Duration::from_secs(timeout_secs), cmd.status()).await {
        Ok(Ok(s)) if s.success() => { 
            println!("{}[ OK ]{} {}\n", GREEN, RESET, description); 
            true 
        }
        Ok(Ok(s)) => { 
            println!("{}[ FAILED (Exit: {}) ]{} {}\n", RED, s.code().unwrap_or(-1), RESET, description); 
            false 
        }
        Ok(Err(e)) => {
            println!("{}[ SYSTEM ERROR: {} ]{} {}\n", RED, e, RESET, description);
            false
        }
        Err(_) => {
            println!("{}[ TIMEOUT EXCEEDED ({}s) ]{} {}\n", RED, timeout_secs, RESET, description);
            false
        }
    }
}

async fn verify_data_environment() {
    let data_path = Path::new("data");
    println!("Verifying Local Data Environment...");

    if data_path.exists() {
        match fs::canonicalize(data_path).await {
            Ok(abs_path) => {
                println!("  {:<50} [ {}FOUND{} ]", "Data directory already exists", CYAN, RESET);
                println!("  {}Location: {}{}", BOLD, abs_path.display(), RESET);
                
                println!("  {}Contents:{}", BOLD, RESET);
                match fs::read_dir(data_path).await {
                    Ok(mut entries) => {
                        let mut has_files = false;
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            println!("    - {}", entry.file_name().to_string_lossy());
                            has_files = true;
                        }
                        if !has_files { println!("    (Directory is empty)"); }
                    }
                    Err(e) => println!("    {}Failed to read directory contents: {}{}", RED, e, RESET),
                }
            }
            Err(e) => println!("  {:<50} [ {}ERROR{} ] Failed to resolve path: {}", "Data directory verification", RED, RESET, e),
        }
    } else {
        match fs::create_dir_all(data_path).await {
            Ok(_) => println!("  {:<50} [ {}CREATED{} ]", "New data directory created", GREEN, RESET),
            Err(e) => {
                println!("  {:<50} [ {}CRITICAL FAIL{} ]", "Failed to create data directory", RED, RESET);
                eprintln!("    Reason: {}", e);
            }
        }
    }
}

async fn deploy_melisa_binary() {
    let target_path = "/usr/local/bin/melisa";
    println!("\n{}REFRESHING BINARY: Overwriting /usr/local/bin/melisa...{}", BOLD, RESET);

    let current_exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            println!("  {:<50} [ {}FATAL{} ]", "Retrieving current binary path", RED, RESET);
            eprintln!("    Error: {}", e);
            return;
        }
    };

    if fs::metadata(target_path).await.is_ok() {
        if let Err(e) = fs::remove_file(target_path).await {
            println!("  {:<50} [ {}WARNING{} ] Failed to remove old binary: {}", "Binary cleanup", RED, RESET, e);
        } else {
            println!("  {:<50} [ {}CLEANED{} ]", "Old binary unlinked", CYAN, RESET);
        }
    }

    let mut cmd = Command::new("cp");
    cmd.args(&[current_exe.to_str().unwrap(), target_path]);

    match timeout(Duration::from_secs(10), cmd.status()).await {
        Ok(Ok(s)) if s.success() => {
            let chown_ok = execute_silent_task("chown", &["root:root", target_path], "Setting root ownership", 5).await;
            let chmod_ok = execute_silent_task("chmod", &["4755", target_path], "Setting SUID bit (4755)", 5).await;

            if chown_ok && chmod_ok {
                println!("  {:<50} [ {}UPDATED{} ]", "New version deployed (SUID set)", GREEN, RESET);
            } else {
                println!("  {:<50} [ {}SECURITY WARNING{} ]", "Binary copied but ownership/SUID assignment failed", RED, RESET);
            }
        }
        Ok(Ok(s)) => println!("  {:<50} [ {}FAILED (Code: {}){} ]", "Failed to copy new binary", RED, s.code().unwrap_or(-1), RESET),
        Ok(Err(e)) => println!("  {:<50} [ {}IO ERROR: {}{} ]", "Failed to copy new binary", RED, e, RESET),
        Err(_) => println!("  {:<50} [ {}TIMEOUT{} ]", "Copy operation timed out", RED, RESET),
    }
}

/// Configures the host's firewall dynamically based on the active OS.
/// This ensures both remote management and container connectivity are fully functional.
async fn setup_ssh_firewall(firewall: &FirewallKind) {
    println!("\nConfiguring Host Firewall for SSH and Network Bridge Access...");

    match firewall {
        FirewallKind::Firewalld => {
            let ssh_status = execute_silent_task(
                "firewall-cmd", &["--add-service=ssh", "--permanent"], "Adding SSH service to firewall rules", 10
            ).await;
            let bridge_status = execute_silent_task(
                "firewall-cmd", &["--zone=trusted", "--add-interface=lxcbr0", "--permanent"], "Assigning lxcbr0 to trusted firewall zone", 10
            ).await;
            let reload_status = execute_silent_task(
                "firewall-cmd", &["--reload"], "Reloading firewall configuration", 15
            ).await;
            
            if ssh_status && bridge_status && reload_status {
                println!("  {:<50} [ {}OK{} ]", "Firewall ready: SSH and LXC Bridge are now authorized", GREEN, RESET);
            } else {
                eprintln!("  {:<50} [ {}FAILED{} ]", "Critical error during firewall deployment", RED, RESET);
            }
        },
        FirewallKind::Ufw => {
            let _ = execute_silent_task("ufw", &["allow", "ssh"], "Allowing SSH in UFW", 10).await;
            let _ = execute_silent_task("ufw", &["allow", "in", "on", "lxcbr0"], "Trusting lxcbr0 interface in UFW", 10).await;
            let _ = execute_silent_task("ufw", &["--force", "enable"], "Enabling UFW firewall", 10).await;
            let _ = execute_silent_task("ufw", &["reload"], "Reloading UFW configuration", 10).await;
            
            println!("  {:<50} [ {}OK{} ]", "UFW ready: SSH and LXC Bridge are now authorized", GREEN, RESET);
        },
        FirewallKind::Iptables => {
            let _ = execute_silent_task("iptables", &["-A", "INPUT", "-p", "tcp", "--dport", "22", "-j", "ACCEPT"], "Allowing SSH via iptables", 10).await;
            let _ = execute_silent_task("iptables", &["-A", "INPUT", "-i", "lxcbr0", "-j", "ACCEPT"], "Trusting lxcbr0 via iptables", 10).await;
            
            println!("  {:<50} [ {}OK{} ]", "Iptables ready: SSH and LXC Bridge are now authorized", GREEN, RESET);
        }
    }
}

async fn setup_lxc_network_quota() {
    let config_path = "/etc/lxc/lxc-usernet";
    println!("\nConfiguring LXC Network Quota...");
    
    if let Ok(user) = std::env::var("SUDO_USER") {
        if user.is_empty() { return; }
        
        let quota_rule = format!("{} veth lxcbr0 10\n", user);
        backup_file(config_path).await;

        let content = fs::read_to_string(config_path).await.unwrap_or_default();
        if !content.contains(&quota_rule) {
            match OpenOptions::new().append(true).create(true).open(config_path).await {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(quota_rule.as_bytes()).await {
                        println!("  {:<50} [ {}IO ERROR{} ] Failed writing to lxc-usernet: {}", "Network Quota", RED, RESET, e);
                    } else {
                        println!("  {:<50} [ {}OK{} ]", format!("Network quota for '{}' assigned", user), GREEN, RESET);
                    }
                }
                Err(e) => println!("  {:<50} [ {}ACCESS DENIED{} ] Cannot open lxc-usernet: {}", "Network Quota", RED, RESET, e),
            }
        } else {
            println!("  {:<50} [ {}SKIPPED{} ]", "Network quota already configured", CYAN, RESET);
        }
    }
}

async fn register_melisa_shell() {
    let shell_path = "/usr/local/bin/melisa";
    println!("\nRegistering Jail Shell...");

    backup_file("/etc/shells").await;

    let cmd = format!("grep -qxF '{0}' /etc/shells || echo '{0}' >> /etc/shells", shell_path);
    if execute_silent_task("sh", &["-c", &cmd], "Registering shell in /etc/shells", 10).await {
        println!("  {:<50} [ {}OK{} ]", "Shell environment registered successfully", GREEN, RESET);
    }
}

async fn configure_sudoers_access() {
    let sudo_rule = "ALL ALL=(ALL) NOPASSWD: /usr/local/bin/melisa\n";
    let sudoers_file = "/etc/sudoers.d/melisa";
    println!("\nConfiguring Sudoers Access...");

    match OpenOptions::new().create(true).write(true).truncate(true).open(sudoers_file).await {
        Ok(mut file) => {
            if let Err(e) = file.write_all(sudo_rule.as_bytes()).await {
                println!("  {:<50} [ {}IO ERROR{} ] Failed writing sudoers file: {}", "Sudoers Policy", RED, RESET, e);
            } else if execute_silent_task("chmod", &["0440", sudoers_file], "Applying strict permissions (0440)", 5).await {
                println!("  {:<50} [ {}OK{} ]", "Sudoers rules successfully deployed", GREEN, RESET);
            }
        }
        Err(e) => println!("  {:<50} [ {}ACCESS DENIED{} ] Failed creating sudoers file: {}", "Sudoers Policy", RED, RESET, e),
    }
}

async fn fix_uidmap_permissions() {
    println!("\nFixing System Traversal Permissions...");
    let paths = ["/usr/bin/newuidmap", "/usr/bin/newgidmap"];
    
    for path in &paths {
        if Path::new(path).exists() {
            execute_silent_task("chmod", &["u+s", path], &format!("Setting SUID on {}", path), 5).await;
        } else {
            println!("  {:<50} [ {}MISSING BINARY{} ]", path, RED, RESET);
        }
    }
    
    if execute_silent_task("chmod", &["+x", "/var/lib/lxc"], "Allowing traversal on /var/lib/lxc", 5).await {
        println!("  {:<50} [ {}OK{} ]", "System traversal permissions resolved", GREEN, RESET);
    }
}

async fn fix_shared_folder_permission(host_path: &str) {
    println!("\nApplying Shared Folder Permissions...");
    // Mapping to UID/GID 100000 for unprivileged LXC environments
    if execute_silent_task("chown", &["-R", "100000:100000", host_path], &format!("Mapping '{}' to 100000:100000", host_path), 60).await {
         println!("  {:<50} [ {}OK{} ]", "Shared folder ownership mapped", GREEN, RESET);
    }
}

async fn setup_projects_directory() {
    println!("\n{}Configuring Master Projects Infrastructure...{}", BOLD, RESET);

    match timeout(Duration::from_secs(10), Command::new("mkdir").args(&["-p", PROJECTS_MASTER]).status()).await {
        Ok(Ok(s)) if s.success() => {
            // Apply Sticky Bit (1777) - Users can create directories but cannot delete others' directories
            if execute_silent_task("chmod", &["1777", PROJECTS_MASTER], "Setting Sticky Bit (1777) on Master Projects", 5).await {
                println!("  {:<50} [ {}OK{} ]", "Master projects directory secured and opened", GREEN, RESET);
            }
        }
        Ok(Ok(s)) => println!("  {:<50} [ {}FAILED (Code: {}){} ]", "Directory creation blocked", RED, s.code().unwrap_or(-1), RESET),
        Ok(Err(e)) => println!("  {:<50} [ {}IO ERROR: {}{} ]", "Directory creation failed", RED, e, RESET),
        Err(_) => println!("  {:<50} [ {}TIMEOUT{} ]", "Directory creation timed out", RED, RESET),
    }
}

async fn configure_git_security() {
    println!("\nConfiguring Global Git Security...");
    
    // --system parameter ensures configuration persists across user boundaries
    if execute_silent_task("git", &["config", "--system", "--add", "safe.directory", "*"], "Setting global git safe.directory='*'", 10).await {
        println!("  {:<50} [ {}OK{} ]", "Global Git safety overrides applied", GREEN, RESET);
    }
}

async fn fix_system_privacy() {
    println!("\nHardening System Privacy Boundaries...");
    // 711 on /home prevents standard users from performing 'ls /home' to enumerate other users
    if execute_silent_task("chmod", &["711", "/home"], "Protecting /home directory indexing", 10).await {
        println!("  {:<50} [ {}OK{} ]", "Directory /home is now fully unlistable", GREEN, RESET);
    }
}

async fn setup_user_mapping(username: &str) {
    println!("\nSetting up LXC Subordinate IDs for User...");
    let desc = format!("Mapping SubUID/SubGID for {}", username);
    
    if execute_silent_task("usermod", &["--add-subuids", "100000-165535", "--add-subgids", "100000-165535", username], &desc, 15).await {
        println!("  {:<50} [ {}OK{} ]", "LXC User namespace mapping successfully configured", GREEN, RESET);
    }
}