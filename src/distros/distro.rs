// src/distros/distro.rs
use tokio::process::Command;
use crate::core::container::DistroMetadata;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::time::sleep;

const GLOBAL_CACHE: &str = "/tmp/melisa_global_distros.cache";
const LOCK_FILE: &str = "/tmp/melisa_distro.lock";
const CACHE_EXPIRY: u64 = 3600; 

pub async fn get_lxc_distro_list() -> (Vec<DistroMetadata>, bool) {
    let cache_exists = Path::new(GLOBAL_CACHE).exists();
    
    // 1. Cek super cepat: Kalau cache fresh dan NGGAK ada yang lagi nge-lock, langsung ambil.
    if cache_exists && is_cache_fresh(GLOBAL_CACHE) && !Path::new(LOCK_FILE).exists() {
        if let Ok(content) = fs::read_to_string(GLOBAL_CACHE) {
            return (parse_distro_list(&content), true);
        }
    }

    // 2. Mekanisme Locking yang lebih ketat
    let mut retry_count = 0;
    let max_retries = 40; // Kita kasih waktu lebih lama (20 detik) karena lxc-download emang lemot

    loop {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true) // Cuma satu orang yang boleh menang di sini
            .open(LOCK_FILE) 
        {
            Ok(_) => {
                // KITA PEMENANGNYA: Kita yang dapet mandat buat update cache
                break; 
            }
            Err(_) => {
                // ADA USER LAIN: Waiter harus sabar nunggu sampai LOCK_FILE dihapus sama si pemenang
                if retry_count >= max_retries {
                    if cache_exists {
                        if let Ok(old_content) = fs::read_to_string(GLOBAL_CACHE) {
                            return (parse_distro_list(&old_content), true);
                        }
                    }
                    break; 
                }

                // Cek apakah lock sudah dilepas (artinya si pemenang beres nulis)
                if !Path::new(LOCK_FILE).exists() {
                    if let Ok(content) = fs::read_to_string(GLOBAL_CACHE) {
                        return (parse_distro_list(&content), true);
                    }
                }

                sleep(Duration::from_millis(500)).await;
                retry_count += 1;
            }
        }
    }

    // 3. Eksekusi Penarikan Data (Hanya dijalankan oleh si pemenang Lock)
    // Mencoba lxc-download langsung
    let output = Command::new("sudo")
        .args(&["/usr/share/lxc/templates/lxc-download", "--list"])
        .output()
        .await;

    let result = match output {
        Ok(out) if out.status.success() => {
            let content = String::from_utf8_lossy(&out.stdout);
            if !content.is_empty() {
                let _ = fs::write(GLOBAL_CACHE, content.to_string());
                let _ = Command::new("sudo").args(&["chmod", "666", GLOBAL_CACHE]).status().await;
                (parse_distro_list(&content), false)
            } else {
                (Vec::new(), false)
            }
        },
        _ => {
            // Fallback kalau lxc-download path salah
            let fallback = Command::new("sudo")
                .args(&["lxc-create", "-n", "MELISA_PROBE_UNUSED", "-t", "download", "--", "--list"])
                .output()
                .await;
            
            if let Ok(out) = fallback {
                let content = String::from_utf8_lossy(&out.stdout);
                if out.status.success() && !content.is_empty() {
                    let _ = fs::write(GLOBAL_CACHE, content.to_string());
                    let _ = Command::new("sudo").args(&["chmod", "666", GLOBAL_CACHE]).status().await;
                    (parse_distro_list(&content), false)
                } else {
                    (Vec::new(), false)
                }
            } else {
                if cache_exists {
                    if let Ok(old_content) = fs::read_to_string(GLOBAL_CACHE) {
                        (parse_distro_list(&old_content), true)
                    } else {
                        (Vec::new(), false)
                    }
                } else {
                    (Vec::new(), false)
                }
            }
        }
    };

    // 4. KRITIKAL: Hapus lock biar user lain nggak nunggu selamanya!
    let _ = fs::remove_file(LOCK_FILE);
    
    result
}

fn is_cache_fresh(path: &str) -> bool {
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                if let Ok(last_mod) = mtime.duration_since(UNIX_EPOCH) {
                    return now.as_secs() - last_mod.as_secs() < CACHE_EXPIRY;
                }
            }
        }
    }
    false
}

fn parse_distro_list(content: &str) -> Vec<DistroMetadata> {
    let mut distros = Vec::new();
    for line in content.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() >= 4 && !line.contains("Distribution") && !line.contains("---") {
            let name = p[0].to_string();
            let release = p[1].to_string();
            let arch = p[2].to_string();
            let variant = p[3].to_string();
            
            let slug = generate_slug(&name, &release, &arch);
            let pkg_manager = match name.as_str() {
                "debian" | "ubuntu" | "kali" => "apt",
                "fedora" | "centos" | "rocky" | "almalinux" => "dnf",
                "alpine" => "apk",
                "archlinux" => "pacman",
                _ => "apt",
            }.to_string();

            distros.push(DistroMetadata { slug, name, release, arch, variant, pkg_manager });
        }
    }
    distros
}

fn generate_slug(name: &str, release: &str, arch: &str) -> String {
    let s_arch = match arch { "amd64" => "x64", "arm64" => "a64", _ => arch };
    format!("{}-{}-{}", &name[..name.len().min(3)], release, s_arch).to_lowercase()
}