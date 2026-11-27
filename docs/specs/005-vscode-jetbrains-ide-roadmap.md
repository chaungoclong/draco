# Spec 005 — Lộ trình IDE (Chọn lọc tinh hoa VS Code + JetBrains) & Đợt 1

- **Trạng thái**: In Progress (Đợt 1 Implemented)
- **Ngày tạo**: 2026-11-28

## 1. Bối cảnh & Triết lý sản phẩm
**Draco** định vị ở giao điểm giữa hai trường phái mạnh nhất:
- **VS Code / Zed**: khởi động tức thì, nhẹ RAM, giao diện tối giản ít nhiễu, Command Palette làm trung tâm, multi-cursor mạnh mẽ.
- **JetBrains (PhpStorm / RustRover / IntelliJ New UI)**: điều hướng ngữ cảnh siêu nhanh (`Recent Files`, `Search Everywhere`, `Go to Symbol`), chỉnh sửa thông minh dựa trên AST (`Extend/Shrink Selection`), phân cấp thị giác rõ ràng và Markdown/Preview chất lượng cao.

### Những gì Draco chủ động TRÁNH
- Không dùng Electron / webview nặng RAM và có độ trễ gõ phím.
- Không nhồi quá nhiều tool window / dialog lồng nhau gây rối mắt.
- Không chạy indexer nặng nề chặn UI khi mở workspace lớn.

---

## 2. Tổng hợp tính năng chọn lọc & Lộ trình 3 đợt

### Đợt 1 — Điều hướng & Chỉnh sửa thông minh (tận dụng Tree-sitter) ✅
1. **AST Extend / Shrink Selection (`Shift+Alt+Right` / `Shift+Alt+Left`)**:
   - Mở rộng vùng chọn thông minh theo cây cú pháp Tree-sitter (`identifier → expression → statement → block → function → root`) và thu gọn lại đúng lịch sử chọn.
2. **Code Outline / Go to Symbol (`Ctrl+R`)**:
   - Trích xuất symbol từ Tree-sitter AST cho Rust, Python, JS/TS, PHP, C/C++, JSON, TOML, Markdown (`struct`, `enum`, `trait`, `impl`, `fn`, `class`, `method`, `interface`, `heading`…).
   - Tìm kiếm fuzzy tức thì bằng `nucleo-matcher` và nhảy thẳng tới dòng định nghĩa.
3. **Sticky Scroll trong `CodeView`**:
   - Tự động ghim tối đa 3 dòng ngữ cảnh bao quanh (`impl`, `fn`, `class`, `struct`…) ở mép trên khung code khi cuộn sâu vào block dài; bấm vào từng dòng ghim để nhảy ngay về đầu khối.
4. **Recent Files (`Ctrl+E`) & Unified Quick Open (`Files | Recent | Symbols`)**:
   - Theo dõi danh sách file vừa truy cập theo thứ tự MRU (Most Recently Used).
   - Hợp nhất điều hướng vào một modal gọn duy nhất có tab chuyển nhanh bằng `Tab` hoặc click (`Files` — `Ctrl+P`, `Recent` — `Ctrl+E`, `Symbols` — `Ctrl+R`).

### Đợt 2 — Tìm kiếm toàn Workspace & Split Editor (Kế hoạch)
- **Workspace Find & Replace (`Ctrl+Shift+F`)** đa luồng dựa trên `ignore` + `regex`, có bộ lọc include/exclude.
- **Split Editor (`Ctrl+\`)** chia 2 pane song song cho mọi file code.
- **Overview Ruler** trên thanh cuộn `CodeView` (chấm màu lỗi LSP, cảnh báo và vị trí kết quả Find).

### Đợt 3 — Nâng cấp LSP lên tầm IDE & Git Inline (Kế hoạch)
- **Quick-Fix / Code Actions (`Alt+Enter`)** & **Rename Symbol (`F2`)**.
- **Inlay Hints** (kiểu biến & tên tham số) + **Signature Help**.
- **Git Gutter Indicators** (vạch màu added / modified / deleted bên lề số dòng).

---

## 3. Thiết kế kỹ thuật Đợt 1

### Module mới (thuần non-UI, không phụ thuộc `egui`)
- `src/syntax/ast.rs`:
  - `extend_selection(tree, rope, current_selection) -> Option<Selection>`: tìm node nhỏ nhất bao phủ `byte_range` hiện tại, leo lên `parent()` cho tới khi tìm được node có phạm vi lớn hơn nghiêm ngặt, chuyển về char offset an toàn UTF-8.
  - `AstSelectionHistory`: lưu stack các `Selection` trước đó để `Shrink Selection` (`Shift+Alt+Left`) quay lui chính xác, tự reset khi con trỏ thay đổi ngoài luồng AST.
- `src/syntax/symbols.rs`:
  - `SymbolKind` (`Function`, `Method`, `Struct`, `Enum`, `Trait`, `Impl`, `Class`, `Interface`, `Module`, `Constant`, `Heading`, `Section`).
  - `DocumentSymbol { name, kind, start_line, end_line, depth }`.
  - `extract_symbols(tree, rope, language) -> Vec<DocumentSymbol>`: duyệt AST bằng `TreeCursor` không cấp phát thừa.
  - `enclosing_scopes(symbols, top_line, max_scopes) -> Vec<&DocumentSymbol>`: lọc các scope bao quanh dòng đang xem (`start_line < top_line && top_line <= end_line`) để render Sticky Scroll.

### Cập nhật UI & App
- `src/syntax/mod.rs`: expose `pub mod ast; pub mod symbols;` và `SyntaxState::tree(&self) -> Option<&Tree>`.
- `src/ui/code_view.rs`:
  - Nhận `Shift+Alt+Right` / `Shift+Alt+Left` cho AST Extend/Shrink Selection.
  - Render thanh **Sticky Scroll** (tối đa 3 dòng scope) ở đầu `CodeView` khi `top_visible_line` nằm bên trong một khối code có độ dài ≥ 4 dòng; click vào dòng ghim sẽ cuộn tới `start_line`.
- `src/ui/quick_open.rs`:
  - Bổ sung `QuickOpenTab` (`Files`, `Recent`, `Symbols`), `QuickOpenResult` (`File(PathBuf)`, `SymbolLine(usize)`), chuyển tab bằng phím `Tab` hoặc click, dùng `nucleo-matcher` cho cả 3 tab.
- `src/keymap.rs` & `src/app.rs`:
  - Thêm `AppAction::OpenRecentFiles` (`Ctrl+E`) và `AppAction::OpenSymbolPicker` (`Ctrl+R`).
  - Lưu `recent_files: Vec<PathBuf>` theo thứ tự MRU (tối đa 30 file) và trích xuất symbol cho tab đang mở.

## 4. Kế hoạch kiểm thử
- **Unit tests** (`tests/unit/ast_symbols_tests.rs`):
  - Kiểm tra AST Extend/Shrink Selection trên Rust & Python (UTF-8 an toàn, leo nhiều cấp, shrink đúng lịch sử).
  - Kiểm tra trích xuất symbols trên Rust (`struct`, `enum`, `impl`, `fn`), Python (`class`, `def`), TypeScript/JS (`class`, `function`), PHP (`class`, `function`), và Markdown headings.
  - Kiểm tra `enclosing_scopes` cho Sticky Scroll (giới hạn `max_scopes`, bỏ qua khối kết thúc trước `top_line`).
- **E2E tests** (`tests/e2e/modals_e2e.rs`):
  - Kiểm tra chuyển tab `Files | Recent | Symbols` trong Quick Open và chọn symbol nhảy đúng dòng.
- **CI (`.github/workflows/ci.yml`)**:
  - Tự động chạy `cargo check --all-targets`, `cargo test`, và `cargo build --release` trên Ubuntu cho mọi push và PR vào `main`.
