# Spec 007 — LSP IDE Actions (Quick-Fix, Rename, Inlay Hints, Signature Help) & Git Gutter

- **Trạng thái**: Implemented
- **Ngày tạo**: 2026-11-28

## 1. Bối cảnh & Mục tiêu
Hoàn thiện bộ tính năng trợ giúp lập trình chuyên sâu qua giao thức LSP chuẩn và theo dõi thay đổi Git trực tiếp trong trình soạn thảo mà không cần thêm dependency nặng:
- Quick-Fix / Code Actions (`Alt+Enter`) và Rename Symbol (`F2`) đồng bộ toàn workspace.
- Inlay Hints (gợi ý kiểu/tham số) và Signature Help khi gọi hàm.
- Vạch màu Git Gutter (`Added` / `Modified` / `Deleted`) ngay bên lề số dòng.

## 2. Thiết kế kỹ thuật
### LSP Client & WorkspaceEdit (`src/lsp/client.rs`, thuần non-UI)
- Kiểu dữ liệu mới: `TextEditItem`, `CodeActionItem`, `InlayHintItem`, `SignatureHelpInfo`.
- Sự kiện mới trong `LspEvent`: `CodeActions`, `RenameEdits`, `InlayHints`, `SignatureHelp`.
- Phương thức gửi yêu cầu JSON-RPC:
  - `request_code_actions` (`textDocument/codeAction`)
  - `request_rename` (`textDocument/rename`)
  - `request_inlay_hints` (`textDocument/inlayHint`)
  - `request_signature_help` (`textDocument/signatureHelp`)
- `parse_workspace_edit(&Value)`: hỗ trợ đầy đủ cả `WorkspaceEdit.changes` và `WorkspaceEdit.documentChanges`.
- `DracoApp::apply_workspace_edits`: áp dụng `TextEditItem` theo **thứ tự offset ngược** (cao → thấp) an toàn UTF-8 cho cả tab đang mở lẫn file trên đĩa.

### LSP Popups & Inline Hints (`src/ui/lsp_popups.rs`, `src/ui/code_view.rs`)
- **Code Actions popup (`Alt+Enter`)**: bám theo con trỏ, điều hướng `↑`/`↓`/`Enter`/`Esc` hoặc click.
- **Rename Symbol modal (`F2`)**: điền sẵn từ tại con trỏ, nhấn `Enter` gửi yêu cầu đổi tên qua LSP.
- **Signature Help tooltip**: tự gọi khi gõ `(` hoặc `,`, hiển thị phía trên con trỏ, nhấn `Esc` để đóng.
- **Inlay Hints**: vẽ dạng pill mờ ở cuối dòng hiển thị trong `CodeView`, không làm thay đổi tọa độ con trỏ hay cột ký tự.

### Git Gutter Indicators (`src/workspace/git_diff.rs`, thuần non-UI)
- `GitLineChange` (`Added`, `Modified`, `Deleted`).
- `parse_unified_diff_hunks(diff: &str) -> HashMap<usize, GitLineChange>`: parser thuần không phụ thuộc tiến trình ngoài cho header `@@ -a,b +c,d @@` của `git diff -U0 --no-color`.
- `GitDiffTracker`: chạy `git diff -U0 --no-color -- <file>` trên `std::thread` nền khi mở hoặc lưu file và gọi `repaint()` khi có kết quả.
- `CodeView`: vẽ vạch dọc 3px sát mép phải gutter (Xanh lá = `Added`, Xanh dương = `Modified`, Đỏ = `Deleted`).

## 3. Hành vi UI & Keybindings
- `Alt+Enter`: Mở menu Quick-Fix / Code Actions.
- `F2`: Mở hộp đổi tên biểu tượng toàn workspace (LSP Rename).
- Gõ `(` hoặc `,`: Tự kích hoạt Signature Help tooltip (`Esc` để ẩn).

## 4. Kiểm thử
- Unit tests: `tests/unit/phase3_lsp_git_tests.rs` (kiểm tra `parse_unified_diff_hunks` cho Added/Modified/Deleted và `parse_workspace_edit` cho cả `changes` và `documentChanges`).
