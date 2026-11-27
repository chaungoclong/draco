# Spec 001 — Draco Core Editor & IDE Essentials

- **Trạng thái**: `Implemented` (v0.1.0)
- **Phạm vi**: `src/core/`, `src/syntax/`, `src/workspace/`, `src/lsp/`, `src/terminal/`, `src/ui/`

## 1. Mục tiêu
Xây dựng GUI code editor nhỏ nhẹ, khởi động nhanh và phản hồi mượt trên file lớn (> 100.000 dòng) với bộ tính năng IDE thiết yếu tích hợp sẵn.

## 2. Yêu cầu chức năng
1. **Core Buffer**: `ropey::Rope`, UTF-8/CRLF safe, Undo/Redo coalescing, Multi-cursor (`SelectionSet`), Auto-indent, Auto-close pairs, Bracket matching, Comment toggle, Find & Replace (Regex).
2. **Syntax Highlighting**: Tree-sitter incremental parsing + visible-range highlight cho Rust, Python, JS, TS, JSON, TOML, C/C++, Markdown.
3. **Workspace**: Lazy `FileTree` (tạo/đổi tên/xóa file & thư mục qua Context Menu), background `.gitignore` indexer, Quick Open (`Ctrl+P`).
4. **LSP Client**: JSON-RPC 2.0 qua stdio (`rust-analyzer`, `pyright-langserver`, `typescript-language-server`, `clangd`) hỗ trợ Diagnostics, Completion, Go to Definition, Hover.
5. **Embedded Terminal**: `portable-pty` + `vte` ANSI/VT100 16/256/truecolor, scrollback 2000 dòng, auto-resize.
6. **Refined UI**: Activity Bar (44px), Vector file/folder icons, Tab Bar + Context Menu + Breadcrumbs, Virtualized `CodeView` + Context Menu + Indent guides, Dark/Light theme.

## 3. Yêu cầu phi chức năng
- Idle CPU ~0% (reactive repaint).
- Render & chỉnh sửa file 100.000 dòng < 250ms trên chế độ headless test, < 16ms trong bản release.
- Phủ test tự động ở cả 3 tầng: Unit, Feature, Headless E2E (`cargo test`).
