use tokio::fs; 
use rustyline::{Editor, Config};
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;

use crate::cli::color_text::{RED, YELLOW, GREEN, RESET, BOLD};
use crate::cli::helper::MelisaHelper;
use crate::cli::prompt::Prompt;
use crate::cli::executor::{execute_command, ExecResult};
use crate::cli::prompt::reset_history;

/// Primary entry point for the MELISA interactive shell.
/// Initializes the REPL environment, loads user configuration, and bridges
/// the synchronous terminal interface with the asynchronous execution engine.
pub async fn melisa() {
    // 1. Initialize Context & Path Resolution
    let p_info = Prompt::new();
    
    // Align the history storage with the system directories established by the bash installer
    let melisa_dir = format!("{}/.local/share/melisa", p_info.home);
    let history_path = format!("{}/history.txt", melisa_dir);

    // Ensure the global MELISA directory exists asynchronously
    let _ = fs::create_dir_all(&melisa_dir).await; 

    // 2. Configure the Rustyline Editor
    let config = Config::builder()
        .history_ignore_dups(true).unwrap_or_default() // Prevent consecutive duplicate commands
        .auto_add_history(false) // We manually control history insertion for security and async sync
        .build();

    // Instantiate the editor. Panic safely with a clear message if terminal allocation fails.
    let mut rl: Editor<MelisaHelper, FileHistory> = Editor::with_config(config)
        .expect("FATAL: Failed to allocate terminal interface for Rustyline Editor");

    // Inject the custom MELISA helper (Autocompletion, Highlighting, Validation)
    rl.set_helper(Some(MelisaHelper {
        hinter: HistoryHinter {},
        highlighter: MatchingBracketHighlighter::new(),
        validator: MatchingBracketValidator::new(),
        file_completer: FilenameCompleter::new(),
    }));

    // Load previous session history if it exists
    let _ = rl.load_history(&history_path);

    println!("{}Authenticated as {}. Secure session granted.{}", BOLD, p_info.user, RESET);

    // 3. The Core Execution Loop
    loop {
        let prompt_str = p_info.build();

        // Note: rustyline::readline blocks the current thread while waiting for user input.
        // This is necessary for raw terminal mode handling.
        match rl.readline(&prompt_str) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() { continue; }

                // --- DISPATCH TO ASYNCHRONOUS EXECUTOR ---
                match execute_command(input, &p_info.user, &p_info.home).await {
                    ExecResult::ResetHistory => {
                        // The execute_command safely delegates the reset authority back to the main loop
                        reset_history(&mut rl, &history_path).await;
                    },
                    ExecResult::Continue => {
                        // Successfully executed. Register command to RAM and sync delta to disk.
                        let _ = rl.add_history_entry(input);
                        // OPTIMIZATION: append_history writes only the new line, saving massive Disk I/O
                        let _ = rl.append_history(&history_path); 
                    },
                    ExecResult::Break => {
                        // Terminal exit requested. Save state and break the loop.
                        let _ = rl.add_history_entry(input);
                        let _ = rl.append_history(&history_path);
                        break; 
                    },
                    ExecResult::Error(e) => {
                        // Command failed, but we still log it to history so the user can press 'Up' and fix typos
                        let _ = rl.add_history_entry(input);
                        let _ = rl.append_history(&history_path);
                        eprintln!("{}[ERROR]{} {}", RED, RESET, e);
                    }
                }
                // ----------------------------------------
            },
            Err(ReadlineError::Interrupted) => {
                // Triggered by Ctrl+C. Cancel current line but keep session alive.
                println!("{}[CTRL+C] Operation aborted.{}", YELLOW, RESET);
                continue;
            },
            Err(ReadlineError::Eof) => {
                // Triggered by Ctrl+D. Gracefully terminate the session.
                println!("{}[EXIT] Secure session terminated.{}", GREEN, RESET);
                break;
            },
            Err(err) => {
                // Catch-all for fatal terminal I/O errors
                eprintln!("{}[FATAL]{} Readline encountered a critical error: {:?}", RED, RESET, err);
                break;
            }
        }
    }
}