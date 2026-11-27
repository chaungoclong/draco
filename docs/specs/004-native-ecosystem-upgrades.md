# Spec 004 — Native Ecosystem Library Upgrades

- **Trạng thái**: `Implemented` (v0.4.0)
- **Phạm vi**: `Cargo.toml`, `src/workspace/fuzzy.rs`, `src/workspace/watcher.rs`, `src/ui/fonts.rs`, `src/settings.rs`, `src/core/buffer.rs`, `src/app.rs`

## 1. Mục tiêu
Thay thế các thành phần hạ tầng bằng các thư viện Rust chính thức, uy tín và tối ưu hiệu năng cao hơn trên đa nền tảng (Linux, macOS, Windows), đồng thời bổ sung tính năng tự động reload file khi thay đổi từ bên ngoài.

## 2. Các nâng cấp đã triển khai
1. **Quick Open Fuzzy Path Matching (`nucleo-matcher`)**:
   - Sử dụng `Config::DEFAULT.match_paths()` với `CaseMatching::Smart` và `Normalization::Smart` (thuật toán SIMD từ Helix Editor), tái sử dụng buffer `Utf32Str` khi lọc hàng chục nghìn file.
2. **Cross-Platform System Font Discovery (`fontdb`)**:
   - Nạp toàn bộ font hệ thống qua `fontdb::Database::load_system_fonts()` (Linux `fontconfig` + `~/.local/share/fonts`, macOS, Windows) và truy vấn tự động các họ font hỗ trợ đầy đủ tiếng Việt / UTF-8 (`Noto Sans Mono`, `JetBrains Mono`, `DejaVu Sans Mono`, `Noto Sans`, `DejaVu Sans`…).
3. **Standard OS Config Directories (`directories`)**:
   - Dùng `directories::ProjectDirs::from("dev", "draco", "draco")` cho đường dẫn `settings.json` và thư mục `plugins/`.
4. **Native OS File Watcher & External Auto-Reload (`notify`)**:
   - `WorkspaceWatcher` theo dõi thư mục workspace đệ quy qua `inotify`/`FSEvents`/`ReadDirectoryChangesW` (bỏ qua `.git`, `target`, `node_modules`).
   - Tự động gọi `Document::reload_from_disk()` cho các tab đang mở chưa có chỉnh sửa cục bộ (`!dirty`) khi file đổi bên ngoài, và tự làm mới `FileTree` + `WorkspaceIndexer` (debounced 600ms) khi có file mới/xóa.
