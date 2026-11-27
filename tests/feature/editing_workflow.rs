use draco::core::buffer::Document;
use draco::core::editing;
use draco::core::search::{self, SearchOptions};
use draco::core::LanguageId;
use draco::syntax::{SyntaxState, TokenKind};

use crate::common::TempDir;

#[test]
fn rust_file_edit_highlight_search_replace_undo_save_reload() {
    let tmp = TempDir::new("feat_rust_workflow");
    let file_path = tmp.write_file(
        "sample.rs",
        "fn compute() {\n    let old_val = 10;\n    let sum = old_val + 5;\n}\n",
    );

    let mut doc = Document::from_file(1, file_path.clone()).unwrap();
    assert_eq!(doc.language, LanguageId::Rust);
    assert!(!doc.dirty);

    let mut syntax = SyntaxState::new(doc.language);
    let d0 = doc.drain_deltas();
    syntax.sync(&doc.rope, doc.language, doc.version, &d0);

    // Multi-cursor Ctrl+D rename
    doc.set_cursor_at_line_col(1, 9);
    editing::select_next_occurrence(&mut doc);
    editing::select_next_occurrence(&mut doc);
    assert_eq!(doc.selections.len(), 2);
    editing::insert_text(&mut doc, "new_val");
    assert!(doc.dirty);
    assert!(doc.rope.to_string().contains("let new_val = 10;"));
    assert!(doc.rope.to_string().contains("let sum = new_val + 5;"));

    // Regex replace all + undo
    let opts = SearchOptions {
        case_sensitive: true,
        whole_word: false,
        use_regex: true,
    };
    let count = search::replace_all(&mut doc, r"\b\d+\b", "42", &opts).unwrap();
    assert_eq!(count, 2);
    assert!(doc.rope.to_string().contains("let new_val = 42;"));
    assert!(doc.undo());
    assert!(doc.rope.to_string().contains("let new_val = 10;"));

    // Incremental Tree-sitter sync
    let deltas = doc.drain_deltas();
    syntax.sync(&doc.rope, doc.language, doc.version, &deltas);
    let spans = syntax.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Function));

    // Save & reload
    doc.save().unwrap();
    assert!(!doc.dirty);
    let reloaded = Document::from_file(2, file_path).unwrap();
    assert_eq!(reloaded.rope.to_string(), doc.rope.to_string());
}

#[test]
fn php_file_editing_comment_toggle_and_syntax_workflow() {
    let tmp = TempDir::new("feat_php_workflow");
    let file_path = tmp.write_file(
        "controller.php",
        "<?php\nfunction index($id) {\n    return $id;\n}\n",
    );

    let mut doc = Document::from_file(1, file_path.clone()).unwrap();
    assert_eq!(doc.language, LanguageId::Php);

    let mut syn = SyntaxState::new(doc.language);
    syn.sync(&doc.rope, doc.language, doc.version, &[]);
    assert!(!syn
        .highlights_in_range(&doc.rope, 0..doc.rope.len_bytes())
        .is_empty());

    // Comment line 2 with `// `, save, reload
    doc.set_cursor_at_line_col(2, 4);
    editing::toggle_comment(&mut doc);
    assert_eq!(doc.line_string_without_newline(2), "    // return $id;");
    doc.save().unwrap();

    let reloaded = Document::from_file(2, file_path).unwrap();
    assert_eq!(reloaded.line_string_without_newline(2), "    // return $id;");
}
