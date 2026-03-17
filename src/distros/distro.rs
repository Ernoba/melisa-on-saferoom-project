// src/distros/distro.rs
use tokio::process::Command;
use crate::core::container::DistroMetadata;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const GLOBAL_CACHE: &str = "/tmp/melisa_global_distros.cache";
const CACHE_EXPIRY: u64 = 3600; 

pub async fn get_lxc_distro_list() -> (Vec<DistroMetadata>, bool) {
    let cache_exists = Path::new(GLOBAL_CACHE).exists();
    
    if cache_exists && is_cache_fresh(GLOBAL_CACHE) {
        if let Ok(content) = fs::read_to_string(GLOBAL_CACHE) {
            return (parse_distro_list(&content), true);
        }
    }

    // GANTI DI SINI: Panggil lxc-download langsung untuk bypass pengecekan nama kontainer
    // Kita coba dua lokasi umum, biasanya di Fedora ada di /usr/share/lxc/templates/
    let output = Command::new("sudo")
        .args(&["/usr/share/lxc/templates/lxc-download", "--list"])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let content = String::from_utf8_lossy(&out.stdout);
            if !content.is_empty() {
                let _ = fs::write(GLOBAL_CACHE, content.to_string());
                let _ = Command::new("sudo").args(&["chmod", "666", GLOBAL_CACHE]).status().await;
                return (parse_distro_list(&content), false);
            }
        },
        _ => {
            // Jika cara pertama gagal (mungkin path beda), coba cara "dummy name" sebagai fallback
            let fallback = Command::new("sudo")
                .args(&["lxc-create", "-n", "MELISA_PROBE_UNUSED", "-t", "download", "--", "--list"])
                .output()
                .await;
            
            if let Ok(out) = fallback {
                let content = String::from_utf8_lossy(&out.stdout);
                if out.status.success() && !content.is_empty() {
                    let _ = fs::write(GLOBAL_CACHE, content.to_string());
                    return (parse_distro_list(&content), false);
                }
            }

            if cache_exists {
                if let Ok(old_content) = fs::read_to_string(GLOBAL_CACHE) {
                    return (parse_distro_list(&old_content), true);
                }
            }
        }
    }

    (Vec::new(), false)
}

fn is_cache_fresh(path: &str) -> bool {
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let last_mod = mtime.duration_since(UNIX_EPOCH).unwrap().as_secs();
            return now - last_mod < CACHE_EXPIRY;
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