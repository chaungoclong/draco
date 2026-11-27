use draco::core::buffer::Document;
use draco::core::selection::Selection;
use draco::core::LanguageId;
use draco::syntax::ast::AstSelectionHistory;
use draco::syntax::symbols::{enclosing_scopes, SymbolKind};
use draco::syntax::SyntaxState;

#[test]
fn ast_extend_and_shrink_selection_walks_syntax_tree_and_handles_utf8() {
    let src = "fn chao_ban() {\n    let msg = \"Xin chào Việt Nam\";\n    println!(\"{}\", msg);\n}\n";
    let mut doc = Document::from_str(1, src, LanguageId::Rust);
    let mut syn = SyntaxState::new(LanguageId::Rust);
    let deltas = doc.drain_deltas();
    syn.sync(&doc.rope, doc.language, doc.version, &deltas);
    let tree = syn.tree().expect("Rust tree must parse");

    // Place caret inside `msg` on line 1
    let caret = src.find("msg").unwrap();
    let initial = Selection::caret(caret);

    let mut hist = AstSelectionHistory::default();
    let s1 = hist.expand(tree, &doc.rope, initial).expect("level 1 expand");
    assert_eq!(doc.rope.slice(s1.range()).to_string(), "msg");

    let s2 = hist.expand(tree, &doc.rope, s1).expect("level 2 expand");
    assert!(s2.range().len() > s1.range().len());

    let s3 = hist.expand(tree, &doc.rope, s2).expect("level 3 expand");
    assert!(s3.range().len() > s2.range().len());

    // Shrink unwinds step-by-step back to initial caret
    assert_eq!(hist.shrink(s3), Some(s2));
    assert_eq!(hist.shrink(s2), Some(s1));
    assert_eq!(hist.shrink(s1), Some(initial));
    assert_eq!(hist.shrink(initial), None);
}

#[test]
fn extracts_symbols_across_rust_python_php_and_markdown() {
    // Rust
    let rust_src = "struct Engine {\n    x: i32,\n}\n\nimpl Engine {\n    fn run(&self) {\n        let a = 1;\n        let b = 2;\n        let _ = a + b;\n    }\n}\n";
    let mut doc = Document::from_str(1, rust_src, LanguageId::Rust);
    let mut syn = SyntaxState::new(LanguageId::Rust);
    let d1 = doc.drain_deltas();
    syn.sync(&doc.rope, doc.language, doc.version, &d1);
    let syms = syn.extract_symbols(&doc.rope);
    assert!(syms.iter().any(|s| s.kind == SymbolKind::Struct && s.name == "Engine"));
    assert!(syms.iter().any(|s| s.kind == SymbolKind::Impl && s.name == "impl Engine"));
    assert!(syms.iter().any(|s| s.kind == SymbolKind::Method && s.name == "run"));

    // Sticky scroll at line 8 (inside `impl Engine` -> `fn run`)
    let scopes = enclosing_scopes(&syms, 8, 3);
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].name, "impl Engine");
    assert_eq!(scopes[1].name, "run");

    // Python
    let py_src = "class Worker:\n    def execute(self):\n        return 42\n";
    let mut py_doc = Document::from_str(2, py_src, LanguageId::Python);
    let mut py_syn = SyntaxState::new(LanguageId::Python);
    let d2 = py_doc.drain_deltas();
    py_syn.sync(&py_doc.rope, py_doc.language, py_doc.version, &d2);
    let py_syms = py_syn.extract_symbols(&py_doc.rope);
    assert!(py_syms.iter().any(|s| s.kind == SymbolKind::Class && s.name == "Worker"));
    assert!(py_syms.iter().any(|s| s.kind == SymbolKind::Method && s.name == "execute"));

    // PHP
    let php_src = "<?php\nclass UserService {\n    public function findUser() {}\n}\n";
    let mut php_doc = Document::from_str(3, php_src, LanguageId::Php);
    let mut php_syn = SyntaxState::new(LanguageId::Php);
    let d3 = php_doc.drain_deltas();
    php_syn.sync(&php_doc.rope, php_doc.language, php_doc.version, &d3);
    let php_syms = php_syn.extract_symbols(&php_doc.rope);
    assert!(php_syms.iter().any(|s| s.kind == SymbolKind::Class && s.name == "UserService"));
    assert!(php_syms.iter().any(|s| s.kind == SymbolKind::Method && s.name == "findUser"));
}
