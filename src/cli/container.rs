use std::process::{Command, Stdio};
use std::env; // Tambahkan ini di bagian paling atas
use std::io::{self, Write};
use crate::cli::color_text::{BOLD, GREEN, RED, RESET, YELLOW}; // Tambah YELLOW biar lebih pas buat warning

// 1. Definisikan Enum untuk Role
pub enum UserRole {
    Admin,
    Regular,
}

const LXC_PATH: &str = "/var/lib/lxc";

pub fn create_new_container(name: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Creating Container: {} ---{}", BOLD, name, RESET);

    let process = Command::new("sudo") // Tambahkan sudo
        .args(&[
            "lxc-create", 
            "-P", LXC_PATH, // Paksa ke path sistem
            "-t", "download", 
            "-n", name, 
            "--", 
            "-d", "debian", 
            "-r", "bookworm", 
            "-a", "amd64"
        ])
        .output();

    match process {
        Ok(output) => {
            if output.status.success() {
                println!("{}[SUCCESS]{} Container '{}' created successfully.", GREEN, RESET, name);
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                if error_msg.contains("already exists") {
                    println!("{}[WARNING]{} Container '{}' already exists.", YELLOW, RESET, name);
                } else {
                    eprintln!("{}[ERROR]{} Failed to create container.{}", RED, RESET, RESET);
                    eprintln!("Details: {}", error_msg);
                }
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Could not run lxc-create: {}", RED, RESET, e),
    }
}

pub fn delete_container(name: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Deleting Container: {} ---{}", BOLD, name, RESET);

    let process = Command::new("sudo")
        .args(&["lxc-destroy", "-P", LXC_PATH, "-f", "-n", name])
        .output();

    match process {
        Ok(output) => {
            if output.status.success() {
                println!("{}[SUCCESS]{} Container '{}' deleted.", GREEN, RESET, name);
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                eprintln!("{}[ERROR]{} Failed to delete container: {}", RED, RESET, error_msg);
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Could not run lxc-destroy: {}", RED, RESET, e),
    }
}

pub fn start_container(name: &str) {
    println!("{}[INFO]{} Starting container '{}'...", GREEN, RESET, name);
    
    let status = Command::new("sudo")
        .args(&["lxc-start", "-P", LXC_PATH, "-n", name, "-d"]) 
        .status();

    match status {
        Ok(s) if s.success() => println!("{}[SUCCESS]{} Container is now running.", GREEN, RESET),
        _ => eprintln!("{}[ERROR]{} Failed to start container. Check if it exists.", RED, RESET),
    }
}

pub fn attach_to_container(name: &str) {
    println!("{}[MODE]{} Entering Saferoom: {}. Type 'exit' to return.", BOLD, name, RESET);

    let _ = Command::new("sudo")
        .args(&["lxc-attach", "-P", LXC_PATH, "-n", name, "--", "bash"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status();
}

pub fn stop_container(name: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}[SHUTDOWN]{} Stopping container '{}'...", YELLOW, RESET, name);

    let process = Command::new("sudo")
        .args(&["lxc-stop", "-P", LXC_PATH, "-n", name])
        .output();

    match process {
        Ok(output) => {
            if output.status.success() {
                println!("{}[SUCCESS]{} Container '{}' stopped.", GREEN, RESET, name);
            } else {
                eprintln!("{}[ERROR]{} Failed to stop container.", RED, RESET);
            }
        },
        Err(e) => eprintln!("{}[FATAL]{} Error: {}", RED, RESET, e),
    }
}

pub fn send_command(name: &str, command_args: &[&str]) {
    if command_args.is_empty() {
        eprintln!("{}[ERROR]{} No command provided.", RED, RESET);
        return;
    }

    // 1. CEK STATUS DULU (Pre-flight Check)
    let check_status = Command::new("sudo")
        .args(&["/usr/bin/lxc-info", "-P", LXC_PATH, "-n", name, "-s"])
        .output();

    if let Ok(out) = check_status {
        let output_str = String::from_utf8_lossy(&out.stdout);
        if !output_str.contains("RUNNING") {
            println!("{}[ERROR]{} Container '{}' is NOT running.", RED, RESET, name);
            println!("{}Tip:{} Run 'melisa --run {}' first.", YELLOW, RESET, name);
            return; // Berhenti di sini, jangan lanjut eksekusi
        }
    } else {
        eprintln!("{}[ERROR]{} Gagal mengecek status kontainer.", RED, RESET);
        return;
    }

    // 2. JIKA RUNNING, BARU EKSEKUSI
    println!("{}[SEND]{} Executing on '{}'...", BOLD, name, RESET);

    let status = Command::new("sudo")
        .arg("lxc-attach")
        .arg("-P")
        .arg(LXC_PATH)
        .arg("-n")
        .arg(name)
        .arg("--")
        .args(command_args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    // 3. CEK APAKAH PERINTAHNYA BERHASIL
    match status {
        Ok(s) if s.success() => println!("\n{}[DONE]{} Command executed successfully.", GREEN, RESET),
        _ => eprintln!("\n{}[ERROR]{} Command inside container returned an error.", RED, RESET),
    }
}

pub fn list_containers(only_active: bool) {
    println!("{}[INFO]{} Listing containers...", GREEN, RESET);
    
    let mut cmd = Command::new("sudo");
    cmd.args(&["lxc-ls", "-P", LXC_PATH, "--fancy"]);

    if only_active {
        cmd.arg("--active");
    }

    let output = cmd.output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                eprintln!("{}[ERROR]{} Gagal mengambil daftar.", RED, RESET);
            }
        }
        Err(e) => eprintln!("{}[FATAL]{} Error: {}", RED, RESET, e),
    }
}
//user management
pub fn add_melisa_user(username: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Adding New Melisa User: {} ---{}", BOLD, username, RESET);

    // Langkah 1: Tanya Role
    println!("{}Select Role for {}:{}", BOLD, username, RESET);
    println!("  1) Admin (Can manage users & LXC)");
    println!("  2) Regular (Can only manage LXC)");
    print!("Choose (1/2): ");
    let _ = io::stdout().flush();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("Failed to read input");

    let role = match choice.trim() {
        "1" => UserRole::Admin,
        _ => UserRole::Regular, // Default ke Regular jika input salah
    };

    // Langkah 2: Buat User Sistem
    let status = Command::new("sudo")
        .args(&["/usr/sbin/useradd", "-m", "-s", "/usr/local/bin/melisa", username])
        .status();

    if let Ok(s) = status {
        if s.success() {
            println!("{}[SUCCESS]{} User '{}' created.", GREEN, RESET, username);
            
            // Langkah 3: Set Password
            if set_user_password(username) {
                // Langkah 4: Konfigurasi Sudoers berdasarkan Role
                configure_sudoers(username, role);
            }
        } else {
            eprintln!("{}[ERROR]{} Failed to create user.", RED, RESET);
        }
    }
}

// Fungsi pembantu untuk set password
pub fn set_user_password(username: &str) -> bool {
    println!("{}[ACTION]{} Please set password for {}:", YELLOW, RESET, username);
    let status = Command::new("sudo")
        .arg("passwd")
        .arg(username)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}[SUCCESS]{} Password updated for {}.", GREEN, RESET, username);
            true
        },
        _ => {
            eprintln!("{}[ERROR]{} Failed to set password.", RED, RESET);
            false
        }
    }
}

pub fn delete_melisa_user(username: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Deleting User: {} ---{}", BOLD, username, RESET);

    // 1. PAKSA: Usir user dan matikan semua prosesnya (SSH, Bash, dll)
    println!("{}[INFO]{} Terminating all processes for user '{}'...", YELLOW, RESET, username);
    let _ = Command::new("sudo").args(&["/usr/bin/pkill", "-u", username]).status();

    // 2. Hapus user sistem
    let status_del = Command::new("sudo")
        .args(&["/usr/sbin/userdel", "-r", "-f", username]) // Tambah -f (force)
        .status();

    // 3. Hapus file sudoers
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);
    let status_rm = Command::new("sudo")
        .args(&["/usr/bin/rm", "-f", &sudoers_path])
        .status();

    match (status_del, status_rm) {
        (Ok(s1), Ok(s2)) if s1.success() && s2.success() => {
            println!("{}[SUCCESS]{} User '{}' and permissions removed.", GREEN, RESET, username);
        },
        _ => {
            eprintln!("{}[ERROR]{} Gagal menghapus total. Mungkin user sedang digunakan atau sudah hilang.", RED, RESET);
        }
    }
}

fn configure_sudoers(username: &str, role: UserRole) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    let mut commands = vec![
        "/usr/sbin/lxc-*", // Izinkan semua sub-command lxc
    ];

    match role {
        UserRole::Admin => {
            // Kita gunakan "*" agar flexibel terhadap flag (seperti -f, -m, -r)
            commands.push("/usr/sbin/useradd *");
            commands.push("/usr/sbin/userdel *");
            commands.push("/usr/bin/passwd *");
            commands.push("/usr/bin/pkill *");
            commands.push("/usr/bin/grep *");
            commands.push("/usr/sbin/lxc-*"); // Ini biasanya sudah mencakup lxc-info
            commands.push("/usr/bin/lxc-info *"); // Tambahkan secara spesifik jika perlu
            commands.push("/usr/bin/ls /etc/sudoers.d/"); // Harus sama persis dengan panggilan di Rust
            commands.push("/usr/bin/rm -f /etc/sudoers.d/melisa_*"); // Match persis dengan kode
            commands.push("/usr/bin/tee /etc/sudoers.d/melisa_*");
        },
        UserRole::Regular => {}
    }

    let sudoers_rule = format!("{} ALL=(root) NOPASSWD: {}\n", username, commands.join(", "));
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);

    // Proses tulis file dengan sudo tee...
    let mut child = Command::new("sudo")
        .args(&["/usr/bin/tee", &sudoers_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("Failed to spawn sudo tee");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(sudoers_rule.as_bytes());
    }
    child.wait().unwrap();
}

pub fn list_melisa_users() {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Registered Melisa Users ---{}", BOLD, RESET);

    // 1. Ambil daftar user asli
    let passwd_out = Command::new("grep")
        .args(&["/usr/local/bin/melisa", "/etc/passwd"])
        .output();

    let mut existing_users = Vec::new();

    if let Ok(out) = passwd_out {
        let result = String::from_utf8_lossy(&out.stdout);
        for line in result.lines() {
            if let Some(user) = line.split(':').next() {
                existing_users.push(user.to_string());
                let tag = if check_if_admin(user) { 
                    format!("{}[ADMIN]{}", GREEN, RESET) 
                } else { 
                    format!("{}[USER]{}", YELLOW, RESET) 
                };
                println!("  > {:<15} {}", user, tag);
            }
        }
    }

    // 2. LOGIKA JANITOR dengan Error Handling yang Jujur
    println!("\n{}--- Checking for Orphaned Sudoers (Trash) ---{}", BOLD, RESET);
    
    // Pastikan path /usr/bin/ls ini SAMA PERSIS dengan yang ada di file sudoers
    let sudoers_files = Command::new("sudo")
        .args(&["/usr/bin/ls", "/etc/sudoers.d/"])
        .output();
    
    match sudoers_files {
        Ok(out) if out.status.success() => {
            let files = String::from_utf8_lossy(&out.stdout);
            let mut found_trash = false;

            for file in files.lines() {
                if file.starts_with("melisa_") {
                    let user_from_file = file.replace("melisa_", "");
                    // Cukup gunakan &user_from_file karena user_from_file adalah String
                    if !existing_users.contains(&user_from_file) {
                        println!("  {}! Found trash:{} {} (User already deleted)", RED, RESET, file);
                        found_trash = true;
                    }
                }
            }
            if !found_trash { 
                println!("  {}No trash found. System is clean.{}", GREEN, RESET); 
            }
        },
        _ => {
            // Jika masuk ke sini, berarti sudo minta password atau ditolak
            println!("{}[ERROR]{} Akses ditolak saat memeriksa sudoers. Pastikan izin NOPASSWD benar.", RED, RESET);
        }
    }
}

pub fn upgrade_user(username: &str) {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Upgrading User Permissions: {} ---{}", BOLD, username, RESET);

    // Cek dulu apakah usernya memang ada di sistem
    let check_user = Command::new("id").arg(username).output();
    if let Ok(out) = check_user {
        if !out.status.success() {
            eprintln!("{}[ERROR]{} User '{}' tidak ditemukan di sistem.", RED, RESET, username);
            return;
        }
    }

    println!("Select New Role for {}:", username);
    println!("  1) Admin (Full Access)");
    println!("  2) Regular (LXC Only)");
    print!("Choose (1/2): ");
    let _ = io::stdout().flush();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    let role = match choice.trim() {
        "1" => UserRole::Admin,
        _ => UserRole::Regular,
    };

    // Panggil fungsi konfigurasi sudoers yang sudah kita buat tadi
    configure_sudoers(username, role);
    println!("{}[DONE]{} Izin user '{}' telah diperbarui.", GREEN, RESET, username);
}

// Tambahkan fungsi ini di src/cli/container.rs
fn check_if_admin(username: &str) -> bool {
    let sudoers_path = format!("/etc/sudoers.d/melisa_{}", username);
    
    let check_admin = Command::new("sudo")
        .arg("-n") // <--- KUNCI SAKTINYA DI SINI (Non-interactive)
        .args(&["/usr/bin/grep", "-qs", "useradd", &sudoers_path])
        .status();

    match check_admin {
        // Jika sukses (0), berarti dia Admin dan punya izin NOPASSWD
        Ok(s) if s.success() => true,
        // Jika gagal (karena nggak ada izin atau perlu password), langsung anggap bukan Admin
        _ => false, 
    }
}

pub fn clean_orphaned_sudoers() {
    if !ensure_admin() { return; } // Gerbang Keamanan
    println!("{}--- Cleaning Orphaned Sudoers ---{}", BOLD, RESET);
    
    // Gunakan match, jangan unwrap
    let passwd_out = Command::new("grep")
        .args(&["/usr/local/bin/melisa", "/etc/passwd"])
        .output();

    if let Ok(out) = passwd_out {
        let result = String::from_utf8_lossy(&out.stdout);
        let existing_users: Vec<&str> = result.lines()
            .map(|l| l.split(':').next().unwrap_or(""))
            .collect();

        let files_out = Command::new("sudo").args(&["/usr/bin/ls", "/etc/sudoers.d/"]).output();
        
        if let Ok(f_out) = files_out {
            let files = String::from_utf8_lossy(&f_out.stdout);
            for file in files.lines() {
                if file.starts_with("melisa_") {
                    let user_name = file.replace("melisa_", "");
                    if !existing_users.contains(&user_name.as_str()) {
                        println!("{}[CLEANING]{} Removing: {}", YELLOW, RESET, file);
                        let _ = Command::new("sudo").args(&["/usr/bin/rm", "-f", &format!("/etc/sudoers.d/{}", file)]).status();
                    }
                }
            }
        }
    }
    println!("{}[DONE]{} System is clean.", GREEN, RESET);
}

// Fungsi untuk mengecek apakah user yang sedang menjalankan aplikasi adalah Admin
fn ensure_admin() -> bool {
    // Ambil nama user yang sedang login/menjalankan binary
    let current_user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    
    if !check_if_admin(&current_user) {
        println!("{}[ERROR] Permission not allowed. This function is for admin only.{}", RED, RESET);
        return false;
    }
    true
}