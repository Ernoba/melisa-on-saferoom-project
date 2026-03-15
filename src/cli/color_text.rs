/// Terminal UI Styling Constants
/// These are used across the application for consistent professional output.

pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const CYAN: &str = "\x1b[36m";
pub const BLUE: &str = "\x1b[34;1m";
#[allow(dead_code)]
pub const YELLOW: &str = "\x1b[33m";
pub const BOLD: &str = "\x1b[1m";
pub const RESET: &str = "\x1b[0m";

// Pro tip: You can also create helper functions here
#[allow(dead_code)]
pub fn print_error(message: &str) {
    eprintln!("{}error:{} {}", RED, RESET, message);
}

#[allow(dead_code)]
pub fn print_success(message: &str) {
    println!("{}success:{} {}", GREEN, RESET, message);
}