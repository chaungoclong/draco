# Task 001 — Clear Previous Workspace State When Opening a New Folder

- **Status**: `ready_for_review`
- **Branch**: `fix/clear-state-on-open-folder`
- **Date**: 2026-11-28

## 1. Scope
- **Bug**: Khi mở thư mục workspace mới (`Ctrl+Shift+O` hoặc nút Open Folder), các tab, lịch sử file, Quick Open index và kết quả tìm kiếm của thư mục cũ vẫn còn hiển thị.
- **Target modules**:
  - `src/app.rs` (`open_workspace_folder`)
  - `src/workspace/fuzzy.rs` (`WorkspaceIndexer` generation counter & clear on `start_index`)
  - `src/workspace/git_diff.rs` (`GitDiffTracker::clear`)
  - `tests/feature/workspace_quick_open.rs`
  - `docs/CHANGELOG.md`, `docs/guides/user-guide.md`

## 2. Acceptance Criteria
- Chuyển sang thư mục mới tự động đóng toàn bộ tab cũ (mở 1 tab trống sạch), reset Split Editor, xóa `recent_files`, `workspace_searcher`, `find_bar`, `diagnostics`, `inlay_hints`, `code_actions`, `completion`, `signature_help`, `git_diff`, `explorer_ui`, và `md_cache`.
- `WorkspaceIndexer::start_index` xóa ngay danh sách file cũ và bỏ qua luồng index cũ nhờ `AtomicU64` generation counter.
- Mở lại đúng thư mục hiện tại không làm mất các tab đang mở.

## 3. Test Report
- **PASS**:
  - `cargo check --all-targets --offline`: 0 errors, 0 warnings.
  - `cargo test --offline -- --test-threads=4`: **99/99 passed** (28 lib, 9 E2E, 9 feature incl. `switching_workspace_folder_clears_previous_indexed_files`, 53 unit).
  - `cargo build --release --offline`: Built `target/release/draco` (26M).
- **FAIL**: None.
- **NOT RUN**: None.
