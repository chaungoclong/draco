# Spec 005 — AST Selection, Code Symbols, Sticky Scroll & Unified Quick Open

- **Trạng thái**: Implemented
- **Ngày tạo**: 2026-11-28

## 1. Bối cảnh & Mục tiêu
Bổ sung bộ công cụ điều hướng và chỉnh sửa thông minh tận dụng trực tiếp cây cú pháp Tree-sitter đã có sẵn trong Draco mà không làm tăng RAM hay độ trễ gõ phím:
- Chọn mở rộng / thu gọn theo cấu trúc cú pháp AST.
- Nhảy nhanh tới symbol trong file hiện tại (`Go to Symbol`).
- Ghim ngữ cảnh khối đang đọc ở mép trên khung code (`Sticky Scroll`) với độ phức tạp O(chiều sâu cây).
- Hợp nhất điều hướng `Files`, `Recent Files` (MRU) và `Symbols` vào một hộp tìm nhanh duy nhất.

## 2. Thiết kế kỹ thuật
### Module thuần non-UI (không phụ thuộc `egui`)
- **`src/syntax/ast.rs`**:
  - `extend_selection(tree, rope, current) -> Option<Selection>`: tìm node Tree-sitter nhỏ nhất bao phủ `byte_range` hiện tại, leo lên `.parent()` tới node có phạm vi lớn hơn nghiêm ngặt và chuyển về char offset an toàn UTF-8.
  - `AstSelectionHistory`: lưu stack các `Selection` trước đó để `shrink()` quay lui chính xác từng bước; tự xóa lịch sử khi vùng chọn thay đổi ngoài luồng AST.
- **`src/syntax/symbols.rs`**:
  - `SymbolKind` (`Function`, `Method`, `Struct`, `Enum`, `Trait`, `Impl`, `Class`, `Interface`, `Module`, `Constant`, `Heading`, `Section`) và `DocumentSymbol { name, kind, start_line, end_line, depth }`.
  - `extract_symbols(tree, rope, language)`: trích xuất symbol cho Rust, Python, JS/TS, PHP, C, TOML, JSON, Markdown (chỉ gọi khi người dùng mở tab `Symbols`).
  - `enclosing_scopes_at_line(tree, rope, language, top_line, max_scopes)`: truy vấn ngược từ node tại `top_line` lên `.parent()` (**O(tree depth) ≈ 0.01ms** kể cả trên file 100.000+ dòng).

### Tích hợp UI & App
- **`src/syntax/mod.rs`**: expose `ast`, `symbols`, `SyntaxState::tree()`, `SyntaxState::extract_symbols()`.
- **`src/ui/code_view.rs`**:
  - Xử lý phím `Shift+Alt+Right` / `Shift+Alt+Left` cho AST Expand/Shrink Selection.
  - Render thanh **Sticky Scroll** (tối đa 3 dòng scope bao quanh) phía trên `ScrollArea`; click vào dòng ghim nhảy thẳng tới `start_line`.
- **`src/ui/quick_open.rs`**, **`src/keymap.rs`**, **`src/app.rs`**:
  - Mở rộng Quick Open với 3 tab `Files | Recent | Symbols` (`QuickOpenTab`), chuyển nhanh bằng phím `Tab` hoặc click, lọc fuzzy bằng `nucleo-matcher`.
  - Theo dõi `recent_files: Vec<PathBuf>` theo thứ tự MRU (tối đa 30 file).

## 3. Hành vi UI & Keybindings
- `Shift+Alt+Right` / `Shift+Alt+Left`: Mở rộng / Thu gọn vùng chọn theo AST.
- `Ctrl+P`: Mở Quick Open tại tab `Files`.
- `Ctrl+E`: Mở Quick Open tại tab `Recent`.
- `Ctrl+R`: Mở Quick Open tại tab `Symbols`.
- `Tab` (trong popup): Xoay vòng giữa `Files → Recent → Symbols`.

## 4. Kiểm thử
- Unit tests: `tests/unit/ast_symbols_tests.rs` (kiểm tra AST Expand/Shrink UTF-8, trích xuất symbol đa ngôn ngữ và `enclosing_scopes`).
- Headless E2E tests: `tests/e2e/modals_e2e.rs` (`unified_quick_open_recent_and_symbols_tabs_e2e`) và `tests/e2e/performance_e2e.rs` (xác nhận Sticky Scroll không làm chậm file 100k dòng).
