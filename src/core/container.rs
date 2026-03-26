use tokio::process::Command;
use std::process::Stdio;
// Upgraded to tokio::fs for non-blocking asynchronous file operations
use tokio::fs::{self, OpenOptions}; 
use tokio::io::AsyncWriteExt; 
use std::path::Path;
use tokio::time::{sleep, Duration}; 
use std::path::PathBuf;

use crate::core::root_check::ensure_admin;
use crate::cli::color_text::{BOLD, GREEN, RED, RESET, YELLOW}; 

use crate::core::metadata::inject_distro_metadata;
use tracing::error; // Tambahkan 'error' di sini

use indicatif::ProgressBar;

// Import modul host_distro untuk perbaikan firewall dinamis
use crate::distros::host_distro::{detect_host_distro, get_distro_config, FirewallKind};

pub const LXC_PATH: &str = "/var/lib/lxc"; 

#[derive(Debug, Clone)]
pub struct DistroMetadata {
    pub slug: String,       
    pub name: String,       
    pub release: String,    
    pub arch: String,       
    #[allow(dead_code)]
    pub variant: String,    
    pub pkg_manager: String 
}

/// Creates a new LXC container using the download template.
/// Handles GPG errors, existing containers, and auto-initializes the network.
pub async fn create_new_container(name: &str, meta: DistroMetadata, pb: ProgressBar)  {
    // [STEP 0] PRE-FLIGHT: Verify host runtime environment (lxcbr0, etc.)
    if !verify_host_runtime().await {
        eprintln!("{}[ERROR]{} Host network bridge is down and auto-repair failed.{}", RED, BOLD, RESET);
        eprintln!("{}Tip:{} Run 'melisa --setup' to initialize host infrastructure.", YELLOW, RESET);
        return; 
    }

    pb.println(format!("{}--- Creating Container: {} ({}) ---{}", BOLD, name, meta.slug, RESET));
    
    // Execute the lxc-create command to pull and build the rootfs
    let process = Command::new("sudo")
        .args(&[
            "-n", "lxc-create", "-P", LXC_PATH, "-t", "download", "-n", name, 
            "--", "-d", &meta.name, "-r", &meta.release, "-a", &meta.arch
        ])
        .output()
        .await; 

    match process {
        Ok(output) => {
            if output.status.success() {
                pb.println(format!("{}[SUCCESS]{} Container successfully created.", GREEN, RESET));

                // Inject metadata distro in container system file
                if let Err(e) = inject_distro_metadata(LXC_PATH, name, &meta).await {
                    // Kita cetak peringatan tapi tidak menghentikan proses (non-fatal)
                    error!("FATAL: Metadata injection failed: {}", e);
                }
                
                // Inject basic network configurations to ensure internet access
                inject_network_config(name).await;
                
                // Inject and lock DNS to prevent systemd-resolved/netconfig from overwriting it
                setup_container_dns(name).await; 

                // Start the container for further internal setups
                pb.println(format!("{}[INFO]{} Starting container for initial setup...", YELLOW, RESET));
                start_container(name).await; 

                // DYNAMIC WAIT: Actively poll for network readiness instead of blindly sleeping
                if wait_for_network_initialization(name).await {
                    // Execute the package manager update inside the container securely
                    auto_initial_setup(name, &meta.pkg_manager).await; 
                } else {
                    pb.println(format!("{}[ERROR]{} Network DHCP initialization timed out. Skipping package manager setup.", RED, RESET));
                }
                
                pb.println(format!("{}[SUCCESS]{} Container successfully provisioned!", GREEN, RESET));
                
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                
                // Parse specific LXC errors to provide user-friendly feedback
                if error_msg.contains("already exists") {
                    pb.println(format!("{}[WARNING]{} Container '{}' already exists. Skipping creation process.", YELLOW, RESET, name));
                } else if error_msg.contains("GPG") {
                    pb.println(format!("{}[ERROR]{} GPG signature verification failed. Try running 'gpg --recv-keys' on the host system.", RED, RESET));
                } else if error_msg.contains("download") {
                    pb.println(format!("{}[ERROR]{} Failed to download template. Please verify the host's internet connection.", RED, RESET));
                } else {
                    pb.println(format!("{}[ERROR]{} Failed to create container: {}", RED, RESET, name));
                    pb.println(format!("Error Details: {}", error_msg));
                }
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Could not execute lxc-create command: {}", RED, RESET, e),
    }
}

/// Performs a lightweight pre-flight check on the host system's networking.
/// If the required bridge is missing, it attempts an automatic repair.
async fn verify_host_runtime() -> bool {
    // Checking /sys/class/net is extremely fast as it avoids spawning a sub-process.
    if Path::new("/sys/class/net/lxcbr0").exists() {
        return true; // Bridge is active; proceed with container operations.
    }

    // If the bridge is missing, notify the user and trigger the repair sequence.
    println!("{}[WARNING]{} Network bridge 'lxcbr0' not found. Initiating host auto-repair...", YELLOW, RESET);
    
    // Call the repair function to start services and set firewall rules
    ensure_host_network_ready().await; 

    // Final check to see if the repair was successful
    Path::new("/sys/class/net/lxcbr0").exists()
}

/// Dynamically polls LXC to check if the container has successfully acquired an IP address.
/// This prevents race conditions where the package manager runs before DNS is ready.
async fn wait_for_network_initialization(name: &str) -> bool {
    println!("{}[INFO]{} Waiting for DHCP lease and network interfaces to initialize...", YELLOW, RESET);
    
    let max_retries = 30; // Maximum wait time of 30 seconds
    
    for _ in 0..max_retries {
        // Query LXC for the container's IP addresses using non-interactive sudo
        let output = Command::new("sudo")
            .args(&["-n", "lxc-info", "-n", name, "-iH"])
            .output()
            .await;

        if let Ok(out) = output {
            let ips = String::from_utf8_lossy(&out.stdout);
            
            // Check if standard IPv4 formatting exists in the output
            if ips.contains(".") && !ips.trim().is_empty() {
                println!("{}[INFO]{} Network connection established. Allowing DNS resolver to settle...", YELLOW, RESET);
                // Buffer time: Give systemd-resolved / netconfig a brief moment to initialize
                sleep(Duration::from_secs(3)).await; 
                return true;
            }
        }
        
        // Wait 1 second before querying again
        sleep(Duration::from_secs(1)).await;
    }
    
    false
}

/// Automatically updates the package repository of the newly created container
/// based on its specific package manager (apt, dnf, apk, etc.)
async fn auto_initial_setup(name: &str, pkg_manager: &str) {
    let cmd = match pkg_manager {
        "apt"    => "apt-get update -y", 
        "dnf"    => "dnf makecache",
        "apk"    => "apk update",
        "pacman" => "pacman -Sy --noconfirm",
        "zypper" => "zypper --non-interactive refresh",
        _        => "true", // Fallback to do nothing securely
    };
    
    println!("{}[INFO]{} Updating package repository for '{}'...", YELLOW, RESET, name);

    let output = Command::new("sudo")
        .args(&["-n", "lxc-attach", "-n", name, "--", "sh", "-c", cmd])
        .output()
        .await; 

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("{}[SUCCESS]{} Initial repository setup completed for {}.", GREEN, RESET, name);
            } else {
                eprintln!("{}[ERROR]{} Failed to execute initial repository setup on {}.", RED, RESET, name);
                eprintln!("[DEBUG] Package Manager Error: {}", String::from_utf8_lossy(&out.stderr));
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Failed to spawn lxc-attach process: {}", RED, RESET, e),
    }
}

/// Injects a veth bridge network configuration into the container's config file.
/// Ensures the container is connected to lxcbr0 with a random MAC address.
async fn inject_network_config(name: &str) {
    let config_path = format!("{}/{}/config", LXC_PATH, name); 
    
    if Path::new(&config_path).exists() {
        // Read existing configuration using non-blocking async I/O
        let content = fs::read_to_string(&config_path).await.unwrap_or_default();
        
        // Prevent duplicate network configuration injections
        if content.contains("lxc.net.0.link") {
            println!("{}[SKIP]{} Network configuration already exists. Skipping injection.", YELLOW, RESET);
            return;
        }

        // Use tokio::fs::OpenOptions for fully asynchronous file operations
        match OpenOptions::new().append(true).open(&config_path).await {
            Ok(mut file) => {
                let net_config = format!(
                    "\n# Auto-generated by MELISA\n\
                    lxc.net.0.type = veth\n\
                    lxc.net.0.link = lxcbr0\n\
                    lxc.net.0.flags = up\n\
                    lxc.net.0.hwaddr = ee:ec:fa:5e:{:02x}:{:02x}\n", 
                    rand::random::<u8>(), rand::random::<u8>() 
                );

                if let Err(e) = file.write_all(net_config.as_bytes()).await {
                    eprintln!("{}[ERROR]{} Failed to write async network config: {}", RED, RESET, e);
                }
            }
            Err(e) => eprintln!("{}[ERROR]{} Failed to open container configuration file asynchronously: {}", RED, RESET, e),
        }
    }
}

/// Injects a static DNS configuration (Google DNS) into the container's rootfs
/// and applies an immutable lock to prevent overwrites by network managers.
async fn setup_container_dns(name: &str) {
    let etc_path = format!("{}/{}/rootfs/etc", LXC_PATH, name);
    let dns_path = format!("{}/resolv.conf", etc_path);
    
    // 1. Ensure the target configuration directory exists
    let _ = Command::new("sudo")
        .args(&["mkdir", "-p", &etc_path])
        .status()
        .await;

    // 2. Remove existing resolv.conf or symlinks to avoid conflicts
    let _ = Command::new("sudo")
        .args(&["rm", "-f", &dns_path])
        .status()
        .await;
    
    // 3. Write static DNS entries using shell redirection
    let dns_content = "nameserver 8.8.8.8\\nnameserver 8.8.4.4\\n";
    let write_status = Command::new("sudo")
        .args(&["bash", "-c", &format!("echo -e '{}' > {}", dns_content, dns_path)])
        .status()
        .await;

    match write_status {
        Ok(s) if s.success() => {
            // 4. Set the immutable attribute to prevent DHCP/NetworkManager from altering the file
            let lock_status = Command::new("sudo")
                .args(&["chattr", "+i", &dns_path])
                .status()
                .await;
                
            if let Ok(ls) = lock_status {
                if ls.success() {
                    println!("{}[INFO]{} DNS configured and locked successfully.", GREEN, RESET);
                } else {
                    println!("{}[WARNING]{} DNS written, but failed to apply immutable lock (chattr).", YELLOW, RESET);
                }
            }
        },
        _ => eprintln!("{}[ERROR]{} Failed to configure DNS.", RED, RESET),
    }
}

/// Helper function to unlock the DNS file later if needed
#[allow(dead_code)]
async fn unlock_container_dns(name: &str) {
    let dns_path = format!("{}/{}/rootfs/etc/resolv.conf", LXC_PATH, name);
    let _ = Command::new("sudo")
        .args(&["-n", "chattr", "-i", &dns_path])
        .status()
        .await;
}

/// Ensures the host system's LXC bridge network and firewall are active
/// Upgraded to dynamically detect and configure the host's firewall rules
pub async fn ensure_host_network_ready() {
    println!("{}[INFO]{} Re-initializing Host Network Infrastructure...", BOLD, RESET);

    // Start lxc-net service silently using non-interactive sudo
    let _ = Command::new("sudo")
        .args(&["-n", "systemctl", "start", "lxc-net"])
        .status()
        .await; 

    // Retrieve active host OS configuration to apply the correct firewall rules
    let distro = detect_host_distro().await;
    let cfg = get_distro_config(&distro);

    match cfg.firewall_tool {
        FirewallKind::Firewalld => {
            let _ = Command::new("sudo")
                .args(&["-n", "firewall-cmd", "--zone=trusted", "--add-interface=lxcbr0", "--permanent"])
                .status()
                .await; 
            let _ = Command::new("sudo")
                .args(&["-n", "firewall-cmd", "--reload"])
                .status()
                .await; 
        },
        FirewallKind::Ufw => {
            let _ = Command::new("sudo")
                .args(&["-n", "ufw", "allow", "in", "on", "lxcbr0"])
                .status()
                .await;
            let _ = Command::new("sudo")
                .args(&["-n", "ufw", "reload"])
                .status()
                .await;
        },
        FirewallKind::Iptables => {
            let _ = Command::new("sudo")
                .args(&["-n", "iptables", "-I", "INPUT", "-i", "lxcbr0", "-j", "ACCEPT"])
                .status()
                .await;
        }
    }
}

/// Helper function to check if a specific container is currently running
async fn is_container_running(name: &str) -> bool {
    let output = Command::new("sudo")
        .args(&["-n", "lxc-info", "-P", LXC_PATH, "-n", name, "-s"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let status_str = String::from_utf8_lossy(&out.stdout);
            status_str.contains("RUNNING")
        },
        _ => false,
    }
}

/// Menghapus file metadata MELISA (info & tmp) secara eksplisit
async fn cleanup_metadata(name: &str) {
    let rootfs_path = PathBuf::from(LXC_PATH).join(name).join("rootfs");
    let target_path = rootfs_path.join("etc").join("melisa-info");
    let temp_path = rootfs_path.join("etc").join("melisa-info.tmp");

    // Hapus file info utama
    if tokio::fs::try_exists(&target_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&target_path).await;
    }

    // Hapus file temporary jika masih ada (bekas crash atau proses gagal)
    if tokio::fs::try_exists(&temp_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_file(&temp_path).await;
    }
}

/// Gracefully stops and destroys a container.
/// It automatically handles running containers and unlocks restricted files before deletion.
pub async fn delete_container(name: &str, pb: ProgressBar) {
    pb.println(format!("{}--- Processing Deletion: {} ---{}", BOLD, name, RESET));

    // 1. PRE-CHECK: If the container is running, we MUST stop it first
    if is_container_running(name).await {
        println!("{}[INFO]{} Container '{}' is currently running.", YELLOW, RESET, name);
        println!("{}[INFO]{} Initiating graceful shutdown before deletion...", YELLOW, RESET);
        
        // Call the actual stop function
        stop_container(name).await;

        // Verify if it actually stopped. If it fails to stop, we cannot proceed.
        if is_container_running(name).await {
            eprintln!("{}[ERROR]{} Failed to stop container '{}'. Deletion aborted to prevent data corruption.", RED, RESET, name);
            return;
        }
    }

    // 2. PREPARATION: Unlock DNS configuration
    // LXC cannot delete the rootfs if 'resolv.conf' is still locked with 'chattr +i'
    pb.println(format!("{}[INFO]{} Unlocking system configurations for {}...", BOLD, RESET, name));
    unlock_container_dns(name).await;

    // --- STEP TAMBAHAN: MELISA METADATA CLEANUP ---
    // Kita bersihkan metadata MELISA sebelum folder rootfs dihancurkan total
    println!("{}[INFO]{} Purging MELISA engine metadata for {}...", BOLD, RESET, name);
    cleanup_metadata(name).await;
    // ----------------------------------------------

    // 3. EXECUTION: Destroy the container
    // We use '-f' (force) as a secondary safety measure
    let status = Command::new("sudo")
        .args(&["-n", "lxc-destroy", "-P", LXC_PATH, "-n", name, "-f"])
        .status() 
        .await;

    match status {
        Ok(s) if s.success() => {
            println!("{}[SUCCESS]{} Container '{}' has been permanently destroyed.", GREEN, RESET, name);
        },
        Ok(s) => {
            eprintln!("{}[ERROR]{} Deletion failed with exit code: {}.", RED, RESET, s.code().unwrap_or(-1));
            eprintln!("{}[TIP]{} Ensure you have sudo permissions or check 'lxc-ls' for container status.", YELLOW, RESET);
        },
        Err(e) => eprintln!("{}[FATAL]{} Could not execute lxc-destroy: {}", RED, RESET, e),
    }
}

/// Boots up a container in daemon (-d) mode
pub async fn start_container(name: &str) {
    println!("{}[INFO]{} Starting container '{}'...", GREEN, RESET, name);
    
    let status = Command::new("sudo")
        .args(&["lxc-start", "-P", LXC_PATH, "-n", name, "-d"]) 
        .status()
        .await; 

    match status {
        Ok(s) if s.success() => println!("{}[SUCCESS]{} Container is now running.", GREEN, RESET),
        _ => eprintln!("{}[ERROR]{} Failed to start container. Check if it exists and is configured properly.", RED, RESET),
    }
}

/// Attaches the host terminal directly into the container's bash session
pub async fn attach_to_container(name: &str) {
    println!("{}[MODE]{} Entering Saferoom: {}. Type 'exit' to return to Host.", BOLD, name, RESET);

    let _ = Command::new("sudo")
        .args(&["lxc-attach", "-P", LXC_PATH, "-n", name, "--", "bash"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .await; 
}

/// Gracefully powers down a running container
pub async fn stop_container(name: &str) {
    if !ensure_admin().await { return; } 
    println!("{}[SHUTDOWN]{} Initiating shutdown for container '{}'...", YELLOW, RESET, name);

    let process = Command::new("sudo")
        .args(&["lxc-stop", "-P", LXC_PATH, "-n", name])
        .output()
        .await; 

    match process {
        Ok(output) => {
            if output.status.success() {
                println!("{}[SUCCESS]{} Container '{}' has been successfully stopped.", GREEN, RESET, name);
            } else {
                eprintln!("{}[ERROR]{} Failed to stop container.", RED, RESET);
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Execution Error: {}", RED, RESET, e),
    }
}

/// Sends a direct execution command to a running container from the host
pub async fn send_command(name: &str, command_args: &[&str]) {
    if command_args.is_empty() {
        eprintln!("{}[ERROR]{} No command payload provided.", RED, RESET);
        return;
    }

    // 1. PRE-FLIGHT CHECK: Ensure the target container is running
    let check_status = Command::new("sudo")
        .args(&["/usr/bin/lxc-info", "-P", LXC_PATH, "-n", name, "-s"])
        .output()
        .await; 

    if let Ok(out) = check_status {
        let output_str = String::from_utf8_lossy(&out.stdout);
        if !output_str.contains("RUNNING") {
            println!("{}[ERROR]{} Container '{}' is NOT running.", RED, RESET, name);
            println!("{}Tip:{} Execute 'melisa --run {}' to start it first.", YELLOW, RESET, name);
            return; // Abort execution safely
        }
    } else {
        eprintln!("{}[ERROR]{} Failed to retrieve container status.", RED, RESET);
        return;
    }

    // 2. EXECUTION: Pass the command to lxc-attach
    println!("{}[SEND]{} Executing payload on '{}'...", BOLD, name, RESET);

    let status = Command::new("sudo")
        .arg("lxc-attach")
        .arg("-P")
        .arg(LXC_PATH)
        .arg("-n")
        .arg(name)
        .arg("--")
        .args(command_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await; 

    // 3. VERIFICATION
    match status {
        Ok(s) if s.success() => println!("\n{}[DONE]{} Command executed successfully within container.", GREEN, RESET),
        _ => eprintln!("\n{}[ERROR]{} Command inside container returned a non-zero exit code.", RED, RESET),
    }
}

/// Mounts a directory from the Host system to the Container via bind mount
pub async fn add_shared_folder(name: &str, host_path: &str, container_path: &str) {
    let config_path = format!("{}/{}/config", LXC_PATH, name);
    
    // 1. Safely resolve absolute path, preventing crashes if the path is invalid
    let abs_host_path = match fs::canonicalize(host_path).await {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}[ERROR]{} Invalid or missing host directory path: {}", RED, RESET, e);
            return;
        }
    };

    if Path::new(&config_path).exists() {
        // 2. Check for duplication before appending
        let content = fs::read_to_string(&config_path).await.unwrap_or_default();
        let mount_entry = format!("lxc.mount.entry = {} {}", abs_host_path.display(), container_path);
        
        if content.contains(&mount_entry) {
            println!("{}[SKIP]{} This directory is already mapped in the configuration.", YELLOW, RESET);
            return;
        }

        // 3. Append to the config file gracefully
        match OpenOptions::new().append(true).open(&config_path).await {
            Ok(mut file) => {
                let mount_config = format!(
                    "\n# Shared Folder mapped by MELISA\n\
                    lxc.mount.entry = {} {} none bind,create=dir 0 0\n", 
                    abs_host_path.display(), container_path
                );

                match file.write_all(mount_config.as_bytes()).await {
                    Ok(_) => {
                        println!("{}[SUCCESS]{} Shared folder integrated to {}.", GREEN, RESET, name);
                        println!("{}[IMPORTANT]{} Please run 'melisa --stop {}' and 'melisa --run {}' to apply changes.", YELLOW, RESET, name, name);
                    },
                    Err(e) => eprintln!("{}[ERROR]{} Failed to write mount configuration: {}", RED, RESET, e),
                }
            }
            Err(e) => eprintln!("{}[ERROR]{} Failed to open container configuration: {}", RED, RESET, e),
        }
    } else {
        eprintln!("{}[ERROR]{} Configuration file for container '{}' not found.", RED, RESET, name);
    }
}

/// Removes a previously mounted shared folder from the container's configuration
pub async fn remove_shared_folder(name: &str, host_path: &str, container_path: &str) {
    let config_path = format!("{}/{}/config", LXC_PATH, name);
    
    // 1. Standardize path to match the exact string in the config file
    let abs_host_path = match fs::canonicalize(host_path).await {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{}[ERROR]{} Host path not found or invalid: {}", RED, RESET, e);
            return;
        }
    };
    let host_path_str = abs_host_path.to_string_lossy();

    if Path::new(&config_path).exists() {
        let content = match fs::read_to_string(&config_path).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}[ERROR]{} Failed to read container configuration: {}", RED, RESET, e);
                return;
            }
        };

        let target_entry = format!("lxc.mount.entry = {} {}", host_path_str, container_path);
        let comment_tag = "# Shared Folder mapped by MELISA";

        // 2. Filter out the specific mount entry and its associated comment
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut removed = false;

        let mut i = 0;
        while i < lines.len() {
            if lines[i].contains(&target_entry) {
                // Remove the MELISA comment tag if it directly precedes the mount entry
                if !new_lines.is_empty() && new_lines.last() == Some(&comment_tag) {
                    new_lines.pop();
                }
                removed = true;
                i += 1;
                continue;
            }
            new_lines.push(lines[i]);
            i += 1;
        }

        if !removed {
            println!("{}[SKIP]{} Shared folder mapping was not found in the configuration.", YELLOW, RESET);
            return;
        }

        // 3. Rewrite the configuration file with the target lines removed
        let new_content = new_lines.join("\n");
        match fs::write(&config_path, new_content).await {
            Ok(_) => {
                println!("{}[SUCCESS]{} Shared folder successfully unmapped from {}.", GREEN, RESET, name);
                println!("{}[IMPORTANT]{} Please restart the container to apply changes.", YELLOW, RESET);
            },
            Err(e) => eprintln!("{}[ERROR]{} Failed to update configuration file: {}", RED, RESET, e),
        }
    } else {
        eprintln!("{}[ERROR]{} Container configuration file not found.", RED, RESET);
    }
}

/// Securely pipes a tarball from standard input directly into the container's filesystem
pub async fn upload_to_container(name: &str, dest_path: &str) {
    let extract_cmd = format!("mkdir -p {} && tar -xzf - -C {}", dest_path, dest_path);
    
    let status = Command::new("sudo")
        .args(&["lxc-attach", "-P", LXC_PATH, "-n", name, "--", "bash", "-c", &extract_cmd])
        .stdin(Stdio::inherit())  // Accepts the incoming tarball stream from the host
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await; 

    match status {
        Ok(s) if s.success() => println!("{}[SUCCESS]{} Upload and extraction to '{}' completed successfully.", GREEN, RESET, dest_path),
        _ => eprintln!("{}[ERROR]{} Failed to extract data stream inside the container.", RED, RESET),
    }
}

/// Displays a list of existing containers using lxc-ls
pub async fn list_containers(only_active: bool) {
    println!("{}[INFO]{} Retrieving container inventory...", GREEN, RESET);
    
    let mut cmd = Command::new("sudo");
    cmd.args(&["lxc-ls", "-P", LXC_PATH, "--fancy"]);

    if only_active {
        cmd.arg("--active");
    }

    let output = cmd.output().await; 

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                eprintln!("{}[ERROR]{} Failed to retrieve container list.", RED, RESET);
            }
        }
        Err(e) => eprintln!("{}[FATAL]{} System Error: {}", RED, RESET, e),
    }
}