use std::path::Path;
use std::time::Instant;

use draco::core::buffer::Document;
use draco::core::LanguageId;
use draco::syntax::{SyntaxState, TokenKind};
use draco::ui::icons::FileIconKind;

#[test]
fn detects_config_languages_and_icons_for_common_filenames() {
    let cases = [
        ("docker-compose.yml", LanguageId::Yaml, FileIconKind::Yaml),
        ("config.yaml", LanguageId::Yaml, FileIconKind::Yaml),
        (".env", LanguageId::DotEnv, FileIconKind::DotEnv),
        (".env.local", LanguageId::DotEnv, FileIconKind::DotEnv),
        (".env.production", LanguageId::DotEnv, FileIconKind::DotEnv),
        ("nginx.conf", LanguageId::Conf, FileIconKind::Conf),
        ("php.ini", LanguageId::Conf, FileIconKind::Conf),
        (".editorconfig", LanguageId::Conf, FileIconKind::Conf),
        ("deploy.sh", LanguageId::Shell, FileIconKind::Shell),
        ("Dockerfile", LanguageId::Dockerfile, FileIconKind::Docker),
        ("Dockerfile.prod", LanguageId::Dockerfile, FileIconKind::Docker),
    ];

    for (name, expected_lang, expected_icon) in cases {
        let p = Path::new(name);
        assert_eq!(LanguageId::from_path(p), expected_lang, "lang for {name}");
        assert_eq!(FileIconKind::from_path(p), expected_icon, "icon for {name}");
    }
}

#[test]
fn yaml_dotenv_conf_shell_dockerfile_produce_accurate_token_kinds() {
    // YAML
    let yaml = "---\n# comment\nserver:\n  port: 8080\n  enabled: true\n  host: \"localhost\"\n  - item\n";
    let doc = Document::from_str(1, yaml, LanguageId::Yaml);
    let syn = SyntaxState::new(LanguageId::Yaml);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword)); // ---
    assert!(spans.iter().any(|s| s.kind == TokenKind::Comment));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Property));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Number));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Constant)); // true
    assert!(spans.iter().any(|s| s.kind == TokenKind::String));

    // .env
    let env = "# database\nexport APP_DEBUG=false\nDB_URL=\"postgres://${DB_USER}:5432\"\nPORT=3000\n";
    let doc = Document::from_str(2, env, LanguageId::DotEnv);
    let syn = SyntaxState::new(LanguageId::DotEnv);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword)); // export
    assert!(spans.iter().any(|s| s.kind == TokenKind::Property));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Operator));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Variable)); // ${DB_USER}
    assert!(spans.iter().any(|s| s.kind == TokenKind::Number));

    // INI / nginx.conf
    let conf = "; ini comment\n[database]\nmax_connections = 100\nserver {\n    listen 443;\n}\n";
    let doc = Document::from_str(3, conf, LanguageId::Conf);
    let syn = SyntaxState::new(LanguageId::Conf);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Comment));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Heading)); // [database]
    assert!(spans.iter().any(|s| s.kind == TokenKind::Property));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword)); // server {
    assert!(spans.iter().any(|s| s.kind == TokenKind::Number));

    // Shell & Dockerfile
    let sh = "#!/usr/bin/env bash\nexport MODE=\"prod\"\nif [ -n \"$MODE\" ]; then echo 1; fi\n";
    let doc = Document::from_str(4, sh, LanguageId::Shell);
    let syn = SyntaxState::new(LanguageId::Shell);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Attribute)); // shebang
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Variable));

    let docker = "FROM rust:1.100 AS builder\nWORKDIR /app\nEXPOSE 8080\n";
    let doc = Document::from_str(5, docker, LanguageId::Dockerfile);
    let syn = SyntaxState::new(LanguageId::Dockerfile);
    let spans = syn.highlights_in_range(&doc.rope, 0..doc.rope.len_bytes());
    assert!(spans.iter().any(|s| s.kind == TokenKind::Keyword));
    assert!(spans.iter().any(|s| s.kind == TokenKind::Number));
}

#[test]
fn config_lexer_visible_range_is_sub_millisecond_on_large_yaml() {
    let mut big = String::with_capacity(1_500_000);
    for i in 0..40_000 {
        big.push_str(&format!("service_{i}: \"http://host-{i}:8080\" # item {i}\n"));
    }
    let doc = Document::from_str(1, &big, LanguageId::Yaml);
    let syn = SyntaxState::new(LanguageId::Yaml);
    let start = doc.rope.line_to_byte(20_000);
    let end = doc.rope.line_to_byte(20_060);

    let t0 = Instant::now();
    let spans = syn.highlights_in_range(&doc.rope, start..end);
    let elapsed = t0.elapsed();
    assert!(!spans.is_empty());
    assert!(
        elapsed.as_millis() < 20,
        "Config visible-range lex took too long: {elapsed:?}"
    );
}
