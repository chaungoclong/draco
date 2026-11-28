# Task 004 — Fast Startup Welcome Screen & Restore Last Workspace

- **Status**: `ready_for_review`
- **Branch**: `fix/startup-welcome-and-restore-workspace`
- **Date**: 2026-11-28

## 1. Scope
- **Problem**: Khi mở Draco từ menu ứng dụng (không truyền tham số đường dẫn CLI), `DracoApp::new` mặc định gọi `open_workspace_folder(std::env::current_dir())` → mở toàn bộ `/home/<user>`, kích hoạt `FileTree` + `notify` watcher + `WorkspaceIndexer` trên hàng chục nghìn thư mục khiến khởi động chậm ~2 giây.
- **Expected behavior (VS Code / PhpStorm style)**:
  - **Lần mở đầu tiên** (chưa có `last_workspace` hoặc thư mục cũ không còn tồn tại) và không có tham số CLI: **không mở `$HOME`/`current_dir()`**; khởi động tức thì ở **Welcome Screen** (nút Open Folder, Open File, New File, danh sách Recent Projects).
  - **Các lần mở sau** (khi đã từng mở project và `last_workspace` còn tồn tại trên đĩa): tự động khôi phục project mở gần nhất (`last_workspace`).
  - Khi mở bằng CLI có đường dẫn (`draco .`, `draco /path/to/proj`, `draco file.rs`): mở đúng đường dẫn yêu cầu và lưu lại vào `last_workspace` / `recent_workspaces`.
- **Target modules**:
  - `src/settings.rs` (`last_workspace`, `recent_workspaces`, MRU helpers)
  - `src/app.rs` (`DracoApp::new`, `open_workspace_folder`, Welcome Screen UI)
  - `tests/unit/settings_tests.rs`
  - `docs/CHANGELOG.md`, `docs/guides/user-guide.md`, `docs/guides/getting-started.md`

## 2. Acceptance Criteria
- Khởi động không tham số khi chưa có `last_workspace` không gọi watcher/indexer trên `$HOME` và hiển thị Welcome Screen.
- Mở một thư mục lưu `last_workspace` + `recent_workspaces` (tối đa 10, MRU) vào `settings.json`.
- Khởi động không tham số lần sau tự khôi phục `last_workspace` nếu thư mục còn tồn tại.
- Tương thích ngược 100% với `settings.json` cũ chưa có 2 trường mới.

## 3. Test Report
- **PASS**:
  - Gate 1 (`cargo check --all-targets --offline`): 0 errors, 0 warnings.
  - Gate 2 (`cargo test --offline -- --test-threads=4`): **100/100 passed** (28 lib, 9 E2E, 9 feature, 54 unit incl. `recent_workspaces_mru_dedup_cap_and_valid_last_workspace`).
  - Gate 3 (`cargo build --release --offline`): Built `target/release/draco` (26M).
- **FAIL**: None.
- **NOT RUN**: None.
