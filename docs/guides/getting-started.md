# Getting Started với Draco

## Yêu cầu hệ thống
- **Rust toolchain** (khuyến nghị bản stable mới nhất, hỗ trợ edition 2024).
- **Trình biên dịch C** (`gcc` hoặc `clang`) để biên dịch các grammar Tree-sitter.
- Môi trường desktop Linux (X11/Wayland), macOS hoặc Windows.

## Build & Chạy

```bash
# Kiểm tra và chạy toàn bộ bộ test (Unit + Feature + E2E)
cargo test

# Build bản release tối ưu (LTO + stripped)
cargo build --release

# Mở Draco với thư mục hoặc file chỉ định
./target/release/draco /path/to/your/project
./target/release/draco src/main.rs
```

## Kích hoạt LSP (Autocomplete, Diagnostics, Go to Definition)
Draco tự động tìm các Language Server có sẵn trong biến môi trường `PATH` khi bạn mở file tương ứng:

| Ngôn ngữ | Binary LSP mặc định | Cách cài nhanh |
|---|---|---|
| Rust | `rust-analyzer` | `rustup component add rust-analyzer` |
| Python | `pyright-langserver` | `npm i -g pyright` |
| JavaScript / TypeScript | `typescript-language-server` | `npm i -g typescript-language-server typescript` |
| C / C++ | `clangd` | `sudo apt install clangd` |

Nếu chưa cài server tương ứng, Draco vẫn hoạt động đầy đủ với Tree-sitter highlighting và hiển thị nhẹ trạng thái trên thanh Status Bar.
