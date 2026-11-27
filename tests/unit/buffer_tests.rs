use draco::core::buffer::{Document, EditKind};
use draco::core::selection::{Selection, SelectionSet};
use draco::core::{LanguageId, TextPoint};

use crate::common::TempDir;

#[test]
fn empty_document_invariants() {
    let mut doc = Document::new_empty(7);
    assert_eq!(doc.title(), "untitled-7");
    assert_eq!(doc.len_chars(), 0);
    assert_eq!(doc.len_lines(), 1);
    assert!(!doc.dirty);
    assert_eq!(doc.char_to_line_col(0), (0, 0));
    assert_eq!(doc.char_to_line_col(999), (0, 0));
    assert_eq!(doc.line_string_without_newline(0), "");
    assert_eq!(doc.line_string_without_newline(5), "");
    assert!(!doc.undo());
    assert!(!doc.redo());
    assert!(doc.save().is_err());
}

#[test]
fn utf8_vietnamese_and_emoji_offsets_roundtrip() {
    let text = "Xin chào 🐉 Draco\nDòng thứ hai";
    let mut doc = Document::from_str(1, text, LanguageId::PlainText);

    // Line 0: "Xin chào 🐉 Draco" -> 16 chars
    assert_eq!(doc.line_len_chars_without_newline(0), 16);
    assert_eq!(doc.line_string_without_newline(0), "Xin chào 🐉 Draco");
    assert_eq!(doc.line_string_without_newline(1), "Dòng thứ hai");

    // Byte point after "Xin chào " (9 chars, 'à' is 2 bytes -> 10 bytes)
    assert_eq!(doc.char_to_byte_point(9), TextPoint { row: 0, column: 10 });

    // Replace dragon emoji (char 9..10) with "Rust"
    doc.apply_edits(
        vec![(9..10, "Rust".to_string())],
        SelectionSet::single(Selection::caret(13)),
        EditKind::Other,
    );
    assert_eq!(doc.line_string_without_newline(0), "Xin chào Rust Draco");
    assert!(doc.dirty);

    assert!(doc.undo());
    assert_eq!(doc.rope.to_string(), text);
    assert!(doc.redo());
    assert_eq!(doc.line_string_without_newline(0), "Xin chào Rust Draco");
}

#[test]
fn crlf_line_endings_strip_both_cr_and_lf() {
    let doc = Document::from_str(1, "alpha\r\nbeta\r\ngamma", LanguageId::PlainText);
    assert_eq!(doc.len_lines(), 3);
    assert_eq!(doc.line_string_without_newline(0), "alpha");
    assert_eq!(doc.line_string_without_newline(1), "beta");
    assert_eq!(doc.line_string_without_newline(2), "gamma");
    assert_eq!(doc.line_len_chars_without_newline(0), 5);
    assert_eq!(doc.line_col_to_char(1, 99), doc.rope.line_to_char(1) + 4);
}

#[test]
fn multi_step_undo_redo_and_redo_invalidation() {
    let mut doc = Document::from_str(1, "base", LanguageId::PlainText);
    doc.apply_edits(
        vec![(4..4, " one".into())],
        SelectionSet::single(Selection::caret(8)),
        EditKind::Other,
    );
    doc.apply_edits(
        vec![(8..8, " two".into())],
        SelectionSet::single(Selection::caret(12)),
        EditKind::Other,
    );
    assert_eq!(doc.rope.to_string(), "base one two");

    assert!(doc.undo());
    assert_eq!(doc.rope.to_string(), "base one");

    // New edit invalidates redo stack
    doc.apply_edits(
        vec![(8..8, " three".into())],
        SelectionSet::single(Selection::caret(14)),
        EditKind::Other,
    );
    assert!(!doc.redo());
    assert_eq!(doc.rope.to_string(), "base one three");
}

#[test]
fn save_save_as_and_language_detection_for_php_and_rust() {
    let tmp = TempDir::new("buf_save");
    let php_path = tmp.path().join("index.php");

    let mut doc = Document::from_str(1, "<?php echo 'hi';\n", LanguageId::PlainText);
    doc.dirty = true;
    doc.save_as(php_path.clone()).unwrap();
    assert!(!doc.dirty);
    assert_eq!(doc.language, LanguageId::Php);
    assert_eq!(doc.title(), "index.php");

    let loaded = Document::from_file(2, php_path).unwrap();
    assert_eq!(loaded.language, LanguageId::Php);
    assert_eq!(loaded.rope.to_string(), "<?php echo 'hi';\n");
}
