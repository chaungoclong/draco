# Specifications (`docs/specs/`)

Thư mục này lưu trữ các tài liệu đặc tả kỹ thuật cho từng tính năng lớn của **Draco**.

## Quy trình thêm Spec mới
1. Tạo file theo định dạng `NNN-ten-tinh-nang.md` (ví dụ: `002-git-gutter.md`, `003-split-view.md`).
2. Mỗi spec nên gồm 4 phần gọn rõ:
   - **Bối cảnh & Mục tiêu**: Giải quyết vấn đề gì?
   - **Thiết kế kỹ thuật**: Cấu trúc dữ liệu, module bị ảnh hưởng, tác động hiệu năng.
   - **Hành vi UI / Keybindings**: Cách người dùng tương tác.
   - **Kế hoạch kiểm thử**: Unit / Feature / E2E test cần bổ sung.
3. Sau khi triển khai xong, cập nhật trạng thái spec thành `Implemented` và ghi nhận vào [../CHANGELOG.md](../CHANGELOG.md).

## Danh sách Spec hiện có
| ID | Tên | Trạng thái | File |
|---|---|---|---|
| 001 | Core Editor, Syntax, Workspace, LSP, Terminal & Refined UI | Implemented | [001-core-editor.md](./001-core-editor.md) |
| 002 | Multi-Tab Terminal, Settings, Auto-Save, Plugins & PHP | Implemented | [002-multi-terminal-settings-plugins-php.md](./002-multi-terminal-settings-plugins-php.md) |
| 003 | Config Highlighting & High-Performance Markdown Preview | Implemented | [003-config-highlighting-markdown-preview.md](./003-config-highlighting-markdown-preview.md) |
| 004 | Native Ecosystem Library Upgrades (`nucleo-matcher`, `fontdb`, `directories`, `notify`) | Implemented | [004-native-ecosystem-upgrades.md](./004-native-ecosystem-upgrades.md) |
| 005 | AST Selection, Code Symbols, Sticky Scroll & Unified Quick Open | Implemented | [005-ast-navigation-and-sticky-scroll.md](./005-ast-navigation-and-sticky-scroll.md) |
| 006 | Workspace Find & Replace, Split Editor & Overview Ruler | Implemented | [006-workspace-search-split-editor-overview-ruler.md](./006-workspace-search-split-editor-overview-ruler.md) |
| 007 | LSP IDE Actions (Quick-Fix, Rename, Inlay Hints, Signature Help) & Git Gutter | Implemented | [007-lsp-ide-actions-and-git-gutter.md](./007-lsp-ide-actions-and-git-gutter.md) |