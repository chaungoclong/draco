use draco::core::buffer::Document;
use draco::core::editing::{self, MoveDir};
use draco::core::selection::Selection;
use draco::core::LanguageId;

#[test]
fn auto_indent_preserves_and_increases_after_openers() {
    let mut doc = Document::from_str(1, "def foo():", LanguageId::Python);
    doc.set_cursor_at_line_col(0, 10);
    editing::insert_newline(&mut doc);
    assert_eq!(doc.rope.to_string(), "def foo():\n    ");

    editing::insert_text(&mut doc, "if True:");
    editing::insert_newline(&mut doc);
    assert_eq!(doc.rope.to_string(), "def foo():\n    if True:\n        ");
}

#[test]
fn auto_close_pairs_skip_over_and_delete_pair() {
    let mut doc = Document::new_empty(1);
    for (open, pair) in [("(", "()"), ("[", "[]"), ("{", "{}"), ("'", "''")] {
        let mut d = Document::new_empty(1);
        editing::insert_text(&mut d, open);
        assert_eq!(d.rope.to_string(), pair);
        assert_eq!(d.selections.primary(), Selection::caret(1));
        editing::backspace(&mut d);
        assert_eq!(d.rope.to_string(), "");
    }

    editing::insert_text(&mut doc, "[");
    editing::insert_text(&mut doc, "]");
    assert_eq!(doc.rope.to_string(), "[]");
    assert_eq!(doc.selections.primary(), Selection::caret(2));
}

#[test]
fn delete_forward_single_and_multi_cursor() {
    let mut doc = Document::from_str(1, "abc\ndef", LanguageId::PlainText);
    doc.set_cursor_at_line_col(0, 0);
    editing::add_cursor_vertical(&mut doc, true);
    editing::delete_forward(&mut doc);
    assert_eq!(doc.rope.to_string(), "bc\nef");
}

#[test]
fn toggle_comment_across_rust_python_php_and_plaintext() {
    for (lang, expected) in [
        (LanguageId::Rust, "// let a = 1;"),
        (LanguageId::Python, "# let a = 1;"),
        (LanguageId::Php, "// let a = 1;"),
        (LanguageId::PlainText, "let a = 1;"),
    ] {
        let mut doc = Document::from_str(1, "let a = 1;", lang);
        editing::toggle_comment(&mut doc);
        assert_eq!(doc.rope.to_string(), expected, "failed for {lang:?}");
        if lang != LanguageId::PlainText {
            editing::toggle_comment(&mut doc);
            assert_eq!(doc.rope.to_string(), "let a = 1;");
        }
    }
}

#[test]
fn move_lines_boundaries_and_duplicate_block() {
    let mut doc = Document::from_str(1, "A\nB\nC", LanguageId::PlainText);
    // Moving line 0 up is a no-op
    doc.set_cursor_at_line_col(0, 0);
    editing::move_lines(&mut doc, true);
    assert_eq!(doc.rope.to_string(), "A\nB\nC");

    // Move line 0 down -> B\nA\nC
    editing::move_lines(&mut doc, false);
    assert_eq!(doc.rope.to_string(), "B\nA\nC");

    // Moving last line down is a no-op
    doc.set_cursor_at_line_col(2, 0);
    editing::move_lines(&mut doc, false);
    assert_eq!(doc.rope.to_string(), "B\nA\nC");

    // Duplicate line 2
    editing::duplicate_lines(&mut doc);
    assert_eq!(doc.rope.to_string(), "B\nA\nC\nC");
}

#[test]
fn page_navigation_extend_selection_and_selected_text_fallback() {
    let mut content = String::new();
    for i in 0..40 {
        content.push_str(&format!("row_{i:02}\n"));
    }
    let mut doc = Document::from_str(1, &content, LanguageId::PlainText);
    // Empty selection returns whole line on selected_text (VS Code line copy behavior)
    assert_eq!(editing::selected_text(&doc), "row_00\n");

    editing::move_cursors(&mut doc, MoveDir::PageDown(15), false);
    assert_eq!(doc.char_to_line_col(doc.selections.primary().head).0, 15);

    editing::move_cursors(&mut doc, MoveDir::LineEnd, true);
    assert_eq!(editing::selected_text(&doc), "row_15");

    editing::move_cursors(&mut doc, MoveDir::PageUp(10), false);
    assert_eq!(doc.char_to_line_col(doc.selections.primary().head).0, 5);
}

#[test]
fn bracket_matching_handles_unmatched_and_all_types() {
    let rope = ropey::Rope::from_str("let v = [{(1 + 2)}];");
    assert_eq!(editing::find_matching_bracket(&rope, 8), Some((8, 18))); // [ .. ]
    assert_eq!(editing::find_matching_bracket(&rope, 9), Some((9, 17))); // { .. }
    assert_eq!(editing::find_matching_bracket(&rope, 10), Some((10, 16))); // ( .. )

    let broken = ropey::Rope::from_str("fn broken(a: i32 {");
    assert_eq!(editing::find_matching_bracket(&broken, 9), None);
}
