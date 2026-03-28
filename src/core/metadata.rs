use crate::VERSION;
use crate::AUTHORS;

use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;
use chrono::Local;

use tracing::{info, instrument}; 
use std::os::unix::fs::PermissionsExt;

use crate::core::container::DistroMetadata;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum MelisaError {
    #[allow(dead_code)]
    #[error("Invalid container name: {0}")]
    InvalidName(String),
    
    #[error("IO failure: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Path traversal detected in name: {0}")]
    SecurityViolation(String),

    // Tambahkan varian baru ini:
    #[error("Metadata not found for container '{0}'. Is it a MELISA container?")]
    MetadataNotFound(String),
}

#[allow(dead_code)]
pub fn validate_container_name(name: &str) -> bool {
    !name.contains('/') && !name.contains('\\') && name != ".."
}

pub async fn print_version() {
    println!("MELISA Engine v{}", VERSION);
    println!("Copyright (c) 2026 {}", AUTHORS);
}

#[instrument(skip(meta), fields(container_name = %name))]
pub async fn inject_distro_metadata(
    lxc_base_path: &str, 
    name: &str, 
    meta: &DistroMetadata
) -> Result<(), MelisaError> {
    if name.contains('/') || name.contains('\\') || name == ".." {
        return Err(MelisaError::SecurityViolation(name.to_string()));
    }

    let rootfs_path = PathBuf::from(lxc_base_path).join(name).join("rootfs");
    let etc_path = rootfs_path.join("etc");
    let target_path = etc_path.join("melisa-info");
    let temp_path = etc_path.join("melisa-info.tmp");

    if !tokio::fs::try_exists(&etc_path).await.unwrap_or(false) {
        fs::create_dir_all(&etc_path).await.map_err(|e| {
            // PAKAI PATH LENGKAP: tracing::error!
            tracing::error!("CRITICAL: Failed to create etc directory at {:?}: {}", etc_path, e);
            MelisaError::Io(e)
        })?;
    }

    let instance_id = Uuid::new_v4().to_string();
    let content = format!(
"MELISA_INSTANCE_NAME={}
MELISA_INSTANCE_ID={}
DISTRO_SLUG={}
DISTRO_NAME={}
DISTRO_RELEASE={}
ARCHITECTURE={}
CREATED_AT={}\n",
        name, instance_id, meta.slug, meta.name, meta.release, meta.arch, Local::now().to_rfc3339()
    );

    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await
            .map_err(|e| {
                tracing::error!("Failed to open temp file {:?}: {}", temp_path, e);
                MelisaError::Io(e)
            })?;

        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        file.sync_all().await?; 
    }

    #[cfg(unix)]
    {
        let perms = std::fs::Permissions::from_mode(0o644);
        fs::set_permissions(&temp_path, perms).await.map_err(|e| {
            tracing::error!("Failed to set permissions on {:?}: {}", temp_path, e);
            MelisaError::Io(e)
        })?;
    }

    fs::rename(&temp_path, &target_path).await.map_err(|e| {
        tracing::error!("Atomic rename failed from {:?} to {:?}: {}", temp_path, target_path, e);
        MelisaError::Io(e)
    })?;

    info!("Metadata successfully injected for container: {}", name);
    Ok(())
}

// cari metadata kontainer berdasarkan nama 
#[instrument(fields(container_name = %name))]
pub async fn inspect_container_metadata(
    name: &str
) -> Result<String, MelisaError> {

    let lxc_path = Path::new("/var/lib/lxc");
    // 1. Validasi Keamanan (Sama seperti saat inject)
    if name.contains('/') || name.contains('\\') || name == ".." {
        return Err(MelisaError::SecurityViolation(name.to_string()));
    }

    // 2. Konstruksi Path ke file melisa-info
    let metadata_path = PathBuf::from(lxc_path)
        .join(name)
        .join("rootfs")
        .join("etc")
        .join("melisa-info");

    // 3. Cek apakah file metadata ada
    if !tokio::fs::try_exists(&metadata_path).await.unwrap_or(false) {
        return Err(MelisaError::MetadataNotFound(name.to_string()));
    }

    // 4. Baca file secara asynchronous
    let content = fs::read_to_string(&metadata_path).await.map_err(|e| {
        tracing::error!("Failed to read metadata at {:?}: {}", metadata_path, e);
        MelisaError::Io(e)
    })?;

    info!("Metadata found and retrieved for: {}", name);
    Ok(content)
}