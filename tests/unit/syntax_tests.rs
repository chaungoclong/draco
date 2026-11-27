use draco::core::buffer::Document;
use draco::core::editing;
use draco::core::LanguageId;
use draco::syntax::{SyntaxState, TokenKind};

#[test]
fn all_nine_languages_highlight_cleanly_and_plaintext_is_empty() {
    let samples = [
        (LanguageId::Rust, "pub struct Draco { fast: bool }"),
        (LanguageId::Python, "class Editor:\n    def run(self): return 1\n"),
        (LanguageId::JavaScript, "export function sum(a, b) { return a + b; }"),
        (LanguageId::TypeScript, "type Id = string | number; const id: Id = 1;"),
        (LanguageId::Json, r#"{"editor":"draco","plugins":[]}"#),
        (LanguageId::Toml, "[editor]\nauto_save = true\n"),
        (LanguageId::C, "#include <stdio.h>\nint main() { return 0; }"),
        (LanguageId::Markdown, "# Title\n\n> Quote\n"),
        (
            LanguageId::Php,
            "<?php\nclass App {\n    public function run(string $msg): void {\n        echo $msg;\n    }\n}\n",
        ),
    ];

    for (lang, code) in samples {
        let doc = Document::from_str(1, code, lang);
        let mut syn = SyntaxState::new(lang);
        syn.sync(&doc.rope, lang, doc.version, &[]);
        let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
        assert!(!spans.is_empty(), "Expected spans for {lang:?}");
    }

    let plain = Document::from_str(2, "just plain text", LanguageId::PlainText);
    let mut syn_plain = SyntaxState::new(LanguageId::PlainText);
    syn_plain.sync(&plain.rope, LanguageId::PlainText, plain.version, &[]);
    assert!(syn_plain
        .highlights_in_range(&plain.rope, 0..plain.rope.len_bytes())
        .is_empty());
}

#[test]
fn language_switch_and_incremental_undo_sync() {
    let mut doc = Document::from_str(1, "<?php $x = 1;\n", LanguageId::PlainText);
    let mut syn = SyntaxState::new(LanguageId::PlainText);
    syn.sync(&doc.rope, doc.language, doc.version, &[]);
    assert!(syn
        .highlights_in_range(&doc.rope, 0..doc.rope.len_bytes())
        .is_empty());

    // Switch to PHP dynamically (e.g. after Save As index.php)
    doc.language = LanguageId::Php;
    syn.sync(&doc.rope, doc.language, doc.version, &[]);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(!spans.is_empty());

    // Edit + incremental sync + undo + incremental sync
    doc.set_cursor_at_line_col(0, 13);
    editing::insert_text(&mut doc, " function test() {}");
    let d1 = doc.drain_deltas();
    syn.sync(&doc.rope, doc.language, doc.version, &d1);
    let spans2 = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans2.iter().any(|s| s.kind == TokenKind::Keyword));

    assert!(doc.undo());
    let d2 = doc.drain_deltas();
    syn.sync(&doc.rope, doc.language, doc.version, &d2);
    assert!(!syn
        .highlights_in_range(&doc.rope, 0..doc.rope.len_bytes())
        .is_empty());
}
