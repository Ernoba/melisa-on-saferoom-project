use std::process::{Command, Stdio};
use std::io::{self, Write};
use crate::cli::color_text::{GREEN, RED, CYAN, BOLD, RESET};

pub fn install() {
    println!("\n{}LXC ENVIRONMENT INITIALIZATION{}\n", BOLD, RESET);

    let commands = vec![
        ("Synchronizing package repositories", "dnf", vec!["update", "-y"]),
        ("Installing virtualization core components", "dnf", vec!["install", "-y", "lxc", "lxc-templates", "libvirt", "bridge-utils"]),
        ("Loading veth kernel module", "modprobe", vec!["veth"]),
        ("Starting LXC system services", "systemctl", vec!["enable", "--now", "lxc.service"]),
    ];

    for (desc, prog, args) in commands {
        if !execute_step(desc, prog, &args) {
            eprintln!("\n{}CRITICAL_FAILURE: Setup terminated at step '{}'{}", RED, desc, RESET);
            std::process::exit(1);
        }
    }

    fix_uidmap_permissions();

    if let Ok(user) = std::env::var("SUDO_USER") {
        setup_user_mapping(&user);
    } else {
        println!("{}NOTICE: Proceeding without SUDO_USER mapping settings.{}", CYAN, RESET);
    }

    println!("\n{}VERIFYING SYSTEM CONFIGURATION...{}", BOLD, RESET);
    let _ = Command::new("lxc-checkconfig").status();

    println!("\n{}LXC DEPLOYMENT COMPLETED SUCCESSFULLY{}\n", GREEN, RESET);

    println!("{}ENJOY YOUR TIME RUN 'melisa'! FOR USE THIS PROGRAM{}",
        CYAN,
        RESET
    );
}

fn execute_step(description: &str, program: &str, args: &[&str]) -> bool {
    // Print description with fixed width for alignment
    print!("  {:<50}", description);
    io::stdout().flush().unwrap();

    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("[ {}OK{} ]", GREEN, RESET);
            true
        }
        _ => {
            println!("[ {}FAILED{} ]", RED, RESET);
            false
        }
    }
}

fn fix_uidmap_permissions() {
    println!("\nApplying binary permission overrides...");
    let paths = ["/usr/bin/newuidmap", "/usr/bin/newgidmap"];
    
    for path in &paths {
        let status = Command::new("chmod").args(&["u+s", path]).status();
        match status {
            Ok(s) if s.success() => println!("  {:<50} [ {}OK{} ]", path, GREEN, RESET),
            _ => println!("  {:<50} [ {}FAILED{} ]", path, RED, RESET),
        }
    }
}

fn setup_user_mapping(username: &str) {
    println!("\nConfiguring sub-UID/GID mapping for: {}", username);
    let status = Command::new("usermod")
        .args(&["--add-subuids", "100000-165535", "--add-subgids", "100000-165535", username])
        .status();

    print!("  Sub-resource allocation");
    if let Ok(s) = status {
        if s.success() {
            println!(" {:>26} [ {}OK{} ]", "", GREEN, RESET);
        } else {
            println!(" {:>26} [ {}FAILED{} ]", "", RED, RESET);
        }
    }
}