use std::ops::Range;

use regex::{Regex, RegexBuilder};
use ropey::Rope;

use crate::core::buffer::{Document, EditKind};
use crate::core::selection::{Selection, SelectionSet};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

fn build_regex(query: &str, opts: &SearchOptions) -> Result<Regex, String> {
    let mut pattern = if opts.use_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    if opts.whole_word {
        pattern = format!(r"\b{pattern}\b");
    }
    RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .build()
        .map_err(|e| e.to_string())
}

/// Finds all matches in `rope`, returning char offset ranges.
pub fn find_all(rope: &Rope, query: &str, opts: &SearchOptions) -> Result<Vec<Range<usize>>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let re = build_regex(query, opts)?;
    let text = rope.to_string();
    let mut out = Vec::new();
    for m in re.find_iter(&text) {
        if m.start() == m.end() {
            continue;
        }
        let start = rope.byte_to_char(m.start());
        let end = rope.byte_to_char(m.end());
        out.push(start..end);
    }
    Ok(out)
}

pub fn find_next(matches: &[Range<usize>], from_char: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    matches
        .iter()
        .position(|r| r.start >= from_char)
        .or(Some(0))
}

pub fn find_prev(matches: &[Range<usize>], from_char: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    matches
        .iter()
        .rposition(|r| r.start < from_char)
        .or(Some(matches.len() - 1))
}

pub fn replace_all(
    doc: &mut Document,
    query: &str,
    replacement: &str,
    opts: &SearchOptions,
) -> Result<usize, String> {
    let matches = find_all(&doc.rope, query, opts)?;
    let count = matches.len();
    if count == 0 {
        return Ok(0);
    }
    let edits: Vec<(Range<usize>, String)> = matches
        .into_iter()
        .map(|r| (r, replacement.to_string()))
        .collect();
    let new_sel = SelectionSet::single(Selection::caret(0));
    doc.apply_edits(edits, new_sel, EditKind::Other);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::LanguageId;

    #[test]
    fn find_and_replace_all_atomic_undo() {
        let mut doc = Document::from_str(1, "alpha Beta alpha BETA", LanguageId::PlainText);
        let opts = SearchOptions {
            case_sensitive: false,
            whole_word: true,
            use_regex: false,
        };
        let found = find_all(&doc.rope, "beta", &opts).unwrap();
        assert_eq!(found.len(), 2);

        let replaced = replace_all(&mut doc, "beta", "gamma", &opts).unwrap();
        assert_eq!(replaced, 2);
        assert_eq!(doc.rope.to_string(), "alpha gamma alpha gamma");
        assert!(doc.undo());
        assert_eq!(doc.rope.to_string(), "alpha Beta alpha BETA");
    }

    #[test]
    fn regex_search_wrap_around_and_invalid_pattern() {
        let rope = Rope::from_str("id_10 foo id_25 bar id_99");
        let opts = SearchOptions {
            case_sensitive: true,
            whole_word: false,
            use_regex: true,
        };
        let matches = find_all(&rope, r"id_\d+", &opts).unwrap();
        assert_eq!(matches.len(), 3);
        assert_eq!(find_next(&matches, 0), Some(0));
        assert_eq!(find_next(&matches, 6), Some(1));
        assert_eq!(find_next(&matches, 25), Some(0)); // wraps around
        assert_eq!(find_prev(&matches, 0), Some(2)); // wraps around

        assert!(find_all(&rope, "[unclosed", &opts).is_err());
    }
}
