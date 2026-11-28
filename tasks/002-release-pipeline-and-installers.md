# Task 002 — Cross-Platform Release Pipeline & End-User Installers

- **Status**: `ready_for_review`
- **Branch**: `ci/release-pipeline-and-installers`
- **Date**: 2026-11-28

## 1. Scope
- **Goal**: Người dùng cuối có thể tải và cài Draco trực tiếp bằng binary dựng sẵn (1 dòng lệnh hoặc gói `.deb` / `.zip` / `.tar.gz` từ GitHub Releases) mà không cần cài Rust toolchain hay tự build từ mã nguồn.
- **Target files**:
  - `.github/workflows/release.yml`
  - `packaging/linux/draco.desktop`
  - `scripts/install.sh` (Linux / macOS)
  - `scripts/install.ps1` (Windows PowerShell)
  - `docs/specs/008-release-pipeline-and-distribution.md`
  - `docs/specs/README.md`, `docs/README.md`, `docs/guides/getting-started.md`, `docs/guides/user-guide.md`, `docs/CHANGELOG.md`

## 2. Acceptance Criteria
- Push tag `v*` (hoặc chạy tay `workflow_dispatch`) tự động build & đóng gói cho 4 target:
  - Linux x86_64 (`.tar.gz` + `.deb` tương thích Ubuntu 22.04+ / Debian 12+)
  - macOS Apple Silicon (`aarch64-apple-darwin`, `.tar.gz`)
  - macOS Intel (`x86_64-apple-darwin`, `.tar.gz`)
  - Windows x86_64 (`x86_64-pc-windows-msvc`, `.zip`)
- Tự sinh `SHA256SUMS.txt` và tạo GitHub Release đính kèm toàn bộ gói cài đặt.
- `scripts/install.sh` (kiểm tra cú pháp bằng `sh -n`) và `scripts/install.ps1` cài được Draco chỉ bằng 1 dòng lệnh, có xác minh SHA256.
- Cập nhật `getting-started.md` và `user-guide.md` ưu tiên hướng dẫn tải/cài bản dựng sẵn trước phần build from source.

## 3. Test Report
- **PASS**:
  - `sh -n scripts/install.sh`: POSIX shell syntax valid.
  - `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`: YAML valid (`YAML OK`).
  - `cargo test --offline -- --test-threads=4`: **98/98 passed** (28 lib, 9 E2E, 8 feature, 53 unit, 0 warnings).
- **FAIL**: None.
- **NOT RUN**:
  - Remote GitHub Actions release matrix execution (will run on GitHub when pushing a `v*` tag or triggering `workflow_dispatch`).
