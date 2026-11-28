use std::ops::Range;

use ropey::Rope;

use crate::core::buffer::{Document, EditKind};
use crate::core::selection::{Selection, SelectionSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDir {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
    PageUp(usize),
    PageDown(usize),
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn closing_pair(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        _ => None,
    }
}

fn is_closing_bracket(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '"' | '\'')
}

pub fn insert_text(doc: &mut Document, text: &str) {
    if text.is_empty() {
        return;
    }

    let single_char = if text.chars().count() == 1 {
        text.chars().next()
    } else {
        None
    };

    let sels = doc.selections.selections().to_vec();
    let mut edits: Vec<(Range<usize>, String)> = Vec::with_capacity(sels.len());
    let mut new_sels: Vec<Selection> = Vec::with_capacity(sels.len());
    let mut delta: isize = 0;

    for sel in &sels {
        let range = sel.range();
        let removed_len = range.end - range.start;

        // Smart skip over existing closing char when typing single closing delimiter
        if let Some(ch) = single_char {
            if sel.is_empty()
                && is_closing_bracket(ch)
                && range.start < doc.len_chars()
                && doc.rope.char(range.start) == ch
            {
                let next_pos = ((range.start + 1) as isize + delta) as usize;
                new_sels.push(Selection::caret(next_pos));
                continue;
            }
            // Auto-close pair when typing opening delimiter
            if let Some(close) = closing_pair(ch) {
                if !sel.is_empty() {
                    // Wrap selection in pair
                    let inner = doc.rope.slice(range.clone()).to_string();
                    let inserted = format!("{ch}{inner}{close}");
                    let ins_len = inserted.chars().count();
                    let new_start = (range.start as isize + delta + 1) as usize;
                    let new_end = new_start + inner.chars().count();
                    edits.push((range, inserted));
                    new_sels.push(Selection::new(new_start, new_end));
                    delta += ins_len as isize - removed_len as isize;
                    continue;
                } else {
                    let next_char = if range.start < doc.len_chars() {
                        Some(doc.rope.char(range.start))
                    } else {
                        None
                    };
                    let should_close = match next_char {
                        None => true,
                        Some(nc) => nc.is_whitespace() || matches!(nc, ')' | ']' | '}' | ',' | ';' | ':'),
                    };
                    if should_close {
                        let inserted = format!("{ch}{close}");
                        let caret = (range.start as isize + delta + 1) as usize;
                        edits.push((range, inserted));
                        new_sels.push(Selection::caret(caret));
                        delta += 2 - removed_len as isize;
                        continue;
                    }
                }
            }
        }

        let ins_len = text.chars().count();
        let caret = (range.start as isize + delta + ins_len as isize) as usize;
        edits.push((range, text.to_string()));
        new_sels.push(Selection::caret(caret));
        delta += ins_len as isize - removed_len as isize;
    }

    let kind = if single_char.is_some() && sels.len() == 1 && sels[0].is_empty() {
        EditKind::InsertChar
    } else {
        EditKind::Other
    };
    let primary = doc.selections.primary_index();
    let new_len = (doc.len_chars() as isize + delta).max(0) as usize;
    let new_set = SelectionSet::from_vec(new_sels, primary, new_len);
    doc.apply_edits(edits, new_set, kind);
}

pub fn insert_newline(doc: &mut Document) {
    let sels = doc.selections.selections().to_vec();
    let mut edits = Vec::with_capacity(sels.len());
    let mut new_sels = Vec::with_capacity(sels.len());
    let mut delta: isize = 0;

    for sel in &sels {
        let range = sel.range();
        let removed_len = range.end - range.start;
        let line_idx = doc.rope.char_to_line(range.start);
        let line_str = doc.line_string_without_newline(line_idx);
        let indent: String = line_str
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();

        let prev_char = if range.start > 0 {
            Some(doc.rope.char(range.start - 1))
        } else {
            None
        };
        let next_char = if range.end < doc.len_chars() {
            Some(doc.rope.char(range.end))
        } else {
            None
        };

        let increases_indent = matches!(prev_char, Some('{' | '(' | '[' | ':'));
        let between_brackets = matches!(
            (prev_char, next_char),
            (Some('{'), Some('}')) | (Some('('), Some(')')) | (Some('['), Some(']'))
        );

        let (inserted, caret_offset) = if between_brackets {
            let inner_indent = format!("{indent}    ");
            let s = format!("\n{inner_indent}\n{indent}");
            let offset = 1 + inner_indent.chars().count();
            (s, offset)
        } else if increases_indent {
            let inner_indent = format!("{indent}    ");
            let s = format!("\n{inner_indent}");
            let offset = s.chars().count();
            (s, offset)
        } else {
            let s = format!("\n{indent}");
            let offset = s.chars().count();
            (s, offset)
        };

        let ins_len = inserted.chars().count();
        let caret = (range.start as isize + delta + caret_offset as isize) as usize;
        edits.push((range, inserted));
        new_sels.push(Selection::caret(caret));
        delta += ins_len as isize - removed_len as isize;
    }

    let primary = doc.selections.primary_index();
    let new_len = (doc.len_chars() as isize + delta).max(0) as usize;
    let new_set = SelectionSet::from_vec(new_sels, primary, new_len);
    doc.apply_edits(edits, new_set, EditKind::Other);
}

pub fn backspace(doc: &mut Document) {
    let sels = doc.selections.selections().to_vec();
    let mut edits = Vec::with_capacity(sels.len());
    let mut new_sels = Vec::with_capacity(sels.len());
    let mut delta: isize = 0;

    for sel in &sels {
        if !sel.is_empty() {
            let r = sel.range();
            let removed = r.end - r.start;
            let caret = (r.start as isize + delta) as usize;
            edits.push((r, String::new()));
            new_sels.push(Selection::caret(caret));
            delta -= removed as isize;
        } else if sel.head > 0 {
            let prev = doc.rope.char(sel.head - 1);
            let next = if sel.head < doc.len_chars() {
                Some(doc.rope.char(sel.head))
            } else {
                None
            };
            // Delete matching empty pair if cursor is right between them
            if closing_pair(prev) == next && next.is_some() {
                let start = sel.head - 1;
                let end = sel.head + 1;
                let caret = (start as isize + delta) as usize;
                edits.push((start..end, String::new()));
                new_sels.push(Selection::caret(caret));
                delta -= 2;
            } else {
                let start = sel.head - 1;
                let caret = (start as isize + delta) as usize;
                edits.push((start..sel.head, String::new()));
                new_sels.push(Selection::caret(caret));
                delta -= 1;
            }
        } else {
            new_sels.push(Selection::caret(0));
        }
    }

    let kind = if sels.len() == 1 && sels[0].is_empty() {
        EditKind::Delete
    } else {
        EditKind::Other
    };
    let primary = doc.selections.primary_index();
    let new_len = (doc.len_chars() as isize + delta).max(0) as usize;
    let new_set = SelectionSet::from_vec(new_sels, primary, new_len);
    doc.apply_edits(edits, new_set, kind);
}

pub fn delete_forward(doc: &mut Document) {
    let sels = doc.selections.selections().to_vec();
    let mut edits = Vec::with_capacity(sels.len());
    let mut new_sels = Vec::with_capacity(sels.len());
    let mut delta: isize = 0;

    for sel in &sels {
        if !sel.is_empty() {
            let r = sel.range();
            let removed = r.end - r.start;
            let caret = (r.start as isize + delta) as usize;
            edits.push((r, String::new()));
            new_sels.push(Selection::caret(caret));
            delta -= removed as isize;
        } else if sel.head < doc.len_chars() {
            let caret = (sel.head as isize + delta) as usize;
            edits.push((sel.head..sel.head + 1, String::new()));
            new_sels.push(Selection::caret(caret));
            delta -= 1;
        } else {
            let caret = (sel.head as isize + delta) as usize;
            new_sels.push(Selection::caret(caret));
        }
    }

    let primary = doc.selections.primary_index();
    let new_len = (doc.len_chars() as isize + delta).max(0) as usize;
    let new_set = SelectionSet::from_vec(new_sels, primary, new_len);
    doc.apply_edits(edits, new_set, EditKind::Other);
}

pub fn move_cursors(doc: &mut Document, dir: MoveDir, extend: bool) {
    let vertical = matches!(
        dir,
        MoveDir::Up | MoveDir::Down | MoveDir::PageUp(_) | MoveDir::PageDown(_)
    );
    let primary_col = if vertical {
        doc.preferred_col.unwrap_or_else(|| {
            let (_, c) = doc.char_to_line_col(doc.selections.primary().head);
            c
        })
    } else {
        0
    };

    let sels = doc.selections.selections().to_vec();
    let max_c = doc.len_chars();
    let mut new_sels = Vec::with_capacity(sels.len());

    for sel in sels {
        let new_head = match dir {
            MoveDir::Left => {
                if !extend && !sel.is_empty() {
                    sel.start()
                } else {
                    sel.head.saturating_sub(1)
                }
            }
            MoveDir::Right => {
                if !extend && !sel.is_empty() {
                    sel.end()
                } else {
                    (sel.head + 1).min(max_c)
                }
            }
            MoveDir::WordLeft => prev_word_boundary(&doc.rope, sel.head),
            MoveDir::WordRight => next_word_boundary(&doc.rope, sel.head),
            MoveDir::LineStart => {
                let (line, col) = doc.char_to_line_col(sel.head);
                let line_str = doc.line_string_without_newline(line);
                let first_non_ws = line_str
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .count();
                let line_start = doc.rope.line_to_char(line);
                if col == first_non_ws {
                    line_start
                } else {
                    line_start + first_non_ws
                }
            }
            MoveDir::LineEnd => {
                let (line, _) = doc.char_to_line_col(sel.head);
                doc.rope.line_to_char(line) + doc.line_len_chars_without_newline(line)
            }
            MoveDir::DocStart => 0,
            MoveDir::DocEnd => max_c,
            MoveDir::Up => {
                let (line, col) = doc.char_to_line_col(sel.head);
                let target_col = if doc.selections.len() == 1 {
                    primary_col
                } else {
                    col
                };
                if line == 0 {
                    0
                } else {
                    doc.line_col_to_char(line - 1, target_col)
                }
            }
            MoveDir::Down => {
                let (line, col) = doc.char_to_line_col(sel.head);
                let target_col = if doc.selections.len() == 1 {
                    primary_col
                } else {
                    col
                };
                if line + 1 >= doc.len_lines() {
                    max_c
                } else {
                    doc.line_col_to_char(line + 1, target_col)
                }
            }
            MoveDir::PageUp(n) => {
                let (line, col) = doc.char_to_line_col(sel.head);
                let target_col = if doc.selections.len() == 1 {
                    primary_col
                } else {
                    col
                };
                doc.line_col_to_char(line.saturating_sub(n), target_col)
            }
            MoveDir::PageDown(n) => {
                let (line, col) = doc.char_to_line_col(sel.head);
                let target_col = if doc.selections.len() == 1 {
                    primary_col
                } else {
                    col
                };
                let target_line = (line + n).min(doc.len_lines().saturating_sub(1));
                doc.line_col_to_char(target_line, target_col)
            }
        };

        if extend {
            new_sels.push(Selection::new(sel.anchor, new_head));
        } else {
            new_sels.push(Selection::caret(new_head));
        }
    }

    let primary = doc.selections.primary_index();
    doc.selections = SelectionSet::from_vec(new_sels, primary, max_c);
    doc.preferred_col = if vertical { Some(primary_col) } else { None };
}

fn prev_word_boundary(rope: &Rope, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let mut i = pos - 1;
    while i > 0 && rope.char(i).is_whitespace() {
        i -= 1;
    }
    let word = is_word_char(rope.char(i));
    while i > 0 && is_word_char(rope.char(i - 1)) == word && !rope.char(i - 1).is_whitespace() {
        i -= 1;
    }
    i
}

fn next_word_boundary(rope: &Rope, pos: usize) -> usize {
    let len = rope.len_chars();
    if pos >= len {
        return len;
    }
    let mut i = pos;
    let word = is_word_char(rope.char(i));
    let ws = rope.char(i).is_whitespace();
    while i < len {
        let c = rope.char(i);
        if ws {
            if !c.is_whitespace() {
                break;
            }
        } else if is_word_char(c) != word || c.is_whitespace() {
            break;
        }
        i += 1;
    }
    i
}

pub fn word_range_at(rope: &Rope, pos: usize) -> Range<usize> {
    let len = rope.len_chars();
    if len == 0 {
        return 0..0;
    }
    let idx = pos.min(len.saturating_sub(1));
    let ch = rope.char(idx);
    if !is_word_char(ch) {
        return idx..(idx + 1).min(len);
    }
    let mut start = idx;
    while start > 0 && is_word_char(rope.char(start - 1)) {
        start -= 1;
    }
    let mut end = idx + 1;
    while end < len && is_word_char(rope.char(end)) {
        end += 1;
    }
    start..end
}

pub fn select_all(doc: &mut Document) {
    let len = doc.len_chars();
    doc.selections.set_single(Selection::new(0, len), len);
}

pub fn selected_text(doc: &Document) -> String {
    let mut parts = Vec::new();
    for sel in doc.selections.selections() {
        if !sel.is_empty() {
            parts.push(doc.rope.slice(sel.range()).to_string());
        } else {
            let (line, _) = doc.char_to_line_col(sel.head);
            parts.push(doc.rope.line(line).to_string());
        }
    }
    parts.join("\n")
}

pub fn indent(doc: &mut Document) {
    let all_empty = doc.selections.selections().iter().all(|s| s.is_empty());
    if all_empty {
        insert_text(doc, "    ");
        return;
    }
    let lines = affected_lines(doc);
    let mut edits = Vec::new();
    for line in &lines {
        let start = doc.rope.line_to_char(*line);
        edits.push((start..start, "    ".to_string()));
    }
    let new_sels = doc
        .selections
        .selections()
        .iter()
        .map(|s| {
            let a_line = doc.rope.char_to_line(s.anchor);
            let h_line = doc.rope.char_to_line(s.head);
            let a_shift = lines.iter().filter(|&&l| l <= a_line).count() * 4;
            let h_shift = lines.iter().filter(|&&l| l <= h_line).count() * 4;
            Selection::new(s.anchor + a_shift, s.head + h_shift)
        })
        .collect();
    let new_len = doc.len_chars() + lines.len() * 4;
    let primary = doc.selections.primary_index();
    doc.apply_edits(edits, SelectionSet::from_vec(new_sels, primary, new_len), EditKind::Other);
}

pub fn outdent(doc: &mut Document) {
    let lines = affected_lines(doc);
    let mut edits = Vec::new();
    let mut removed_per_line: Vec<(usize, usize)> = Vec::new();

    for &line in &lines {
        let start = doc.rope.line_to_char(line);
        let text = doc.line_string_without_newline(line);
        let remove = if text.starts_with('\t') {
            1
        } else {
            text.chars().take(4).take_while(|&c| c == ' ').count()
        };
        if remove > 0 {
            edits.push((start..start + remove, String::new()));
            removed_per_line.push((line, remove));
        }
    }
    if edits.is_empty() {
        return;
    }

    let total_removed: usize = removed_per_line.iter().map(|(_, r)| *r).sum();
    let new_sels = doc
        .selections
        .selections()
        .iter()
        .map(|s| {
            let a_line = doc.rope.char_to_line(s.anchor);
            let h_line = doc.rope.char_to_line(s.head);
            let a_shift: usize = removed_per_line
                .iter()
                .filter(|(l, _)| *l <= a_line)
                .map(|(_, r)| *r)
                .sum();
            let h_shift: usize = removed_per_line
                .iter()
                .filter(|(l, _)| *l <= h_line)
                .map(|(_, r)| *r)
                .sum();
            Selection::new(
                s.anchor.saturating_sub(a_shift),
                s.head.saturating_sub(h_shift),
            )
        })
        .collect();
    let new_len = doc.len_chars().saturating_sub(total_removed);
    let primary = doc.selections.primary_index();
    doc.apply_edits(edits, SelectionSet::from_vec(new_sels, primary, new_len), EditKind::Other);
}

pub fn toggle_comment(doc: &mut Document) {
    let Some(prefix) = doc.language.line_comment() else {
        return;
    };
    let lines = affected_lines(doc);
    let all_commented = lines.iter().all(|&l| {
        let s = doc.line_string_without_newline(l);
        let trimmed = s.trim_start();
        trimmed.is_empty() || trimmed.starts_with(prefix)
    });

    let mut edits = Vec::new();
    let mut line_deltas: Vec<(usize, isize)> = Vec::new();

    for &line in &lines {
        let s = doc.line_string_without_newline(line);
        let ws_len = s.chars().take_while(|c| *c == ' ' || *c == '\t').count();
        let line_start = doc.rope.line_to_char(line);
        let content_start = line_start + ws_len;
        let trimmed = &s[s.char_indices().nth(ws_len).map(|(b, _)| b).unwrap_or(s.len())..];

        if all_commented {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let mut remove_len = prefix.chars().count();
                if rest.starts_with(' ') {
                    remove_len += 1;
                }
                edits.push((content_start..content_start + remove_len, String::new()));
                line_deltas.push((line, -(remove_len as isize)));
            }
        } else {
            let ins = format!("{prefix} ");
            let ins_len = ins.chars().count() as isize;
            edits.push((content_start..content_start, ins));
            line_deltas.push((line, ins_len));
        }
    }

    if edits.is_empty() {
        return;
    }
    let total_delta: isize = line_deltas.iter().map(|(_, d)| *d).sum();
    let new_sels = doc
        .selections
        .selections()
        .iter()
        .map(|s| {
            let a_line = doc.rope.char_to_line(s.anchor);
            let h_line = doc.rope.char_to_line(s.head);
            let a_shift: isize = line_deltas
                .iter()
                .filter(|(l, _)| *l <= a_line)
                .map(|(_, d)| *d)
                .sum();
            let h_shift: isize = line_deltas
                .iter()
                .filter(|(l, _)| *l <= h_line)
                .map(|(_, d)| *d)
                .sum();
            Selection::new(
                (s.anchor as isize + a_shift).max(0) as usize,
                (s.head as isize + h_shift).max(0) as usize,
            )
        })
        .collect();
    let new_len = (doc.len_chars() as isize + total_delta).max(0) as usize;
    let primary = doc.selections.primary_index();
    doc.apply_edits(
        edits,
        SelectionSet::from_vec(new_sels, primary, new_len),
        EditKind::Other,
    );
}

pub fn duplicate_lines(doc: &mut Document) {
    let lines = affected_lines(doc);
    let Some(&last_line) = lines.last() else {
        return;
    };
    let Some(&first_line) = lines.first() else {
        return;
    };
    let mut block = String::new();
    for l in first_line..=last_line {
        block.push('\n');
        block.push_str(&doc.line_string_without_newline(l));
    }
    let insert_pos = doc.rope.line_to_char(last_line) + doc.line_len_chars_without_newline(last_line);
    let block_len = block.chars().count();
    let new_sels = doc
        .selections
        .selections()
        .iter()
        .map(|s| Selection::new(s.anchor + block_len, s.head + block_len))
        .collect();
    let new_len = doc.len_chars() + block_len;
    let primary = doc.selections.primary_index();
    doc.apply_edits(
        vec![(insert_pos..insert_pos, block)],
        SelectionSet::from_vec(new_sels, primary, new_len),
        EditKind::Other,
    );
}

pub fn move_lines(doc: &mut Document, up: bool) {
    let (line, col) = doc.char_to_line_col(doc.selections.primary().head);
    let total = doc.len_lines();
    if up && line == 0 {
        return;
    }
    if !up && line + 1 >= total {
        return;
    }
    let other = if up { line - 1 } else { line + 1 };
    let first = line.min(other);
    let second = line.max(other);

    let first_str = doc.line_string_without_newline(first);
    let second_str = doc.line_string_without_newline(second);
    let start = doc.rope.line_to_char(first);
    let end = doc.rope.line_to_char(second) + doc.line_len_chars_without_newline(second);
    let swapped = format!("{second_str}\n{first_str}");

    let target_line = other;
    let new_line_start = if target_line == first {
        start
    } else {
        start + second_str.chars().count() + 1
    };
    let new_col = col.min(doc.line_len_chars_without_newline(line));
    let caret = new_line_start + new_col;
    doc.apply_edits(
        vec![(start..end, swapped)],
        SelectionSet::single(Selection::caret(caret)),
        EditKind::Other,
    );
}

pub fn add_cursor_vertical(doc: &mut Document, below: bool) {
    let p = doc.selections.primary();
    let (line, col) = doc.char_to_line_col(p.head);
    let target_line = if below {
        if line + 1 >= doc.len_lines() {
            return;
        }
        line + 1
    } else {
        if line == 0 {
            return;
        }
        line - 1
    };
    let target_pos = doc.line_col_to_char(target_line, col);
    doc.selections.add(Selection::caret(target_pos), doc.len_chars());
}

/// VS Code-style `Ctrl+D`: selects word under primary cursor if empty, otherwise adds next occurrence.
pub fn select_next_occurrence(doc: &mut Document) {
    let p = doc.selections.primary();
    if p.is_empty() {
        let range = word_range_at(&doc.rope, p.head);
        if range.start < range.end {
            doc.selections
                .set_single(Selection::new(range.start, range.end), doc.len_chars());
        }
        return;
    }

    let needle = doc.rope.slice(p.range()).to_string();
    if needle.is_empty() {
        return;
    }
    let full = doc.rope.to_string();
    let start_byte = doc.rope.char_to_byte(p.end());
    let found_byte = full[start_byte..]
        .find(&needle)
        .map(|rel| start_byte + rel)
        .or_else(|| full[..start_byte].find(&needle));

    if let Some(byte_idx) = found_byte {
        let start_char = doc.rope.byte_to_char(byte_idx);
        let end_char = start_char + needle.chars().count();
        doc.selections
            .add(Selection::new(start_char, end_char), doc.len_chars());
    }
}

/// Finds the matching bracket pair if the cursor is adjacent to a bracket.
pub fn find_matching_bracket(rope: &Rope, cursor: usize) -> Option<(usize, usize)> {
    let len = rope.len_chars();
    if len == 0 {
        return None;
    }
    let candidates = [
        if cursor < len { Some(cursor) } else { None },
        if cursor > 0 { Some(cursor - 1) } else { None },
    ];

    for idx in candidates.into_iter().flatten() {
        let ch = rope.char(idx);
        let (target, forward) = match ch {
            '(' => (')', true),
            '[' => (']', true),
            '{' => ('}', true),
            ')' => ('(', false),
            ']' => ('[', false),
            '}' => ('{', false),
            _ => continue,
        };

        let mut depth = 0usize;
        let max_scan = 20_000usize;
        if forward {
            for i in idx..len.min(idx + max_scan) {
                let c = rope.char(i);
                if c == ch {
                    depth += 1;
                } else if c == target {
                    depth -= 1;
                    if depth == 0 {
                        return Some((idx, i));
                    }
                }
            }
        } else {
            let low = idx.saturating_sub(max_scan);
            for i in (low..=idx).rev() {
                let c = rope.char(i);
                if c == ch {
                    depth += 1;
                } else if c == target {
                    depth -= 1;
                    if depth == 0 {
                        return Some((i, idx));
                    }
                }
            }
        }
    }
    None
}

fn affected_lines(doc: &Document) -> Vec<usize> {
    let mut lines = Vec::new();
    for sel in doc.selections.selections() {
        let start_line = doc.rope.char_to_line(sel.start());
        let mut end_line = doc.rope.char_to_line(sel.end());
        if !sel.is_empty() && sel.end() == doc.rope.line_to_char(end_line) && end_line > start_line {
            end_line -= 1;
        }
        for l in start_line..=end_line {
            if !lines.contains(&l) {
                lines.push(l);
            }
        }
    }
    lines.sort_unstable();
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::LanguageId;

    #[test]
    fn multi_cursor_insert_and_ctrl_d() {
        let mut doc = Document::from_str(1, "foo bar foo baz foo", LanguageId::Rust);
        doc.set_cursor_at_line_col(0, 1);
        select_next_occurrence(&mut doc); // selects first "foo"
        select_next_occurrence(&mut doc); // adds second "foo"
        assert_eq!(doc.selections.len(), 2);
        insert_text(&mut doc, "qux");
        assert_eq!(doc.rope.to_string(), "qux bar qux baz foo");
    }

    #[test]
    fn auto_indent_between_braces() {
        let mut doc = Document::from_str(1, "fn main() {}", LanguageId::Rust);
        doc.set_cursor_at_line_col(0, 11); // between { and }
        insert_newline(&mut doc);
        assert_eq!(doc.rope.to_string(), "fn main() {\n    \n}");
    }

    #[test]
    fn toggle_comment_adds_and_removes() {
        let mut doc = Document::from_str(1, "let a = 1;\nlet b = 2;", LanguageId::Rust);
        select_all(&mut doc);
        toggle_comment(&mut doc);
        assert_eq!(doc.rope.to_string(), "// let a = 1;\n// let b = 2;");
        toggle_comment(&mut doc);
        assert_eq!(doc.rope.to_string(), "let a = 1;\nlet b = 2;");
    }

    #[test]
    fn bracket_matching_nested() {
        let rope = Rope::from_str("fn a(b: Vec<(i32, i32)>) {}");
        assert_eq!(find_matching_bracket(&rope, 4), Some((4, 23)));
        assert_eq!(find_matching_bracket(&rope, 23), Some((4, 23)));
    }

    #[test]
    fn auto_close_skip_over_and_pair_backspace() {
        let mut doc = Document::new_empty(1);
        insert_text(&mut doc, "(");
        assert_eq!(doc.rope.to_string(), "()");
        assert_eq!(doc.selections.primary(), Selection::caret(1));

        // Typing closing ')' skips over existing ')'
        insert_text(&mut doc, ")");
        assert_eq!(doc.rope.to_string(), "()");
        assert_eq!(doc.selections.primary(), Selection::caret(2));

        // Move back inside and backspace deletes whole pair
        move_cursors(&mut doc, MoveDir::Left, false);
        backspace(&mut doc);
        assert_eq!(doc.rope.to_string(), "");
    }

    #[test]
    fn wrap_selection_in_quotes_and_brackets() {
        let mut doc = Document::from_str(1, "hello", LanguageId::Rust);
        select_all(&mut doc);
        insert_text(&mut doc, "\"");
        assert_eq!(doc.rope.to_string(), "\"hello\"");
        assert_eq!(selected_text(&doc), "hello");
    }

    #[test]
    fn indent_outdent_move_and_duplicate_lines() {
        let mut doc = Document::from_str(1, "line1\nline2\nline3", LanguageId::Python);
        select_all(&mut doc);
        indent(&mut doc);
        assert_eq!(doc.rope.to_string(), "    line1\n    line2\n    line3");

        outdent(&mut doc);
        assert_eq!(doc.rope.to_string(), "line1\nline2\nline3");

        doc.set_cursor_at_line_col(1, 2);
        move_lines(&mut doc, true);
        assert_eq!(doc.rope.to_string(), "line2\nline1\nline3");

        duplicate_lines(&mut doc);
        assert_eq!(doc.rope.to_string(), "line2\nline2\nline1\nline3");
    }

    #[test]
    fn cursor_movement_words_lines_and_vertical_preferred_col() {
        let mut doc = Document::from_str(1, "    alpha_1 beta\nhi\n    gamma delta", LanguageId::Rust);
        doc.set_cursor_at_line_col(0, 10);
        // Down to shorter line ("hi"), then down again restores column 10
        move_cursors(&mut doc, MoveDir::Down, false);
        assert_eq!(doc.char_to_line_col(doc.selections.primary().head), (1, 2));
        move_cursors(&mut doc, MoveDir::Down, false);
        assert_eq!(doc.char_to_line_col(doc.selections.primary().head), (2, 10));

        // Smart LineStart toggles between first non-whitespace (col 4) and col 0
        move_cursors(&mut doc, MoveDir::LineStart, false);
        assert_eq!(doc.char_to_line_col(doc.selections.primary().head), (2, 4));
        move_cursors(&mut doc, MoveDir::LineStart, false);
        assert_eq!(doc.char_to_line_col(doc.selections.primary().head), (2, 0));

        // WordRight jumps over spaces/word
        move_cursors(&mut doc, MoveDir::DocStart, false);
        move_cursors(&mut doc, MoveDir::WordRight, false);
        move_cursors(&mut doc, MoveDir::WordRight, false);
        assert_eq!(doc.char_to_line_col(doc.selections.primary().head), (0, 11));
    }

    #[test]
    fn python_comment_toggle_and_vertical_cursors() {
        let mut doc = Document::from_str(1, "x = 1\ny = 2", LanguageId::Python);
        doc.set_cursor_at_line_col(0, 0);
        add_cursor_vertical(&mut doc, true);
        assert_eq!(doc.selections.len(), 2);
        toggle_comment(&mut doc);
        assert_eq!(doc.rope.to_string(), "# x = 1\n# y = 2");
        toggle_comment(&mut doc);
        assert_eq!(doc.rope.to_string(), "x = 1\ny = 2");
    }
}
