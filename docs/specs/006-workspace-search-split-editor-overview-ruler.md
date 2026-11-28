# Spec 006 — Workspace Find & Replace, Split Editor & Overview Ruler

- **Trạng thái**: Implemented
- **Ngày tạo**: 2026-11-28

## 1. Bối cảnh & Mục tiêu
Nâng cấp khả năng làm việc trên dự án nhiều file và file dài:
- Tìm kiếm & thay thế trên toàn bộ workspace tôn trọng `.gitignore`, chạy nền không chặn UI.
- Chia đôi cửa sổ soạn thảo (`Split Editor`) để đọc/sửa song song 2 file hoặc 2 vị trí khác nhau của cùng 1 file.
- Hiển thị bản đồ vị trí lỗi LSP, cảnh báo, kết quả tìm kiếm và con trỏ trên thanh dọc `Overview Ruler`.

## 2. Thiết kế kỹ thuật
### Workspace Find & Replace
- **`src/workspace/search.rs`** (thuần non-UI):
  - `WorkspaceSearchOptions`: `case_sensitive`, `whole_word`, `use_regex`, `include_filter`, `exclude_filter`.
  - `search_workspace_sync`: dùng `ignore::WalkBuilder` (`.require_git(false).git_ignore(true)`) + `regex`, tự bỏ qua thư mục `.git`, file nhị phân và file > 2MB; tính cột ký tự UTF-8 chính xác.
  - `replace_in_workspace`: thay thế an toàn toàn workspace và trả về `WorkspaceReplaceSummary { files_changed, replacements, modified_paths }`.
  - `WorkspaceSearcher`: chạy tìm kiếm trên `std::thread` kèm `AtomicU64` generation cancellation để hủy truy vấn cũ khi gõ nhanh, gọi `repaint()` khi có kết quả mới.
- **`src/ui/workspace_search.rs`**:
  - Giao diện đặt trong Sidebar trái (2 tab `Explorer | Search`), hỗ trợ `Aa`, `W`, `.*`, bộ lọc Include/Exclude (`⋯`), khung Replace (`⇄`) và nhóm kết quả theo file.
  - Sau khi `ReplaceAll`, `DracoApp` tự động reload lại các tab đang mở tương ứng (nếu chưa có chỉnh sửa cục bộ).

### Split Editor
- **`src/app.rs`**, **`src/keymap.rs`**:
  - Lưu `split_tab: Option<usize>`, `split_right_focused: bool`, và `split_view: CodeViewState` với `scroll_salt: 1` (tách biệt hoàn toàn ID `ScrollArea` để cuộn độc lập ngay cả khi mở cùng 1 file ở cả 2 pane).
  - Pane phải có thanh tiêu đề gọn hiển thị tên file, nút chuyển tab (`‹` / `›`) và nút đóng split (`×`); pane đang focus có viền accent 2px ở mép trên.

### Overview Ruler
- **`src/ui/code_view.rs`**:
  - Hàm thuần `overview_marker_y(line, total_lines, ruler_top, ruler_height) -> f32`.
  - Thanh dọc 10px mép phải `CodeView` vẽ vạch kết quả Find (lấy mẫu tối đa 400 điểm để giữ O(1)), vạch LSP Warning (vàng), LSP Error (đỏ) và vị trí con trỏ chính; click/kéo chuột để cuộn tức thì tới dòng tương ứng.

## 3. Hành vi UI & Keybindings
- `Ctrl+Shift+F`: Mở Sidebar trái tại tab `Search` (Workspace Find & Replace).
- `Ctrl+\`: Bật/tắt Split Editor 2 pane song song.
- Click / kéo trên Overview Ruler (mép phải `CodeView`): Nhảy nhanh tới vị trí tương ứng trong file.

## 4. Kiểm thử
- Unit tests: `tests/unit/workspace_search_tests.rs` (kiểm tra `.gitignore`, Include/Exclude, UTF-8 column index, Replace All và `overview_marker_y`).
