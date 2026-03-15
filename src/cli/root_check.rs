pub fn check_root() -> bool {
    // Pastikan sudah menambahkan 'libc = "0.2"' di Cargo.toml
    unsafe { libc::geteuid() == 0 }
}