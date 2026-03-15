use std::fs;
use rustyline::{Editor, Config};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;

// Menggunakan crate::cli karena mereka sekarang sudah 'pub' di parent
use crate::cli::color_text::{BOLD, RESET};
use crate::cli::helper::MelisaHelper;
use crate::cli::prompt::Prompt;
use crate::cli::executor::{execute_command, ExecResult};

pub fn melisa() {
    let _ = fs::create_dir_all("data");
    let history_path = "data/history.txt";

    // Gunakan unwrap_or_default agar lebih ringkas
    let config = Config::builder()
        .history_ignore_dups(true).ok()
        .map(|b| b.build())
        .unwrap_or_default();

    let mut rl: Editor<MelisaHelper, FileHistory> = Editor::with_config(config).expect("Fail init");

    rl.set_helper(Some(MelisaHelper {
        hinter: HistoryHinter {},
        highlighter: MatchingBracketHighlighter::new(),
        validator: MatchingBracketValidator::new(),
        file_completer: FilenameCompleter::new(),
    }));

    let _ = rl.load_history(history_path);
    let p_info = Prompt::new();

    println!("{BOLD}Authenticated as melisa. Access granted.{RESET}");

    loop {
        // Fix: Berikan tipe String secara eksplisit untuk prompt
        let prompt_str: String = p_info.build();

        match rl.readline(&prompt_str) {
            Ok(line) => {
                // FIX E0282: Berikan anotasi tipe &str agar compiler tidak bingung
                let input: &str = line.trim();
                
                if input.is_empty() { continue; }

                match execute_command(input, &p_info.user, &p_info.home) {
                    ExecResult::Break => {
                        let _ = rl.save_history(history_path);
                        break;
                    },
                    ExecResult::Error(e) => eprintln!("{}", e),
                    ExecResult::Continue => {
                        let _ = rl.add_history_entry(input);
                        // Save history berkala jika perlu
                        let _ = rl.save_history(history_path); 
                    }
                }
            },
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => {
                let _ = rl.save_history(history_path);
                break;
            },
            _ => break,
        }
    }
}