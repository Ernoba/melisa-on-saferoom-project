use std::collections::HashSet;
use rustyline::completion::{Completer, FilenameCompleter, Pair, extract_word};
use rustyline::highlight::MatchingBracketHighlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::history::SearchDirection;
use rustyline::{Helper, Hinter, Highlighter, Validator, Context};

#[derive(Helper, Highlighter, Validator, Hinter)]
pub struct MelisaHelper {
    #[rustyline(Hinter)]
    pub hinter: HistoryHinter,
    #[rustyline(Highlighter)]
    pub highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    pub validator: MatchingBracketValidator,
    pub file_completer: FilenameCompleter,
}

impl Completer for MelisaHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, word) = extract_word(line, pos, None, |c| "|& ".contains(c));
        
        // Autocomplete file untuk command cd atau jika mengandung path
        if line.starts_with("cd ") || line.contains('/') {
            return self.file_completer.complete(line, pos, ctx);
        }

        // Suggestion dari history (maksimal 10)
        let mut seen = HashSet::new();
        let history = ctx.history();
        let mut suggest = Vec::new();

        for i in (0..history.len()).rev() {
            if let Ok(Some(entry)) = history.get(i, SearchDirection::Forward) {
                if entry.entry.starts_with(word) && seen.insert(entry.entry.to_string()) {
                    suggest.push(Pair {
                        display: entry.entry.to_string(),
                        replacement: entry.entry.to_string(),
                    });
                }
            }
            if suggest.len() >= 10 { break; }
        }

        if !suggest.is_empty() {
            Ok((start, suggest))
        } else {
            self.file_completer.complete(line, pos, ctx)
        }
    }
}