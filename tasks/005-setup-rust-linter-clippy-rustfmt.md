# Task 005 — Setup Rust Linter (`clippy` + `rustfmt`) & CI Lint Gate

- **Status**: `ready_for_review`
- **Branch**: `chore/setup-rust-linter-clippy-rustfmt`
- **Date**: 2026-11-28

## 1. Scope
- Thiết lập chuẩn lint và format cho mã nguồn Rust của Draco:
  - Thêm `clippy.toml` và `rustfmt.toml` (edition 2024).
  - Bổ sung cổng `cargo clippy --all-targets -- -D warnings` vào `.github/workflows/ci.yml` và `docs/task-workflow.md` / `AGENTS.md`.
  - Sửa sạch toàn bộ cảnh báo clippy hiện có trong `src/` và `tests/` mà không làm thay đổi logic test.

## 2. Acceptance Criteria
- `cargo clippy --all-targets --offline -- -D warnings` pass với **0 warnings**.
- `cargo test --offline -- --test-threads=4` pass **100/100 tests**.
- `.github/workflows/ci.yml` tự động chặn PR nếu có cảnh báo Clippy.

## 3. Test Report
- **PASS**:
  - `.github/workflows/ci.yml` YAML validation: `YAML OK`.
  - `cargo clippy --all-targets --offline -- -D warnings`: **0 warnings**.
  - `cargo test --offline -- --test-threads=4`: **100/100 passed** (28 lib, 9 E2E, 9 feature, 54 unit).
- **FAIL**: None.
- **NOT RUN**: None.
