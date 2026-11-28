# Spec 008 — Cross-Platform Release Pipeline & End-User Distribution

- **Trạng thái**: Implemented
- **Ngày tạo**: 2026-11-28

## 1. Bối cảnh & Mục tiêu
Người dùng cuối không nên phải cài Rust toolchain, C compiler hay tự biên dịch từ mã nguồn mới dùng được Draco. Spec này chuẩn hóa quy trình phát hành tự động trên GitHub Releases tham khảo mô hình phân phối của các ứng dụng Rust hiện đại (Zed, Helix, Alacritty, Neovide):
- Tự động build binary tối ưu (`--release`, LTO, stripped) cho **Linux**, **macOS (Apple Silicon & Intel)** và **Windows** khi đẩy tag `v*`.
- Đóng gói định dạng quen thuộc nhất với từng hệ điều hành (`.deb`, `.tar.gz`, `.zip`) kèm `SHA256SUMS.txt`.
- Cung cấp script cài đặt 1 dòng lệnh (`install.sh` cho Linux/macOS, `install.ps1` cho Windows) và file `.desktop` cho Linux app launcher.

## 2. Thiết kế kỹ thuật
### GitHub Actions Release Workflow (`.github/workflows/release.yml`)
- **Trigger**:
  - `push` tag dạng `v*` (ví dụ `v0.1.0`, `v0.2.0`).
  - `workflow_dispatch` (cho phép chạy thử thủ công với input `tag`).
- **Build Matrix**:
  | Target | Runner | Định dạng đầu ra |
  |---|---|---|
  | `x86_64-unknown-linux-gnu` | `ubuntu-22.04` (glibc 2.35 tương thích rộng) | `draco-<tag>-x86_64-linux.tar.gz` + `draco_<ver>_amd64.deb` |
  | `aarch64-apple-darwin` | `macos-latest` | `draco-<tag>-aarch64-macos.tar.gz` |
  | `x86_64-apple-darwin` | `macos-latest` | `draco-<tag>-x86_64-macos.tar.gz` |
  | `x86_64-pc-windows-msvc` | `windows-latest` | `draco-<tag>-x86_64-setup.exe` (Inno Setup) + `draco-<tag>-x86_64-windows.exe` + `draco-<tag>-x86_64-windows.zip` |
- **Publish Job**:
  - Tải toàn bộ artifact từ matrix, tạo `SHA256SUMS.txt`.
  - Tạo GitHub Release bằng `softprops/action-gh-release@v2` kèm release notes tự động và hướng dẫn cài nhanh.

### Đóng gói & Trình cài đặt 1 dòng lệnh
- **`packaging/linux/draco.desktop`**: XDG Desktop Entry chuẩn để Draco xuất hiện trong ứng dụng Desktop trên Ubuntu/Debian/Fedora/Arch và hỗ trợ mở file trực tiếp (`Exec=draco %F`).
- **`scripts/install.sh`** (POSIX `sh`, Linux/macOS):
  - Tự nhận diện hệ điều hành & kiến trúc CPU (`x86_64`, `arm64`/`aarch64`).
  - Truy vấn bản phát hành mới nhất từ GitHub Releases (hoặc cài phiên bản chỉ định qua `DRACO_VERSION=v0.1.0`).
  - Tải `tar.gz` + đối chiếu mã băm với `SHA256SUMS.txt` (khi có `sha256sum` hoặc `shasum`).
  - Cài binary vào `~/.local/bin/draco` (tùy biến qua `DRACO_INSTALL_DIR`) và tự cài `draco.desktop` vào `~/.local/share/applications/` trên Linux.
- **`scripts/install.ps1`** (Windows PowerShell):
  - Tải `draco-<tag>-x86_64-windows.zip` + xác minh SHA256.
  - Giải nén vào `%LOCALAPPDATA%\Draco\bin` và tự thêm vào biến môi trường `PATH` của người dùng.

## 3. Quy trình phát hành dành cho Maintainer
```bash
# 1. Cập nhật version trong Cargo.toml và docs/CHANGELOG.md (qua PR vào main)
# 2. Tạo và đẩy tag semver từ main:
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```
GitHub Actions sẽ tự build toàn bộ 4 nền tảng và xuất bản GitHub Release trong ~5–8 phút.

## 4. Kiểm thử & Xác minh
- Kiểm tra cú pháp POSIX shell (`sh -n scripts/install.sh`) và tính hợp lệ YAML của `.github/workflows/release.yml`.
- Chạy `cargo check --all-targets` và `cargo test -- --test-threads=4`.
