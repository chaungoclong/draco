# Tutorial: Thêm một ngôn ngữ lập trình mới vào Draco

Hướng dẫn này minh họa cách thêm hỗ trợ ngôn ngữ mới (ví dụ: **Go**) vào Draco trong 5 bước ngắn gọn.

## Bước 1: Thêm grammar Tree-sitter vào `Cargo.toml`
Thêm crate grammar tương thích với `tree-sitter = "0.24"`:

```toml
tree-sitter-go = "0.23"
```

## Bước 2: Đăng ký `LanguageId` trong `src/core/mod.rs`
1. Thêm variant `Go` vào enum `LanguageId`.
2. Cập nhật các phương thức:
   - `from_path`: map `Some("go") => Self::Go`
   - `display_name`: `Self::Go => "Go"`
   - `line_comment`: `Self::Go => Some("//")`
   - `lsp_language_id`: `Self::Go => "go"`

## Bước 3: Kết nối Grammar trong `src/syntax/languages.rs`
Thêm nhánh trong `grammar_for(lang)`:

```rust
LanguageId::Go => Some(GrammarSpec {
    language: tree_sitter_go::LANGUAGE.into(),
    highlights_query: tree_sitter_go::HIGHLIGHTS_QUERY,
}),
```

## Bước 4: Thêm File Icon trong `src/ui/icons.rs`
1. Thêm `FileIconKind::Go`, map `Some("go") => Self::Go`.
2. Gán màu đặc trưng (ví dụ cyan `Color32::from_rgb(0, 173, 216)`) và badge `Some("go")`.

## Bước 5: (Tùy chọn) Đăng ký LSP Server trong `src/lsp/client.rs`
Thêm vào `LspManager::server_command`:

```rust
LanguageId::Go => Some(("gopls", &[])),
```

## Bước 6: Bổ sung Test & Cập nhật Docs
1. Thêm mẫu code Go vào test `all_supported_languages_produce_highlights` trong `src/syntax/mod.rs`.
2. Chạy `cargo test` để xác nhận.
3. Ghi nhận thay đổi vào `docs/CHANGELOG.md`.
