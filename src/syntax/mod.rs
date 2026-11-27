pub mod ast;
pub mod config_lexer;
pub mod languages;
pub mod symbols;

use std::ops::Range;

use ropey::Rope;
use streaming_iterator::StreamingIterator;
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, Tree};

use crate::core::buffer::EditDelta;
use crate::core::LanguageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Plain,
    Comment,
    Keyword,
    String,
    Number,
    Constant,
    Type,
    Function,
    Macro,
    Variable,
    Property,
    Operator,
    Punctuation,
    Attribute,
    Heading,
}

impl TokenKind {
    pub fn from_capture_name(name: &str) -> Self {
        let base = name.split('.').next().unwrap_or(name);
        match base {
            "comment" => Self::Comment,
            "keyword" | "repeat" | "conditional" | "include" | "storageclass" => Self::Keyword,
            "string" | "character" | "escape" => Self::String,
            "number" | "float" | "boolean" => Self::Number,
            "constant" => Self::Constant,
            "type" | "constructor" | "module" | "namespace" => Self::Type,
            "function" | "method" => Self::Function,
            "macro" => Self::Macro,
            "property" | "field" => Self::Property,
            "operator" => Self::Operator,
            "punctuation" => Self::Punctuation,
            "attribute" | "label" | "tag" => Self::Attribute,
            "text" | "markup" | "title" => Self::Heading,
            "variable" | "parameter" => Self::Variable,
            _ => Self::Plain,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub byte_range: Range<usize>,
    pub kind: TokenKind,
}

pub struct SyntaxState {
    language: LanguageId,
    parser: Option<Parser>,
    query: Option<Query>,
    capture_kinds: Vec<TokenKind>,
    tree: Option<Tree>,
    last_version: i32,
}

impl SyntaxState {
    pub fn new(language: LanguageId) -> Self {
        let mut state = Self {
            language,
            parser: None,
            query: None,
            capture_kinds: Vec::new(),
            tree: None,
            last_version: -1,
        };
        state.init_grammar(language);
        state
    }

    fn init_grammar(&mut self, language: LanguageId) {
        self.language = language;
        self.tree = None;
        self.last_version = -1;
        self.parser = None;
        self.query = None;
        self.capture_kinds.clear();

        let Some(spec) = languages::grammar_for(language) else {
            return;
        };
        let mut parser = Parser::new();
        if parser.set_language(&spec.language).is_err() {
            return;
        }
        if let Ok(query) = Query::new(&spec.language, spec.highlights_query) {
            self.capture_kinds = query
                .capture_names()
                .iter()
                .map(|n| TokenKind::from_capture_name(n))
                .collect();
            self.query = Some(query);
        }
        self.parser = Some(parser);
    }

    /// Incrementally updates the syntax tree from document deltas.
    pub fn sync(&mut self, rope: &Rope, language: LanguageId, version: i32, deltas: &[EditDelta]) {
        if self.language != language {
            self.init_grammar(language);
        }
        if self.last_version == version && self.tree.is_some() {
            return;
        }
        let Some(parser) = self.parser.as_mut() else {
            self.last_version = version;
            return;
        };

        if let Some(tree) = self.tree.as_mut() {
            if deltas.is_empty() && self.last_version != -1 {
                self.last_version = version;
                return;
            }
            for d in deltas {
                tree.edit(&InputEdit {
                    start_byte: d.start_byte,
                    old_end_byte: d.old_end_byte,
                    new_end_byte: d.new_end_byte,
                    start_position: Point::new(d.start_point.row, d.start_point.column),
                    old_end_position: Point::new(d.old_end_point.row, d.old_end_point.column),
                    new_end_position: Point::new(d.new_end_point.row, d.new_end_point.column),
                });
            }
        }

        let len_bytes = rope.len_bytes();
        let old_tree = if deltas.is_empty() {
            None
        } else {
            self.tree.as_ref()
        };

        let new_tree = parser.parse_with(
            &mut |byte_offset: usize, _position: Point| -> &[u8] {
                if byte_offset >= len_bytes {
                    return &[];
                }
                let (chunk, chunk_byte_idx, _, _) = rope.chunk_at_byte(byte_offset);
                &chunk.as_bytes()[byte_offset - chunk_byte_idx..]
            },
            old_tree,
        );
        self.tree = new_tree;
        self.last_version = version;
    }

    /// Queries highlight spans ONLY within `byte_range` (e.g., visible viewport lines).
    pub fn highlights_in_range(&self, rope: &Rope, byte_range: Range<usize>) -> Vec<HighlightSpan> {
        if config_lexer::is_config_lexer_language(self.language) {
            return config_lexer::highlight_config_in_range(self.language, rope, byte_range);
        }
        let (Some(tree), Some(query)) = (&self.tree, &self.query) else {
            return Vec::new();
        };
        if byte_range.start >= byte_range.end {
            return Vec::new();
        }

        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(byte_range.clone());

        let provider = RopeTextProvider { rope };
        let mut captures = cursor.captures(query, tree.root_node(), provider);
        let mut spans = Vec::new();

        while let Some((m, capture_idx)) = captures.next() {
            let cap = m.captures[*capture_idx];
            let node_range = cap.node.byte_range();
            let start = node_range.start.max(byte_range.start);
            let end = node_range.end.min(byte_range.end);
            if start < end {
                let kind = self
                    .capture_kinds
                    .get(cap.index as usize)
                    .copied()
                    .unwrap_or(TokenKind::Plain);
                if kind != TokenKind::Plain {
                    spans.push(HighlightSpan {
                        byte_range: start..end,
                        kind,
                    });
                }
            }
        }
        spans
    }

    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    pub fn extract_symbols(&self, rope: &Rope) -> Vec<symbols::DocumentSymbol> {
        match &self.tree {
            Some(tree) => symbols::extract_symbols(tree, rope, self.language),
            None => Vec::new(),
        }
    }
}

struct RopeTextProvider<'a> {
    rope: &'a Rope,
}

impl<'a> tree_sitter::TextProvider<&'a [u8]> for RopeTextProvider<'a> {
    type I = RopeChunkIter<'a>;

    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        let range = node.byte_range();
        let start = range.start.min(self.rope.len_bytes());
        let end = range.end.min(self.rope.len_bytes());
        RopeChunkIter {
            rope: self.rope,
            pos: start,
            end,
        }
    }
}

struct RopeChunkIter<'a> {
    rope: &'a Rope,
    pos: usize,
    end: usize,
}

impl<'a> Iterator for RopeChunkIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.end {
            return None;
        }
        let (chunk, chunk_start, _, _) = self.rope.chunk_at_byte(self.pos);
        let offset = self.pos - chunk_start;
        let available = chunk.len() - offset;
        let take = available.min(self.end - self.pos);
        self.pos += take;
        Some(&chunk.as_bytes()[offset..offset + take])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buffer::Document;
    use crate::core::editing::insert_text;
    use std::time::Instant;

    #[test]
    fn incremental_rust_highlighting_works() {
        let mut doc = Document::from_str(1, "fn main() {\n    let x = 42;\n}\n", LanguageId::Rust);
        let mut syn = SyntaxState::new(LanguageId::Rust);
        let deltas = doc.drain_deltas();
        syn.sync(&doc.rope, doc.language, doc.version, &deltas);

        let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
        assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword));
        assert!(spans.iter().any(|s| s.kind == TokenKind::Function));

        doc.set_cursor_at_line_col(1, 15);
        insert_text(&mut doc, " + 1");
        let deltas2 = doc.drain_deltas();
        syn.sync(&doc.rope, doc.language, doc.version, &deltas2);
        let spans2 = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
        assert!(!spans2.is_empty());
    }

    #[test]
    fn visible_range_query_is_fast_on_large_file() {
        let mut big = String::with_capacity(2_500_000);
        for i in 0..50_000 {
            big.push_str(&format!("fn func_{i}(x: i32) -> i32 {{ x + {i} }}\n"));
        }
        let doc = Document::from_str(1, &big, LanguageId::Rust);
        let mut syn = SyntaxState::new(LanguageId::Rust);
        syn.sync(&doc.rope, doc.language, doc.version, &[]);

        let start_byte = doc.rope.line_to_byte(25_000);
        let end_byte = doc.rope.line_to_byte(25_060);
        let t0 = Instant::now();
        let spans = syn.highlights_in_range(&doc.rope, start_byte..end_byte);
        let elapsed = t0.elapsed();
        assert!(!spans.is_empty());
        assert!(
            elapsed.as_millis() < 50,
            "Visible range highlight took too long: {elapsed:?}"
        );
    }

    #[test]
    fn all_supported_languages_produce_highlights() {
        let samples = [
            (LanguageId::Rust, "pub fn add(a: i32) -> i32 { a + 1 }"),
            (LanguageId::Python, "def greet(name):\n    return 'hi ' + name\n"),
            (LanguageId::JavaScript, "const x = (a) => { return a * 2; };"),
            (LanguageId::TypeScript, "interface User { id: number; name: string; }"),
            (LanguageId::Json, r#"{"name": "draco", "fast": true, "ver": 1}"#),
            (LanguageId::Toml, "[package]\nname = \"draco\"\nversion = \"0.1.0\"\n"),
            (LanguageId::C, "int main(void) { return 0; }"),
            (LanguageId::Markdown, "# Draco\n\n- fast\n- lightweight\n"),
            (LanguageId::Php, "<?php\nfunction greet($name) {\n    return 'Hello ' . $name;\n}\n"),
        ];

        for (lang, src) in samples {
            let doc = Document::from_str(1, src, lang);
            let mut syn = SyntaxState::new(lang);
            syn.sync(&doc.rope, doc.language, doc.version, &[]);
            let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
            assert!(
                !spans.is_empty(),
                "Expected non-empty highlights for {lang:?}"
            );
        }
    }
}
