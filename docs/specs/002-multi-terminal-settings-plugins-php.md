# Spec 002 — Multi-Tab Terminal, Settings, Auto-Save, Plugins & PHP

- **Trạng thái**: `Implemented` (v0.2.0)
- **Phạm vi**: `src/settings.rs`, `src/plugins/`, `src/terminal/manager.rs`, `src/ui/settings_modal.rs`, `src/ui/command_palette.rs`, `src/ui/terminal_panel.rs`, `src/core/`, `src/syntax/`

## 1. Mục tiêu
Mở rộng Draco thành môi trường làm việc linh hoạt hơn: quản lý nhiều phiên terminal song song với nhiều kiểu bố cục, cấu hình lưu bền vững (Settings), Auto-Save mặc định bật kèm hiển thị rõ trạng thái file, hệ thống Plugin dễ viết/cài đặt và hỗ trợ ngôn ngữ PHP.

## 2. Thiết kế kỹ thuật
1. **Multi-Tab Terminal & 4 Display Modes**:
   - `TerminalManager` quản lý nhiều `TerminalTab` độc lập, mỗi tab là một `PtySession` riêng.
   - 4 chế độ hiển thị (`TerminalLayout`): `Bottom`, `Right`, `Floating` (cửa sổ nổi kéo giãn), `Maximized` (phủ toàn vùng editor).
2. **Settings (`~/.config/draco/settings.json`)**:
   - Lưu cấu hình theme, font size, tab size, `auto_save` (mặc định `true`), `auto_save_delay_ms` (800ms), `show_indent_guides`, `terminal_layout`, `disabled_plugins`.
   - Giao diện Settings Modal (`Ctrl+,` và icon bánh răng trên Activity Bar).
3. **Auto-Save & File Status Indicators**:
   - Auto-Save tự động lưu các file đã có đường dẫn sau khoảng trễ khi ngừng gõ mà vẫn giữ 0% CPU khi đứng yên (`ctx.request_repaint_after`).
   - Trạng thái `● Modified` / `✓ Saved` / `● Unsaved` hiển thị đồng bộ tại Tab Bar, Breadcrumbs, Status Bar và chấm `●` trên File Explorer; nút `Auto Save: ON/OFF` bật/tắt ngay trên Status Bar.
4. **Plugin System (`src/plugins/`)**:
   - Manifest `plugin.json` trong `~/.config/draco/plugins/<id>/` hoặc `.draco/plugins/<id>/`.
   - Hỗ trợ `install_from_dir`, bật/tắt, gỡ cài đặt, thực thi lệnh với biến môi trường `DRACO_*`, truyền stdin (`none`/`selection`/`document`/`selection_or_document`) và xử lý stdout (`status`/`insert`/`replace_selection`/`replace_document`).
   - Chạy lệnh plugin qua **Command Palette (`Ctrl+Shift+P`)** hoặc trực tiếp trong Settings → Plugins.
5. **Ngôn ngữ PHP**:
   - `LanguageId::Php` (`.php`, `.phtml`), grammar `tree-sitter-php`, vector icon màu tím `php`, comment `//`, LSP `intelephense --stdio`.
