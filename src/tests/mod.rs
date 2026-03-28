/// =============================================================================
/// MELISA - Unit Test Module
/// File: src/tests/mod.rs
///
/// Cara penggunaan:
///   cargo test                    # semua tes
///   cargo test distro             # tes distro saja
///   cargo test -- --nocapture     # lihat output println!
/// =============================================================================

// ============================================================
// TEST: src/distros/distro.rs
// Fungsi yang diuji: parse_distro_list(), generate_slug()
// CATATAN: Ubah visibility di distro.rs menjadi pub(crate)
// ============================================================
#[cfg(test)]
mod distro_tests {
    use crate::distros::distro::{parse_distro_list_pub, generate_slug_pub};

    #[test]
    fn test_generate_slug_amd64() {
        // ubuntu + 22.04 + amd64 → "ubu-22.04-x64"
        let slug = generate_slug_pub("ubuntu", "22.04", "amd64");
        assert_eq!(slug, "ubu-22.04-x64");
    }

    #[test]
    fn test_generate_slug_arm64() {
        let slug = generate_slug_pub("debian", "12", "arm64");
        assert_eq!(slug, "deb-12-a64");
    }

    #[test]
    fn test_generate_slug_i386() {
        let slug = generate_slug_pub("alpine", "3.18", "i386");
        assert_eq!(slug, "alp-3.18-x86");
    }

    #[test]
    fn test_generate_slug_unknown_arch() {
        // Arch yang tidak dikenal: gunakan apa adanya
        let slug = generate_slug_pub("fedora", "39", "riscv64");
        assert_eq!(slug, "fed-39-riscv64");
    }

    #[test]
    fn test_generate_slug_long_name_truncated() {
        // Nama panjang hanya diambil 3 karakter pertama
        let slug = generate_slug_pub("archlinux", "base", "amd64");
        assert_eq!(slug, "arc-base-x64");
    }

    #[test]
    fn test_parse_distro_list_valid() {
        let input = "\
Distribution Release Architecture Variant
-------------------------------------------
ubuntu       22.04   amd64        default
debian       12      arm64        default
alpine       3.18    i386         default
";
        let result = parse_distro_list_pub(input);
        assert_eq!(result.len(), 3);

        let ubuntu = result.iter().find(|d| d.name == "ubuntu").unwrap();
        assert_eq!(ubuntu.release, "22.04");
        assert_eq!(ubuntu.arch, "amd64");
        assert_eq!(ubuntu.pkg_manager, "apt");
        assert_eq!(ubuntu.slug, "ubu-22.04-x64");

        let debian = result.iter().find(|d| d.name == "debian").unwrap();
        assert_eq!(debian.pkg_manager, "apt");

        let alpine = result.iter().find(|d| d.name == "alpine").unwrap();
        assert_eq!(alpine.pkg_manager, "apk");
    }

    #[test]
    fn test_parse_distro_list_all_pkg_managers() {
        let input = "\
Distribution Release Architecture Variant
---
debian       12    amd64 default
ubuntu       22.04 amd64 default
kali         2024  amd64 default
fedora       39    amd64 default
centos       8     amd64 default
rocky        9     amd64 default
almalinux    9     amd64 default
alpine       3.18  amd64 default
archlinux    base  amd64 default
opensuse     15.5  amd64 default
voidlinux    5     amd64 default
";
        let result = parse_distro_list_pub(input);

        let check = |name: &str, expected_pm: &str| {
            let d = result.iter().find(|d| d.name == name)
                .unwrap_or_else(|| panic!("distro '{}' tidak ditemukan", name));
            assert_eq!(
                d.pkg_manager, expected_pm,
                "pkg_manager untuk '{}' salah: expected='{}', got='{}'",
                name, expected_pm, d.pkg_manager
            );
        };

        check("debian",    "apt");
        check("ubuntu",    "apt");
        check("kali",      "apt");
        check("fedora",    "dnf");
        check("centos",    "dnf");
        check("rocky",     "dnf");
        check("almalinux", "dnf");
        check("alpine",    "apk");
        check("archlinux", "pacman");
        check("opensuse",  "zypper");
        check("voidlinux", "apt"); // fallback ke apt
    }

    #[test]
    fn test_parse_distro_list_skips_header_lines() {
        let input = "\
DIST RELEASE ARCH VARIANT
Distribution Release Architecture Variant
-------------------------------------------
ubuntu 22.04 amd64 default
";
        let result = parse_distro_list_pub(input);
        // Hanya 1 baris data yang valid, header harus dibuang
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "ubuntu");
    }

    #[test]
    fn test_parse_distro_list_empty_input() {
        let result = parse_distro_list_pub("");
        assert!(result.is_empty(), "Input kosong harus menghasilkan list kosong");
    }

    #[test]
    fn test_parse_distro_list_incomplete_line() {
        // Baris dengan kurang dari 4 kolom harus dibuang
        let input = "ubuntu 22.04 amd64\ndebian 12 arm64 default\n";
        let result = parse_distro_list_pub(input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "debian");
    }
}

// ============================================================
// TEST: src/distros/host_distro.rs
// Fungsi yang diuji: get_distro_config()
// ============================================================
#[cfg(test)]
mod host_distro_tests {
    use crate::distros::host_distro::{HostDistro, FirewallKind, get_distro_config};

    #[test]
    fn test_fedora_config() {
        let cfg = get_distro_config(&HostDistro::Fedora);
        assert_eq!(cfg.pkg_manager, "dnf");
        assert_eq!(cfg.firewall_tool, FirewallKind::Firewalld);
        assert_eq!(cfg.ssh_service, "sshd");
        assert!(cfg.lxc_packages.contains(&"lxc"));
    }

    #[test]
    fn test_ubuntu_config() {
        let cfg = get_distro_config(&HostDistro::Ubuntu);
        assert_eq!(cfg.pkg_manager, "apt-get");
        assert_eq!(cfg.firewall_tool, FirewallKind::Ufw);
        assert_eq!(cfg.ssh_service, "ssh");
        assert!(cfg.lxc_packages.contains(&"lxc-utils"));
    }

    #[test]
    fn test_debian_config() {
        let cfg = get_distro_config(&HostDistro::Debian);
        assert_eq!(cfg.pkg_manager, "apt-get");
        assert_eq!(cfg.firewall_tool, FirewallKind::Ufw);
    }

    #[test]
    fn test_arch_config() {
        let cfg = get_distro_config(&HostDistro::Arch);
        assert_eq!(cfg.pkg_manager, "pacman");
        assert_eq!(cfg.firewall_tool, FirewallKind::Iptables);
        assert_eq!(cfg.ssh_package, "openssh");
    }

    #[test]
    fn test_unknown_distro_fallback() {
        let cfg = get_distro_config(&HostDistro::Unknown("nixos".to_string()));
        // Harus fallback ke apt-get / ufw
        assert_eq!(cfg.pkg_manager, "apt-get");
        assert_eq!(cfg.firewall_tool, FirewallKind::Ufw);
        assert_eq!(cfg.ssh_service, "ssh");
    }
}

// ============================================================
// TEST: src/core/metadata.rs
// Fungsi yang diuji: validate_container_name() (fungsi baru)
// inject_distro_metadata() (dengan tempdir)
// ============================================================
#[cfg(test)]
mod metadata_tests {
    use crate::core::metadata::{MelisaError, validate_container_name};

    #[test]
    fn test_valid_container_names() {
        let valid_names = ["myapp", "ubuntu-dev", "test123", "a", "x-y-z"];
        for name in &valid_names {
            assert!(
                validate_container_name(name),
                "Nama '{}' seharusnya valid", name
            );
        }
    }

    #[test]
    fn test_invalid_container_name_slash() {
        assert!(!validate_container_name("container/hack"));
        assert!(!validate_container_name("/etc/passwd"));
        assert!(!validate_container_name("foo/bar"));
    }

    #[test]
    fn test_invalid_container_name_backslash() {
        assert!(!validate_container_name("container\\hack"));
        assert!(!validate_container_name("foo\\bar"));
    }

    #[test]
    fn test_invalid_container_name_dotdot() {
        assert!(!validate_container_name(".."));
        // ".." di tengah masih oke karena tidak sama persis
        // tapi "/foo/.." tidak oke karena ada slash
    }

    #[test]
    fn test_melisa_error_display() {
        let err = MelisaError::InvalidName("test/bad".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test/bad"), "Error message harus menyebut nama yang salah");
    }

    #[test]
    fn test_security_violation_error() {
        let err = MelisaError::SecurityViolation("../../etc".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("../../etc"));
        assert!(msg.contains("traversal") || msg.contains("Path"));
    }

    #[test]
    fn test_metadata_not_found_error() {
        let err = MelisaError::MetadataNotFound("nocontainer".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("nocontainer"));
        assert!(msg.contains("MELISA") || msg.contains("Metadata"));
    }

    #[tokio::test]
    async fn test_inject_metadata_security_violation() {
        use crate::core::metadata::inject_distro_metadata;
        use crate::core::container::DistroMetadata;

        let meta = DistroMetadata {
            slug: "ubu-22.04-x64".to_string(),
            name: "ubuntu".to_string(),
            release: "22.04".to_string(),
            arch: "amd64".to_string(),
            variant: "default".to_string(),
            pkg_manager: "apt".to_string(),
        };

        // Percobaan path traversal harus ditolak
        let result = inject_distro_metadata("/tmp", "../etc", &meta).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            MelisaError::SecurityViolation(name) => {
                assert_eq!(name, "../etc");
            }
            e => panic!("Error yang diharapkan SecurityViolation, bukan: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_inject_metadata_with_slash_rejected() {
        use crate::core::metadata::inject_distro_metadata;
        use crate::core::container::DistroMetadata;

        let meta = DistroMetadata {
            slug: "test".to_string(),
            name: "alpine".to_string(),
            release: "3.18".to_string(),
            arch: "amd64".to_string(),
            variant: "default".to_string(),
            pkg_manager: "apk".to_string(),
        };

        let result = inject_distro_metadata("/tmp", "evil/path", &meta).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MelisaError::SecurityViolation(_)));
    }

    #[tokio::test]
    async fn test_inject_and_inspect_metadata_roundtrip() {
        use crate::core::metadata::{inject_distro_metadata, inspect_container_metadata};
        use crate::core::container::DistroMetadata;
        use std::path::PathBuf;

        // Buat temp dir yang mirip struktur LXC
        let tmp = std::env::temp_dir();
        let container_name = format!("melisa-test-{}", uuid::Uuid::new_v4());
        let rootfs_etc = tmp.join(&container_name).join("rootfs").join("etc");
        std::fs::create_dir_all(&rootfs_etc).unwrap();

        let meta = DistroMetadata {
            slug: "deb-12-x64".to_string(),
            name: "debian".to_string(),
            release: "12".to_string(),
            arch: "amd64".to_string(),
            variant: "default".to_string(),
            pkg_manager: "apt".to_string(),
        };

        // Inject
        let base_path = tmp.to_str().unwrap();
        let result = inject_distro_metadata(base_path, &container_name, &meta).await;
        assert!(result.is_ok(), "inject_distro_metadata gagal: {:?}", result);

        // Verifikasi file ada
        let info_path = rootfs_etc.join("melisa-info");
        assert!(info_path.exists(), "File melisa-info tidak dibuat");

        // Baca dan cek konten
        let content = std::fs::read_to_string(&info_path).unwrap();
        assert!(content.contains(&format!("MELISA_INSTANCE_NAME={}", container_name)));
        assert!(content.contains("DISTRO_NAME=debian"));
        assert!(content.contains("DISTRO_RELEASE=12"));
        assert!(content.contains("ARCHITECTURE=amd64"));
        assert!(content.contains("MELISA_INSTANCE_ID="));
        assert!(content.contains("CREATED_AT="));

        // Cleanup
        let _ = std::fs::remove_dir_all(tmp.join(&container_name));
    }
}

// ============================================================
// TEST: src/core/container.rs
// Fungsi yang diuji: get_pkg_update_cmd() (fungsi baru yang diekstrak)
// ============================================================
#[cfg(test)]
mod container_tests {
    use crate::core::container::get_pkg_update_cmd;

    #[test]
    fn test_apt_update_cmd() {
        assert_eq!(get_pkg_update_cmd("apt"), "apt-get update -y");
    }

    #[test]
    fn test_dnf_update_cmd() {
        assert_eq!(get_pkg_update_cmd("dnf"), "dnf makecache");
    }

    #[test]
    fn test_apk_update_cmd() {
        assert_eq!(get_pkg_update_cmd("apk"), "apk update");
    }

    #[test]
    fn test_pacman_update_cmd() {
        assert_eq!(get_pkg_update_cmd("pacman"), "pacman -Sy --noconfirm");
    }

    #[test]
    fn test_zypper_update_cmd() {
        assert_eq!(get_pkg_update_cmd("zypper"), "zypper --non-interactive refresh");
    }

    #[test]
    fn test_unknown_pkg_manager_fallback() {
        assert_eq!(get_pkg_update_cmd("unknown"), "true");
        assert_eq!(get_pkg_update_cmd(""), "true");
        assert_eq!(get_pkg_update_cmd("yum"), "true");
    }
}

// ============================================================
// TEST: src/core/project_management.rs
// Fungsi yang diuji: validate_project_input() (fungsi baru)
// ============================================================
#[cfg(test)]
mod project_management_tests {
    use crate::core::project_management::validate_project_input;

    #[test]
    fn test_valid_inputs() {
        assert!(validate_project_input("myproject", "alice"));
        assert!(validate_project_input("backend-api", "bob123"));
        assert!(validate_project_input("proj_name", "user_name"));
    }

    #[test]
    fn test_reject_slash_in_project() {
        assert!(!validate_project_input("proj/evil", "alice"));
        assert!(!validate_project_input("/etc/shadow", "alice"));
    }

    #[test]
    fn test_reject_slash_in_username() {
        assert!(!validate_project_input("project", "alice/hack"));
        assert!(!validate_project_input("project", "/root"));
    }

    #[test]
    fn test_reject_dotdot_in_project() {
        assert!(!validate_project_input("..", "alice"));
        assert!(!validate_project_input("../secret", "alice"));
    }

    #[test]
    fn test_reject_dotdot_in_username() {
        assert!(!validate_project_input("project", ".."));
        assert!(!validate_project_input("project", "../admin"));
    }
}

// ============================================================
// TEST: src/cli/executor.rs
// Fungsi yang diuji: parse_command() (fungsi baru yang diekstrak)
// ============================================================
#[cfg(test)]
mod executor_tests {
    use crate::cli::executor::parse_command;

    #[test]
    fn test_parse_basic_command() {
        let (parts, audit) = parse_command("melisa --list");
        assert_eq!(parts, vec!["melisa", "--list"]);
        assert!(!audit);
    }

    #[test]
    fn test_parse_command_with_audit_flag() {
        let (parts, audit) = parse_command("melisa --list --audit");
        assert_eq!(parts, vec!["melisa", "--list"]);
        assert!(audit);
    }

    #[test]
    fn test_parse_audit_flag_anywhere() {
        let (parts, audit) = parse_command("melisa --audit --create mybox ubu-22.04-x64");
        assert_eq!(parts, vec!["melisa", "--create", "mybox", "ubu-22.04-x64"]);
        assert!(audit);
    }

    #[test]
    fn test_parse_empty_command() {
        let (parts, audit) = parse_command("");
        assert!(parts.is_empty());
        assert!(!audit);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let (parts, audit) = parse_command("   ");
        assert!(parts.is_empty());
        assert!(!audit);
    }

    #[test]
    fn test_parse_cd_command() {
        let (parts, audit) = parse_command("cd /home/user");
        assert_eq!(parts, vec!["cd", "/home/user"]);
        assert!(!audit);
    }

    #[test]
    fn test_parse_exit_command() {
        let (parts, audit) = parse_command("exit");
        assert_eq!(parts, vec!["exit"]);
        assert!(!audit);
    }

    #[test]
    fn test_parse_melisa_send_with_multi_word_cmd() {
        let (parts, audit) = parse_command("melisa --send mybox apt update");
        assert_eq!(parts, vec!["melisa", "--send", "mybox", "apt", "update"]);
        assert!(!audit);
    }
}