use std::process::Command;

pub fn check_lxc() -> bool {
    // Menggunakan lxc-info karena ini adalah bagian dari core tools LXC
    if let Ok(output) = Command::new("lxc-info")
        .arg("--version")
        .output()
    {
        output.status.success()
    } else {
        // Jika lxc-info tidak ditemukan di PATH, return false
        false
    }
}

pub fn check_root() -> bool {
    // Pastikan sudah menambahkan 'libc = "0.2"' di Cargo.toml
    unsafe { libc::geteuid() == 0 }
}