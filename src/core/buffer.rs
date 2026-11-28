use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ropey::Rope;

use crate::core::selection::{Selection, SelectionSet};
use crate::core::{LanguageId, TextPoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    InsertChar,
    Delete,
    Other,
}

#[derive(Debug, Clone)]
pub struct TextChange {
    pub start_char: usize,
    pub old_end_char: usize,
    pub new_text: String,
    pub old_text: String,
}

/// Byte/Point delta recorded for incremental Tree-sitter tree updates.
#[derive(Debug, Clone, Copy)]
pub struct EditDelta {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: TextPoint,
    pub old_end_point: TextPoint,
    pub new_end_point: TextPoint,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub changes: Vec<TextChange>,
    pub before_selections: SelectionSet,
    pub after_selections: SelectionSet,
    pub timestamp_ms: u64,
    pub kind: EditKind,
}

#[derive(Debug, Default, Clone)]
pub struct UndoHistory {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
}

const COALESCE_WINDOW_MS: u64 = 450;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl UndoHistory {
    pub fn push(&mut self, tx: Transaction) {
        self.redo_stack.clear();
        if let Some(last) = self.undo_stack.last_mut() {
            let can_coalesce = last.kind == tx.kind
                && matches!(tx.kind, EditKind::InsertChar | EditKind::Delete)
                && last.changes.len() == 1
                && tx.changes.len() == 1
                && tx.timestamp_ms.saturating_sub(last.timestamp_ms) <= COALESCE_WINDOW_MS;

            if can_coalesce {
                let prev = &mut last.changes[0];
                let next = &tx.changes[0];
                if tx.kind == EditKind::InsertChar
                    && prev.old_text.is_empty()
                    && next.old_text.is_empty()
                    && next.start_char == prev.start_char + prev.new_text.chars().count()
                {
                    prev.new_text.push_str(&next.new_text);
                    last.after_selections = tx.after_selections;
                    last.timestamp_ms = tx.timestamp_ms;
                    return;
                }
                if tx.kind == EditKind::Delete
                    && prev.new_text.is_empty()
                    && next.new_text.is_empty()
                    && next.old_end_char == prev.start_char
                {
                    let mut combined = next.old_text.clone();
                    combined.push_str(&prev.old_text);
                    prev.start_char = next.start_char;
                    prev.old_text = combined;
                    last.after_selections = tx.after_selections;
                    last.timestamp_ms = tx.timestamp_ms;
                    return;
                }
            }
        }
        self.undo_stack.push(tx);
        if self.undo_stack.len() > 2000 {
            self.undo_stack.remove(0);
        }
    }

    pub fn pop_undo(&mut self) -> Option<Transaction> {
        self.undo_stack.pop()
    }

    pub fn push_redo(&mut self, tx: Transaction) {
        self.redo_stack.push(tx);
    }

    pub fn pop_redo(&mut self) -> Option<Transaction> {
        self.redo_stack.pop()
    }

    pub fn push_undo_raw(&mut self, tx: Transaction) {
        self.undo_stack.push(tx);
    }
}

pub struct Document {
    pub id: u64,
    pub path: Option<PathBuf>,
    pub rope: Rope,
    pub version: i32,
    pub dirty: bool,
    pub selections: SelectionSet,
    pub history: UndoHistory,
    pub language: LanguageId,
    pub pending_deltas: Vec<EditDelta>,
    /// Desired visual column for vertical cursor movement.
    pub preferred_col: Option<usize>,
}

impl Document {
    pub fn new_empty(id: u64) -> Self {
        Self {
            id,
            path: None,
            rope: Rope::new(),
            version: 0,
            dirty: false,
            selections: SelectionSet::default(),
            history: UndoHistory::default(),
            language: LanguageId::PlainText,
            pending_deltas: Vec::new(),
            preferred_col: None,
        }
    }

    pub fn from_str(id: u64, text: &str, language: LanguageId) -> Self {
        Self {
            id,
            path: None,
            rope: Rope::from_str(text),
            version: 0,
            dirty: false,
            selections: SelectionSet::default(),
            history: UndoHistory::default(),
            language,
            pending_deltas: Vec::new(),
            preferred_col: None,
        }
    }

    pub fn from_file(id: u64, path: PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        let language = LanguageId::from_path(&path);
        Ok(Self {
            id,
            path: Some(path),
            rope: Rope::from_str(&content),
            version: 0,
            dirty: false,
            selections: SelectionSet::default(),
            history: UndoHistory::default(),
            language,
            pending_deltas: Vec::new(),
            preferred_col: None,
        })
    }

    pub fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("untitled-{}", self.id))
    }

    pub fn save(&mut self) -> io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Document has no file path",
            ));
        };
        self.save_to(&path)
    }

    pub fn save_as(&mut self, path: PathBuf) -> io::Result<()> {
        self.save_to(&path)?;
        self.language = LanguageId::from_path(&path);
        self.path = Some(path);
        Ok(())
    }

    /// Reloads content from disk if it differs from the current rope.
    /// Returns `Ok(true)` if the buffer actually changed, `Ok(false)` if already identical.
    pub fn reload_from_disk(&mut self) -> io::Result<bool> {
        let Some(path) = self.path.clone() else {
            return Ok(false);
        };
        let disk_content = fs::read_to_string(&path)?;
        if self.rope.len_bytes() == disk_content.len() && self.rope == disk_content.as_str() {
            return Ok(false);
        }
        self.rope = Rope::from_str(&disk_content);
        self.selections.normalize(self.rope.len_chars());
        self.pending_deltas.clear();
        self.version += 1;
        self.dirty = false;
        self.preferred_col = None;
        Ok(true)
    }

    fn save_to(&mut self, path: &Path) -> io::Result<()> {
        let file = fs::File::create(path)?;
        let mut writer = io::BufWriter::new(file);
        self.rope.write_to(&mut writer)?;
        self.dirty = false;
        Ok(())
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let clamped = char_idx.min(self.rope.len_chars());
        let line = self.rope.char_to_line(clamped);
        let line_start = self.rope.line_to_char(line);
        (line, clamped - line_start)
    }

    pub fn char_to_byte_point(&self, char_idx: usize) -> TextPoint {
        let clamped = char_idx.min(self.rope.len_chars());
        let row = self.rope.char_to_line(clamped);
        let line_start_byte = self.rope.line_to_byte(row);
        let byte_idx = self.rope.char_to_byte(clamped);
        TextPoint {
            row,
            column: byte_idx - line_start_byte,
        }
    }

    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        let max_line = self.rope.len_lines().saturating_sub(1);
        let clamped_line = line.min(max_line);
        let line_start = self.rope.line_to_char(clamped_line);
        let line_len = self.line_len_chars_without_newline(clamped_line);
        line_start + col.min(line_len)
    }

    pub fn line_len_chars_without_newline(&self, line: usize) -> usize {
        if line >= self.rope.len_lines() {
            return 0;
        }
        let slice = self.rope.line(line);
        let mut len = slice.len_chars();
        if len > 0 && slice.char(len - 1) == '\n' {
            len -= 1;
            if len > 0 && slice.char(len - 1) == '\r' {
                len -= 1;
            }
        }
        len
    }

    pub fn line_string_without_newline(&self, line: usize) -> String {
        if line >= self.rope.len_lines() {
            return String::new();
        }
        let mut s = self.rope.line(line).to_string();
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        s
    }

    /// Applies non-overlapping edits (in any order) atomically, updating Tree-sitter deltas and Undo history.
    pub fn apply_edits(
        &mut self,
        mut edits: Vec<(Range<usize>, String)>,
        new_selections: SelectionSet,
        kind: EditKind,
    ) {
        if edits.is_empty() {
            self.selections = new_selections;
            self.selections.normalize(self.rope.len_chars());
            return;
        }

        let before_selections = self.selections.clone();
        // Sort descending by start char so applying edits doesn't shift earlier offsets.
        edits.sort_by_key(|a| std::cmp::Reverse(a.0.start));

        let mut changes = Vec::with_capacity(edits.len());
        for (range, new_text) in edits {
            let max_c = self.rope.len_chars();
            let start = range.start.min(max_c);
            let end = range.end.min(max_c).max(start);

            let old_text = self.rope.slice(start..end).to_string();
            let delta = self.apply_single_raw(start, end, &new_text);
            self.pending_deltas.push(delta);

            changes.push(TextChange {
                start_char: start,
                old_end_char: end,
                new_text,
                old_text,
            });
        }

        let mut after_selections = new_selections;
        after_selections.normalize(self.rope.len_chars());
        self.selections = after_selections.clone();
        self.version += 1;
        self.dirty = true;
        self.preferred_col = None;

        self.history.push(Transaction {
            changes,
            before_selections,
            after_selections,
            timestamp_ms: now_ms(),
            kind,
        });
    }

    fn apply_single_raw(&mut self, start_char: usize, end_char: usize, new_text: &str) -> EditDelta {
        let start_byte = self.rope.char_to_byte(start_char);
        let old_end_byte = self.rope.char_to_byte(end_char);
        let start_point = self.char_to_byte_point(start_char);
        let old_end_point = self.char_to_byte_point(end_char);

        if start_char < end_char {
            self.rope.remove(start_char..end_char);
        }
        if !new_text.is_empty() {
            self.rope.insert(start_char, new_text);
        }

        let new_end_char = start_char + new_text.chars().count();
        let new_end_byte = self.rope.char_to_byte(new_end_char);
        let new_end_point = self.char_to_byte_point(new_end_char);

        EditDelta {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_point,
            old_end_point,
            new_end_point,
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some(tx) = self.history.pop_undo() else {
            return false;
        };
        // Changes were stored in descending order (the order they were applied).
        // To undo, apply their inverses in reverse order (ascending).
        for ch in tx.changes.iter().rev() {
            let new_len = ch.new_text.chars().count();
            let delta =
                self.apply_single_raw(ch.start_char, ch.start_char + new_len, &ch.old_text);
            self.pending_deltas.push(delta);
        }
        self.selections = tx.before_selections.clone();
        self.selections.normalize(self.rope.len_chars());
        self.version += 1;
        self.dirty = true;
        self.preferred_col = None;
        self.history.push_redo(tx);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(tx) = self.history.pop_redo() else {
            return false;
        };
        for ch in &tx.changes {
            let delta = self.apply_single_raw(ch.start_char, ch.old_end_char, &ch.new_text);
            self.pending_deltas.push(delta);
        }
        self.selections = tx.after_selections.clone();
        self.selections.normalize(self.rope.len_chars());
        self.version += 1;
        self.dirty = true;
        self.preferred_col = None;
        self.history.push_undo_raw(tx);
        true
    }

    pub fn drain_deltas(&mut self) -> Vec<EditDelta> {
        std::mem::take(&mut self.pending_deltas)
    }

    pub fn set_cursor_at_line_col(&mut self, line: usize, col: usize) {
        let pos = self.line_col_to_char(line, col);
        self.selections
            .set_single(Selection::caret(pos), self.rope.len_chars());
        self.preferred_col = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_insert_undo_redo_roundtrip() {
        let mut doc = Document::from_str(1, "Xin chào thế giới", LanguageId::PlainText);
        doc.apply_edits(
            vec![(4..8, "Chào".to_string())],
            SelectionSet::single(Selection::caret(8)),
            EditKind::Other,
        );
        assert_eq!(doc.rope.to_string(), "Xin Chào thế giới");
        assert!(doc.undo());
        assert_eq!(doc.rope.to_string(), "Xin chào thế giới");
        assert!(doc.redo());
        assert_eq!(doc.rope.to_string(), "Xin Chào thế giới");
    }

    #[test]
    fn coalesces_typing_into_single_undo() {
        let mut doc = Document::new_empty(1);
        for (i, ch) in ['a', 'b', 'c'].into_iter().enumerate() {
            doc.apply_edits(
                vec![(i..i, ch.to_string())],
                SelectionSet::single(Selection::caret(i + 1)),
                EditKind::InsertChar,
            );
        }
        assert_eq!(doc.rope.to_string(), "abc");
        assert!(doc.undo());
        assert_eq!(doc.rope.to_string(), "");
    }

    #[test]
    fn crlf_and_utf8_point_conversions() {
        let doc = Document::from_str(1, "Xin chào\r\nDraco\n", LanguageId::PlainText);
        assert_eq!(doc.line_string_without_newline(0), "Xin chào");
        assert_eq!(doc.line_len_chars_without_newline(0), 8);
        assert_eq!(doc.line_string_without_newline(1), "Draco");
        // 'à' is 2 bytes in UTF-8, so byte column at char index 8 ("Xin chào") is 9 bytes
        let pt = doc.char_to_byte_point(8);
        assert_eq!(pt, TextPoint { row: 0, column: 9 });
    }

    #[test]
    fn save_as_and_reload_updates_language_and_dirty_flag() {
        let path = std::env::temp_dir().join(format!(
            "draco_unit_buf_{}.rs",
            std::process::id()
        ));
        let mut doc = Document::from_str(1, "fn main() {}\n", LanguageId::PlainText);
        doc.dirty = true;
        doc.save_as(path.clone()).unwrap();
        assert!(!doc.dirty);
        assert_eq!(doc.language, LanguageId::Rust);

        let loaded = Document::from_file(2, path.clone()).unwrap();
        assert_eq!(loaded.rope.to_string(), "fn main() {}\n");
        assert_eq!(loaded.language, LanguageId::Rust);
        assert!(!loaded.dirty);
        let _ = std::fs::remove_file(path);
    }
}
