use tree_sitter::Language;

use crate::core::LanguageId;

pub struct GrammarSpec {
    pub language: Language,
    pub highlights_query: &'static str,
}

pub fn grammar_for(lang: LanguageId) -> Option<GrammarSpec> {
    match lang {
        LanguageId::Rust => Some(GrammarSpec {
            language: tree_sitter_rust::LANGUAGE.into(),
            highlights_query: tree_sitter_rust::HIGHLIGHTS_QUERY,
        }),
        LanguageId::Python => Some(GrammarSpec {
            language: tree_sitter_python::LANGUAGE.into(),
            highlights_query: tree_sitter_python::HIGHLIGHTS_QUERY,
        }),
        LanguageId::JavaScript => Some(GrammarSpec {
            language: tree_sitter_javascript::LANGUAGE.into(),
            highlights_query: tree_sitter_javascript::HIGHLIGHT_QUERY,
        }),
        LanguageId::TypeScript => Some(GrammarSpec {
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            highlights_query: tree_sitter_typescript::HIGHLIGHTS_QUERY,
        }),
        LanguageId::Json => Some(GrammarSpec {
            language: tree_sitter_json::LANGUAGE.into(),
            highlights_query: tree_sitter_json::HIGHLIGHTS_QUERY,
        }),
        LanguageId::Toml => Some(GrammarSpec {
            language: tree_sitter_toml_ng::LANGUAGE.into(),
            highlights_query: tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        }),
        LanguageId::C => Some(GrammarSpec {
            language: tree_sitter_c::LANGUAGE.into(),
            highlights_query: tree_sitter_c::HIGHLIGHT_QUERY,
        }),
        LanguageId::Markdown => Some(GrammarSpec {
            language: tree_sitter_md::LANGUAGE.into(),
            highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        }),
        LanguageId::Php => Some(GrammarSpec {
            language: tree_sitter_php::LANGUAGE_PHP.into(),
            highlights_query: tree_sitter_php::HIGHLIGHTS_QUERY,
        }),
        LanguageId::PlainText
        | LanguageId::Yaml
        | LanguageId::DotEnv
        | LanguageId::Conf
        | LanguageId::Shell
        | LanguageId::Dockerfile => None,
    }
}
