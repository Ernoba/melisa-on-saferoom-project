use tokio::fs;
use crate::cli::color_text::{YELLOW, RESET};

// ==============================================================================
// HOST DISTRIBUTION DETECTION & CONFIGURATION ENGINE
// Description: Identifies the underlying Linux host OS and maps it to the
//              appropriate package managers, services, and firewall tools.
// ==============================================================================

/// Represents the supported underlying Linux host distributions.
#[derive(Debug, Clone, PartialEq)]
pub enum HostDistro {
    Fedora,
    Ubuntu,
    Debian,
    Arch,
    Unknown(String),
}

/// Defines the specific firewall management tool used by the host OS.
#[derive(Debug, Clone, PartialEq)]
pub enum FirewallKind {
    Firewalld, // Standard for RHEL/Fedora
    Ufw,       // Standard for Ubuntu/Debian
    Iptables,  // Standard for Arch or minimal systems
}

/// Core configuration mapping for package managers and system services.
#[derive(Debug, Clone)]
pub struct DistroConfig {
    pub pkg_manager: &'static str,
    pub update_args: Vec<&'static str>,
    pub lxc_packages: Vec<&'static str>,
    pub ssh_package: &'static str,
    pub firewall_tool: FirewallKind,
    pub ssh_service: &'static str,
}

// --- CORE DETECTION LOGIC ---

/// Asynchronously parses /etc/os-release to determine the host's operating system.
/// This method is lightweight and standard across virtually all modern Linux distros.
pub async fn detect_host_distro() -> HostDistro {
    // 1. Attempt to read the standard os-release file non-blockingly
    let content = fs::read_to_string("/etc/os-release")
        .await
        .unwrap_or_default()
        .to_lowercase();

    // 2. Map the OS signature to the appropriate enum
    if content.contains("fedora") || content.contains("rhel") || content.contains("centos") || content.contains("rocky") {
        HostDistro::Fedora
    } else if content.contains("ubuntu") {
        HostDistro::Ubuntu
    } else if content.contains("debian") {
        HostDistro::Debian
    } else if content.contains("arch") {
        HostDistro::Arch
    } else {
        // Fallback: Extract the exact ID if the distro is unsupported
        let id = content
            .lines()
            .find(|l| l.starts_with("id="))
            .unwrap_or("id=unknown")
            .replace("id=", "");
        HostDistro::Unknown(id)
    }
}

// --- CONFIGURATION MAPPING ---

/// Retrieves the specific installation arguments and service names based on the detected distro.
pub fn get_distro_config(distro: &HostDistro) -> DistroConfig {
    match distro {
        HostDistro::Fedora => DistroConfig {
            pkg_manager: "dnf",
            update_args: vec!["update", "-y"],
            lxc_packages: vec!["lxc", "lxc-templates", "libvirt", "bridge-utils"],
            ssh_package: "openssh-server",
            firewall_tool: FirewallKind::Firewalld,
            ssh_service: "sshd",
        },
        HostDistro::Ubuntu | HostDistro::Debian => DistroConfig {
            pkg_manager: "apt-get",
            update_args: vec!["update"], // Note: Ubuntu apt-get update doesn't use -y, install does
            lxc_packages: vec!["lxc", "lxc-utils", "bridge-utils"],
            ssh_package: "openssh-server",
            firewall_tool: FirewallKind::Ufw,
            ssh_service: "ssh", // CRITICAL: Ubuntu uses 'ssh' for the service name, not 'sshd'
        },
        HostDistro::Arch => DistroConfig {
            pkg_manager: "pacman",
            update_args: vec!["-Sy", "--noconfirm"],
            lxc_packages: vec!["lxc", "bridge-utils"],
            ssh_package: "openssh",
            firewall_tool: FirewallKind::Iptables,
            ssh_service: "sshd",
        },
        HostDistro::Unknown(id) => {
            // SAFETY FALLBACK: Log a warning but attempt to proceed using Debian/Ubuntu defaults
            // since apt is the most widely adopted standard outside of the RHEL ecosystem.
            eprintln!(
                "{}[WARNING]{} Host OS '{}' is not explicitly supported. Falling back to 'apt-get' safe defaults.",
                YELLOW, RESET, id
            );
            DistroConfig {
                pkg_manager: "apt-get",
                update_args: vec!["update"],
                lxc_packages: vec!["lxc", "lxc-utils", "bridge-utils"],
                ssh_package: "openssh-server",
                firewall_tool: FirewallKind::Ufw,
                ssh_service: "ssh",
            }
        }
    }
}