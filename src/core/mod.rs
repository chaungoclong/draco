pub mod buffer;
pub mod editing;
pub mod search;
pub mod selection;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    PlainText,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Json,
    Toml,
    C,
    Markdown,
    Php,
    Yaml,
    DotEnv,
    Conf,
    Shell,
    Dockerfile,
}

impl LanguageId {
    pub fn from_path(path: &Path) -> Self {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if file_name == ".env" || file_name.starts_with(".env.") {
            return Self::DotEnv;
        }
        if file_name == "dockerfile" || file_name.starts_with("dockerfile.") {
            return Self::Dockerfile;
        }
        if matches!(
            file_name.as_str(),
            ".editorconfig" | ".gitconfig" | ".npmrc" | ".htaccess" | "makefile"
        ) {
            return Self::Conf;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("rs") => Self::Rust,
            Some("py") | Some("pyi") => Self::Python,
            Some("js") | Some("mjs") | Some("cjs") | Some("jsx") => Self::JavaScript,
            Some("ts") | Some("mts") | Some("cts") | Some("tsx") => Self::TypeScript,
            Some("json") | Some("jsonc") => Self::Json,
            Some("toml") => Self::Toml,
            Some("c") | Some("h") | Some("cpp") | Some("cc") | Some("hpp") => Self::C,
            Some("md") | Some("markdown") => Self::Markdown,
            Some("php") | Some("phtml") => Self::Php,
            Some("yml") | Some("yaml") => Self::Yaml,
            Some("env") => Self::DotEnv,
            Some("conf") | Some("ini") | Some("cfg") | Some("cnf") | Some("properties") => {
                Self::Conf
            }
            Some("sh") | Some("bash") | Some("zsh") => Self::Shell,
            Some("dockerfile") => Self::Dockerfile,
            _ => Self::PlainText,
        }
    }

    pub fn from_fence_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "rs" | "rust" => Self::Rust,
            "py" | "python" => Self::Python,
            "js" | "javascript" | "jsx" => Self::JavaScript,
            "ts" | "typescript" | "tsx" => Self::TypeScript,
            "json" | "jsonc" => Self::Json,
            "toml" => Self::Toml,
            "c" | "cpp" | "c++" => Self::C,
            "md" | "markdown" => Self::Markdown,
            "php" => Self::Php,
            "yml" | "yaml" => Self::Yaml,
            "env" | "dotenv" => Self::DotEnv,
            "conf" | "ini" | "cfg" | "nginx" | "properties" => Self::Conf,
            "sh" | "bash" | "zsh" | "shell" => Self::Shell,
            "dockerfile" | "docker" => Self::Dockerfile,
            _ => Self::PlainText,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::PlainText => "Plain Text",
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Json => "JSON",
            Self::Toml => "TOML",
            Self::C => "C/C++",
            Self::Markdown => "Markdown",
            Self::Php => "PHP",
            Self::Yaml => "YAML",
            Self::DotEnv => "DotEnv",
            Self::Conf => "Config / INI",
            Self::Shell => "Shell",
            Self::Dockerfile => "Dockerfile",
        }
    }

    pub fn line_comment(self) -> Option<&'static str> {
        match self {
            Self::Rust | Self::JavaScript | Self::TypeScript | Self::C | Self::Php => Some("//"),
            Self::Python
            | Self::Toml
            | Self::Yaml
            | Self::DotEnv
            | Self::Conf
            | Self::Shell
            | Self::Dockerfile => Some("#"),
            Self::PlainText | Self::Json | Self::Markdown => None,
        }
    }

    pub fn lsp_language_id(self) -> &'static str {
        match self {
            Self::PlainText => "plaintext",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Json => "json",
            Self::Toml => "toml",
            Self::C => "c",
            Self::Markdown => "markdown",
            Self::Php => "php",
            Self::Yaml => "yaml",
            Self::DotEnv => "dotenv",
            Self::Conf => "ini",
            Self::Shell => "shellscript",
            Self::Dockerfile => "dockerfile",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextPoint {
    pub row: usize,
    pub column: usize,
}
