use std::collections::HashSet;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::history::SearchDirection;
use rustyline::{Helper, Hinter, Validator, Context};
use std::borrow::Cow;
use rustyline::highlight::{Highlighter, CmdKind};

// Hapus Highlighter dari sini
#[derive(Helper, Validator, Hinter)] 
pub struct MelisaHelper {
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,
    
    // Hapus #[rustyline(Highlighter)] dari sini
    pub highlighter: MatchingBracketHighlighter, 
    
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    pub file_completer: FilenameCompleter,
}

impl Highlighter for MelisaHelper {
    // Mewarnai teks asli yang sedang diketik
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    // PERUBAHAN DI SINI: ganti `forced: bool` menjadi `kind: CmdKind`
    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, kind)
    }

    // Mewarnai teks saran (hint) dari history menjadi abu-abu
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }
}

impl Completer for MelisaHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        // 1. Autocomplete file untuk command cd atau jika mengandung path
        // Kita tetap menggunakan logic file_completer bawaan
        if line.starts_with("cd ") || line[..pos].contains('/') {
            return self.file_completer.complete(line, pos, ctx);
        }

        // 2. Ambil SELURUH kalimat dari awal sampai posisi kursor saat ini
        let prefix = &line[..pos];
        
        let mut seen = HashSet::new();
        let history = ctx.history();
        let mut suggest = Vec::new();

        // 3. Cari di history berdasarkan prefix keseluruhan, bukan per kata
        for i in (0..history.len()).rev() {
            if let Ok(Some(entry)) = history.get(i, SearchDirection::Forward) {
                // Cek apakah history diawali dengan teks yang sedang diketik
                if entry.entry.starts_with(prefix) && seen.insert(entry.entry.to_string()) {
                    suggest.push(Pair {
                        display: entry.entry.to_string(),
                        replacement: entry.entry.to_string(),
                    });
                }
            }
            if suggest.len() >= 10 { break; }
        }

        if !suggest.is_empty() {
            // PENTING: Kembalikan index 0 karena kita me-replace SELURUH baris,
            // bukan cuma kata terakhirnya saja.
            Ok((0, suggest))
        } else {
            // Fallback ke file completer atau hapus extract_word di sini jika tidak butuh
            self.file_completer.complete(line, pos, ctx)
        }
    }
}