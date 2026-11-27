use std::ops::Range;

use ropey::Rope;

use crate::core::LanguageId;
use crate::syntax::{HighlightSpan, TokenKind};

pub fn is_config_lexer_language(lang: LanguageId) -> bool {
    matches!(
        lang,
        LanguageId::Yaml
            | LanguageId::DotEnv
            | LanguageId::Conf
            | LanguageId::Shell
            | LanguageId::Dockerfile
    )
}

/// Fast visible-range lexical highlighter for YAML, .env, INI/CONF, Shell, and Dockerfile.
/// Only scans lines overlapping `byte_range` — O(visible lines), zero external dependencies.
pub fn highlight_config_in_range(
    lang: LanguageId,
    rope: &Rope,
    byte_range: Range<usize>,
) -> Vec<HighlightSpan> {
    if !is_config_lexer_language(lang) || byte_range.start >= byte_range.end {
        return Vec::new();
    }
    let len_bytes = rope.len_bytes();
    let start_b = byte_range.start.min(len_bytes);
    let end_b = byte_range.end.min(len_bytes);
    if start_b >= end_b {
        return Vec::new();
    }

    let first_line = rope.byte_to_line(start_b);
    let last_line = rope.byte_to_line(end_b.saturating_sub(1));
    let mut spans = Vec::new();

    for line_idx in first_line..=last_line {
        let line_start = rope.line_to_byte(line_idx);
        let raw = rope.line(line_idx).to_string();
        let line = raw.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            continue;
        }
        match lang {
            LanguageId::Yaml => lex_yaml_line(line, line_start, &mut spans),
            LanguageId::DotEnv => lex_dotenv_line(line, line_start, &mut spans),
            LanguageId::Conf => lex_conf_line(line, line_start, &mut spans),
            LanguageId::Shell => lex_shell_line(line, line_start, &mut spans),
            LanguageId::Dockerfile => lex_dockerfile_line(line, line_start, &mut spans),
            _ => {}
        }
    }

    spans
}

fn find_unquoted_comment(line: &str, allow_semicolon: bool) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && in_double {
            escaped = true;
            continue;
        }
        if b == b'\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if b == b'"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if !in_single && !in_double {
            let is_hash = b == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace());
            let is_semi = allow_semicolon && b == b';' && (i == 0 || bytes[i - 1].is_ascii_whitespace());
            if is_hash || is_semi {
                return Some(i);
            }
        }
    }
    None
}

fn push_value_tokens(val: &str, base_byte: usize, spans: &mut Vec<HighlightSpan>) {
    let trimmed_start = val.len() - val.trim_start().len();
    let v = val[trimmed_start..].trim_end();
    if v.is_empty() {
        return;
    }
    let start = base_byte + trimmed_start;
    let end = start + v.len();

    if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
        || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
    {
        spans.push(HighlightSpan {
            byte_range: start..end,
            kind: TokenKind::String,
        });
        // Highlight ${VAR} interpolations inside double-quoted strings
        if v.starts_with('"') {
            lex_var_interpolations(v, start, spans);
        }
        return;
    }

    let lower = v.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off" | "null" | "~"
    ) {
        spans.push(HighlightSpan {
            byte_range: start..end,
            kind: TokenKind::Constant,
        });
        return;
    }

    if looks_like_number(v) {
        spans.push(HighlightSpan {
            byte_range: start..end,
            kind: TokenKind::Number,
        });
        return;
    }

    // Highlight any embedded ${VAR} or $VAR inside unquoted values
    if v.contains('$') {
        lex_var_interpolations(v, start, spans);
    } else if v.starts_with('&') || v.starts_with('*') || v.starts_with('!') {
        spans.push(HighlightSpan {
            byte_range: start..end,
            kind: TokenKind::Attribute,
        });
    } else {
        spans.push(HighlightSpan {
            byte_range: start..end,
            kind: TokenKind::String,
        });
    }
}

fn looks_like_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    if s.is_empty() {
        return false;
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit());
    }
    // Allow numeric values with common config units (80, 3.14, 10m, 60s, 512mb)
    let digits_end = s
        .bytes()
        .position(|b| !(b.is_ascii_digit() || b == b'.' || b == b'_'))
        .unwrap_or(s.len());
    if digits_end == 0 {
        return false;
    }
    let suffix = &s[digits_end..];
    suffix.is_empty()
        || matches!(
            suffix.to_ascii_lowercase().as_str(),
            "s" | "ms" | "m" | "h" | "d" | "k" | "kb" | "mb" | "gb" | "%"
        )
}

fn lex_var_interpolations(s: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if let Some(close) = s[i + 2..].find('}') {
                    let end = i + 2 + close + 1;
                    spans.push(HighlightSpan {
                        byte_range: (base + i)..(base + end),
                        kind: TokenKind::Variable,
                    });
                    i = end;
                    continue;
                }
            } else {
                let mut end = i + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > i + 1 {
                    spans.push(HighlightSpan {
                        byte_range: (base + i)..(base + end),
                        kind: TokenKind::Variable,
                    });
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn lex_yaml_line(line: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    let trimmed = line.trim();
    if trimmed == "---" || trimmed == "..." {
        let offset = line.len() - line.trim_start().len();
        spans.push(HighlightSpan {
            byte_range: (base + offset)..(base + offset + trimmed.len()),
            kind: TokenKind::Keyword,
        });
        return;
    }

    let comment_at = find_unquoted_comment(line, false);
    let code = match comment_at {
        Some(idx) => {
            spans.push(HighlightSpan {
                byte_range: (base + idx)..(base + line.len()),
                kind: TokenKind::Comment,
            });
            &line[..idx]
        }
        None => line,
    };

    let indent = code.len() - code.trim_start().len();
    let mut rest = &code[indent..];
    let mut rest_base = base + indent;

    // Leading list marker `- `
    if rest.starts_with("- ") || rest == "-" {
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + 1),
            kind: TokenKind::Punctuation,
        });
        let adv = if rest.len() >= 2 { 2 } else { 1 };
        rest = &rest[adv..];
        rest_base += adv;
    }

    // Key: value
    if let Some(colon) = find_yaml_colon(rest) {
        let key_part = rest[..colon].trim_end();
        if !key_part.is_empty() {
            spans.push(HighlightSpan {
                byte_range: rest_base..(rest_base + key_part.len()),
                kind: TokenKind::Property,
            });
        }
        spans.push(HighlightSpan {
            byte_range: (rest_base + colon)..(rest_base + colon + 1),
            kind: TokenKind::Punctuation,
        });
        let val = &rest[colon + 1..];
        push_value_tokens(val, rest_base + colon + 1, spans);
    } else if !rest.trim().is_empty() {
        push_value_tokens(rest, rest_base, spans);
    }
}

fn find_yaml_colon(s: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\'' && !in_double {
            in_single = !in_single;
        } else if b == b'"' && !in_single {
            in_double = !in_double;
        } else if b == b':' && !in_single && !in_double {
            if i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace() {
                return Some(i);
            }
        }
    }
    None
}

fn lex_dotenv_line(line: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    let comment_at = find_unquoted_comment(line, false);
    let code = match comment_at {
        Some(idx) => {
            spans.push(HighlightSpan {
                byte_range: (base + idx)..(base + line.len()),
                kind: TokenKind::Comment,
            });
            &line[..idx]
        }
        None => line,
    };

    let indent = code.len() - code.trim_start().len();
    let mut rest = &code[indent..];
    let mut rest_base = base + indent;

    if let Some(after) = rest.strip_prefix("export ") {
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + 6),
            kind: TokenKind::Keyword,
        });
        rest = after;
        rest_base += 7;
    }

    if let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim_end();
        if !key.is_empty() {
            spans.push(HighlightSpan {
                byte_range: rest_base..(rest_base + key.len()),
                kind: TokenKind::Property,
            });
        }
        spans.push(HighlightSpan {
            byte_range: (rest_base + eq)..(rest_base + eq + 1),
            kind: TokenKind::Operator,
        });
        push_value_tokens(&rest[eq + 1..], rest_base + eq + 1, spans);
    }
}

fn lex_conf_line(line: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    let comment_at = find_unquoted_comment(line, true);
    let code = match comment_at {
        Some(idx) => {
            spans.push(HighlightSpan {
                byte_range: (base + idx)..(base + line.len()),
                kind: TokenKind::Comment,
            });
            &line[..idx]
        }
        None => line,
    };

    let indent = code.len() - code.trim_start().len();
    let rest = code[indent..].trim_end();
    let rest_base = base + indent;
    if rest.is_empty() {
        return;
    }

    // [section] header
    if rest.starts_with('[') && rest.ends_with(']') {
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + rest.len()),
            kind: TokenKind::Heading,
        });
        return;
    }

    // key = value or key: value
    if let Some(sep) = rest.find('=').or_else(|| rest.find(':')) {
        let key = rest[..sep].trim_end();
        if !key.is_empty() {
            spans.push(HighlightSpan {
                byte_range: rest_base..(rest_base + key.len()),
                kind: TokenKind::Property,
            });
        }
        spans.push(HighlightSpan {
            byte_range: (rest_base + sep)..(rest_base + sep + 1),
            kind: TokenKind::Operator,
        });
        push_value_tokens(&rest[sep + 1..], rest_base + sep + 1, spans);
        return;
    }

    // Nginx / Apache style: `directive arg1 arg2;`
    if let Some(ws) = rest.find(|c: char| c.is_ascii_whitespace()) {
        let dir = &rest[..ws];
        let is_block = rest.ends_with('{');
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + dir.len()),
            kind: if is_block {
                TokenKind::Keyword
            } else {
                TokenKind::Property
            },
        });
        let args = rest[ws..].trim_end_matches([';', '{']).trim_end();
        push_value_tokens(args, rest_base + ws, spans);
    } else if rest == "}" || rest == "{" {
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + 1),
            kind: TokenKind::Punctuation,
        });
    }
}

fn lex_shell_line(line: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    if line.starts_with("#!") {
        spans.push(HighlightSpan {
            byte_range: base..(base + line.len()),
            kind: TokenKind::Attribute,
        });
        return;
    }
    let comment_at = find_unquoted_comment(line, false);
    let code = match comment_at {
        Some(idx) => {
            spans.push(HighlightSpan {
                byte_range: (base + idx)..(base + line.len()),
                kind: TokenKind::Comment,
            });
            &line[..idx]
        }
        None => line,
    };

    lex_generic_words_and_strings(
        code,
        base,
        &[
            "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac",
            "function", "return", "export", "local", "set", "echo", "exit", "source",
        ],
        spans,
    );
}

fn lex_dockerfile_line(line: &str, base: usize, spans: &mut Vec<HighlightSpan>) {
    let comment_at = find_unquoted_comment(line, false);
    let code = match comment_at {
        Some(idx) => {
            spans.push(HighlightSpan {
                byte_range: (base + idx)..(base + line.len()),
                kind: TokenKind::Comment,
            });
            &line[..idx]
        }
        None => line,
    };
    let indent = code.len() - code.trim_start().len();
    let rest = &code[indent..];
    let rest_base = base + indent;
    let first_end = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let instr = &rest[..first_end];
    if matches!(
        instr.to_ascii_uppercase().as_str(),
        "FROM"
            | "RUN"
            | "CMD"
            | "LABEL"
            | "EXPOSE"
            | "ENV"
            | "ADD"
            | "COPY"
            | "ENTRYPOINT"
            | "VOLUME"
            | "USER"
            | "WORKDIR"
            | "ARG"
            | "HEALTHCHECK"
            | "SHELL"
            | "AS"
    ) {
        spans.push(HighlightSpan {
            byte_range: rest_base..(rest_base + first_end),
            kind: TokenKind::Keyword,
        });
        lex_generic_words_and_strings(&rest[first_end..], rest_base + first_end, &["AS"], spans);
    } else {
        lex_generic_words_and_strings(rest, rest_base, &[], spans);
    }
}

fn lex_generic_words_and_strings(
    code: &str,
    base: usize,
    keywords: &[&str],
    spans: &mut Vec<HighlightSpan>,
) {
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' || b == b'\'' {
            let quote = b;
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && quote == b'"' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
            spans.push(HighlightSpan {
                byte_range: (base + start)..(base + i),
                kind: TokenKind::String,
            });
            if quote == b'"' {
                lex_var_interpolations(&code[start..i], base + start, spans);
            }
            continue;
        }
        if b == b'$' {
            let start = i;
            i += 1;
            if i < bytes.len() && bytes[i] == b'{' {
                while i < bytes.len() && bytes[i] != b'}' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            } else {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
            }
            if i > start + 1 {
                spans.push(HighlightSpan {
                    byte_range: (base + start)..(base + i),
                    kind: TokenKind::Variable,
                });
            }
            continue;
        }
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &code[start..i];
            if keywords.iter().any(|k| k.eq_ignore_ascii_case(word)) {
                spans.push(HighlightSpan {
                    byte_range: (base + start)..(base + i),
                    kind: TokenKind::Keyword,
                });
            }
            continue;
        }
        if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            spans.push(HighlightSpan {
                byte_range: (base + start)..(base + i),
                kind: TokenKind::Number,
            });
            continue;
        }
        i += 1;
    }
}
