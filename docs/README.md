# Draco Documentation Hub

Chào mừng đến với trung tâm tài liệu của **Draco** — GUI code editor nhỏ nhẹ, nhanh, GPU-accelerated viết bằng Rust.

## Cấu trúc tài liệu

- **[CHANGELOG.md](./CHANGELOG.md)** — Lịch sử thay đổi, tính năng mới và bản vá theo từng phiên bản.
- **[architecture.md](./architecture.md)** — Kiến trúc tổng thể, mô hình luồng nền, `CodeView` virtualized và Tree-sitter tăng dần.
- **[task-workflow.md](./task-workflow.md)** — Quy trình thực thi task, chất lượng kiểm thử, tách nhánh và đồng bộ tài liệu.

### Specifications (`docs/specs/`)
- **[specs/README.md](./specs/README.md)** — Quy ước viết đặc tả (spec) cho tính năng mới.
- **[specs/001-core-editor.md](./specs/001-core-editor.md)** — Đặc tả nền tảng của Draco (Buffer, Syntax, Workspace, LSP, Terminal, UI).
- **[specs/002-multi-terminal-settings-plugins-php.md](./specs/002-multi-terminal-settings-plugins-php.md)** — Đặc tả Multi-Tab Terminal, Settings, Auto-Save, Plugin System và PHP.
- **[specs/003-config-highlighting-markdown-preview.md](./specs/003-config-highlighting-markdown-preview.md)** — Đặc tả Highlight file cấu hình, Markdown Preview kiểu PhpStorm & Scroll Sync.
- **[specs/004-native-ecosystem-upgrades.md](./specs/004-native-ecosystem-upgrades.md)** — Nâng cấp thư viện chuẩn Rust (`nucleo-matcher`, `fontdb`, `directories`, `notify`).
- **[specs/005-ast-navigation-and-sticky-scroll.md](./specs/005-ast-navigation-and-sticky-scroll.md)** — AST Selection, Code Symbols, Sticky Scroll & Unified Quick Open (`Files | Recent | Symbols`).
- **[specs/006-workspace-search-split-editor-overview-ruler.md](./specs/006-workspace-search-split-editor-overview-ruler.md)** — Workspace Find & Replace, Split Editor & Overview Ruler.
- **[specs/007-lsp-ide-actions-and-git-gutter.md](./specs/007-lsp-ide-actions-and-git-gutter.md)** — LSP Quick-Fix, Rename, Inlay Hints, Signature Help & Git Gutter.

### Guides (`docs/guides/`)
- **[guides/user-guide.md](./guides/user-guide.md)** — **Hướng dẫn sử dụng chi tiết cho người dùng cuối** (từng bước làm chủ mọi tính năng của Draco).
- **[guides/getting-started.md](./guides/getting-started.md)** — Hướng dẫn build, chạy Draco và cài đặt Language Server (LSP).
- **[guides/keybindings.md](./guides/keybindings.md)** — Bảng tra cứu đầy đủ phím tắt và thao tác chuột / context menu.
- **[guides/testing.md](./guides/testing.md)** — Hướng dẫn chạy và viết Unit, Feature và Headless E2E tests.

### Tutorials (`docs/tutorials/`)
- **[tutorials/adding-a-language.md](./tutorials/adding-a-language.md)** — Hướng dẫn từng bước thêm hỗ trợ một ngôn ngữ lập trình mới (Tree-sitter + File Icon + LSP).
- **[tutorials/writing-plugins.md](./tutorials/writing-plugins.md)** — Hướng dẫn viết, cài đặt và chạy Plugin cho Draco.
