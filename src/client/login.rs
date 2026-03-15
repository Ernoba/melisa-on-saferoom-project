use std::io::{self, Write};
use crate::local_data::save_profil::Profil;
use crossterm::event::{read, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crate::cli::color_text::{GREEN, RED, BOLD, RESET};

pub fn login(_username: &str, _password: &str) -> bool {
    if let Some(profil) = Profil::load_from_file() {
        println!("{}Authenticated as {}{}", BOLD, profil.username, RESET);

        print!("Password for {}: ", profil.username);
        io::stdout().flush().unwrap();
        
        let password = read_masked().unwrap_or_default();

        if password == profil.password {
            println!("\n{}Access granted.{}", GREEN, RESET);
            true
        } else {
            eprintln!("\n{}Authentication failed: incorrect password.{}", RED, RESET);
            false
        }
    } else {
        println!("\n\x1b[33mNo profile found. Please create one:\x1b[0m");

        print!("Enter new username: ");
        io::stdout().flush().unwrap();
        let mut input_username = String::new();
        io::stdin().read_line(&mut input_username).unwrap();

        print!("Enter new password: ");
        io::stdout().flush().unwrap();
        let input_password = read_masked().unwrap_or_default();

        let new_profil = Profil {
            username: input_username.trim().to_string(),
            password: input_password.trim().to_string(),
        };

        new_profil.save_to_file();
        println!("\n{}Profile saved successfully.{}", GREEN, RESET);
        true
    }
}

/// Helper function to read input and display asterisks
fn read_masked() -> io::Result<String> {
    let mut password = String::new();
    enable_raw_mode()?; // Start capturing raw keypresses

    loop {
        if let Event::Key(KeyEvent { code, .. }) = read()? {
            match code {
                KeyCode::Enter => break,
                KeyCode::Char(c) => {
                    password.push(c);
                    print!("*");
                }
                KeyCode::Backspace => {
                    if !password.is_empty() {
                        password.pop();
                        print!("\x08 \x08"); // Move cursor back, overwrite with space, move back again
                    }
                }
                KeyCode::Esc => {
                    disable_raw_mode()?;
                    return Ok(String::new());
                }
                _ => {}
            }
            io::stdout().flush()?;
        }
    }

    disable_raw_mode()?; // Must disable raw mode to return terminal to normal
    Ok(password)
}