use tokio::fs; 
use rustyline::{Editor, Config};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;

use crate::cli::color_text::{RED, RESET, BOLD};
use crate::cli::helper::MelisaHelper;
use crate::cli::prompt::Prompt;
use crate::cli::executor::{execute_command, ExecResult};
use crate::cli::prompt::reset_history;

// 1. Ubah menjadi pub async fn
pub async fn melisa() {
    // 2. Operasi folder secara async
    let _ = fs::create_dir_all("data").await; 
    let history_path = "data/history.txt";

    let config = Config::builder()
        .history_ignore_dups(true).ok()
        .map(|b| b.build())
        .unwrap_or_default();

    // Rustyline sendiri masih sinkron, tapi kita menjalankannya di dalam context async
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
        let prompt_str = p_info.build();

        match rl.readline(&prompt_str) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }

                // --- DI SINI TEMPATNYA ---
                // Kita panggil executor, lalu tangani hasilnya di sini
                match execute_command(input, &p_info.user, &p_info.home).await {
                    ExecResult::ResetHistory => {
                        // Karena 'rl' dan 'history_path' ada di scope fungsi melisa() ini,
                        // kita bisa memanggil fungsi reset_history dengan aman.
                        reset_history(&mut rl, history_path).await;
                    },
                    ExecResult::Continue => {
                        // Command biasa berhasil, simpan ke history
                        let _ = rl.add_history_entry(input);
                        let _ = rl.save_history(history_path); 
                    },
                    ExecResult::Break => {
                        let _ = rl.save_history(history_path);
                        break; // Keluar dari loop (exit melisa)
                    },
                    ExecResult::Error(e) => {
                        eprintln!("{RED}[ERROR]{RESET} {}", e);
                    }
                }
                // -------------------------
            },
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            _ => break,
        }
    }
}