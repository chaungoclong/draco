# Getting Started với Draco

## 1. Cài đặt nhanh (Khuyên dùng — Không cần cài Rust)

Người dùng cuối có thể tải và cài bản dựng sẵn trực tiếp từ [GitHub Releases](https://github.com/chaungoclong/draco/releases):

### Linux & macOS (1 dòng lệnh)
```sh
curl -fsSL https://raw.githubusercontent.com/chaungoclong/draco/main/scripts/install.sh | sh
```
- Tự động nhận diện hệ điều hành & kiến trúc (`x86_64-linux`, `aarch64-macos`, `x86_64-macos`), kiểm tra mã băm `SHA256SUMS.txt`, cài `draco` vào `~/.local/bin/draco` và tạo biểu tượng ứng dụng `draco.desktop` trên Linux.

### Ubuntu / Debian (Gói `.deb`)
Tải file `draco_*_amd64.deb` từ trang [Releases](https://github.com/chaungoclong/draco/releases) và cài đặt:
```bash
sudo apt install ./draco_*_amd64.deb
```

### Windows (`.exe` chạy trực tiếp hoặc PowerShell)
- **Chạy ngay không cần cài**: tải file `draco-*-x86_64-windows.exe` từ [GitHub Releases](https://github.com/chaungoclong/draco/releases) và click đúp để mở.
- **Hoặc cài vào `PATH` bằng PowerShell**:
```powershell
irm https://raw.githubusercontent.com/chaungoclong/draco/main/scripts/install.ps1 | iex
```
- Tự tải bản `x86_64-windows.zip`, xác minh SHA256, giải nén vào `%LOCALAPPDATA%\Draco\bin` và thêm vào User `PATH`.

### Khởi chạy
```bash
draco .
draco src/main.rs
```

---

## 2. Kích hoạt LSP (Autocomplete, Quick-Fix, Rename, Diagnostics)
Draco tự động tìm các Language Server có sẵn trong biến môi trường `PATH` khi bạn mở file tương ứng:

| Ngôn ngữ | Binary LSP mặc định | Cách cài nhanh |
|---|---|---|
| Rust | `rust-analyzer` | `rustup component add rust-analyzer` |
| Python | `pyright-langserver` | `npm i -g pyright` |
| JavaScript / TypeScript | `typescript-language-server` | `npm i -g typescript-language-server typescript` |
| PHP | `intelephense` | `npm i -g intelephense` |
| C / C++ | `clangd` | `sudo apt install clangd` |

Nếu chưa cài server tương ứng, Draco vẫn hoạt động đầy đủ với Tree-sitter highlighting và hiển thị nhẹ trạng thái trên thanh Status Bar.

---

## 3. Build từ mã nguồn (Dành cho lập trình viên)

Yêu cầu **Rust toolchain (stable, edition 2024)** và trình biên dịch C (`gcc` hoặc `clang`):

```bash
# Chạy toàn bộ bộ test (Lib + Unit + Feature + Headless E2E)
cargo test -- --test-threads=4

# Build bản release tối ưu (LTO + stripped)
cargo build --release

# Chạy trực tiếp bản vừa build
./target/release/draco .
```
