# Changelog

Tất cả thay đổi đáng chú ý của **Draco** đều được ghi nhận tại đây (theo chuẩn [Keep a Changelog](https://keepachangelog.com/)).

## [Unreleased]

### Added
- **Native OS File Watcher & Auto-Reload (`notify`, `src/workspace/watcher.rs`)**:
  - Tự động reload các file đang mở (khi chưa có chỉnh sửa cục bộ) nếu bị thay đổi từ bên ngoài (vd: `git checkout`, formatter, script) và tự làm mới File Explorer / Quick Open khi thêm/xóa file.

### Changed
- **Quick Open Fuzzy Matcher (`nucleo-matcher`)**: Nâng cấp `src/workspace/fuzzy.rs` sang `nucleo-matcher` (SIMD, Unicode smart-case & path matching từ Helix Editor).
- **Cross-Platform Font Discovery (`fontdb`)**: Nâng cấp `src/ui/fonts.rs` sang `fontdb` để tự khám phá font tiếng Việt / UTF-8 qua `fontconfig` / thư mục hệ thống trên Linux, macOS, Windows.
- **OS-Standard Config Paths (`directories`)**: Nâng cấp `src/settings.rs` sang `directories::ProjectDirs`.
- **Highlight cho các file cấu hình phổ biến (`src/syntax/config_lexer.rs`)**:
  - Hỗ trợ `YAML` (`.yml`/`.yaml`), `.env` (`.env`, `.env.*`), `INI/CONF/nginx.conf/properties/.editorconfig`, `Shell` (`.sh`/`.bash`/`.zsh`) và `Dockerfile` chạy trực tiếp trên vùng byte hiển thị (O(visible lines), < 1ms) kèm icon vector riêng.
- **Markdown Preview mượt theo phong cách PhpStorm + Outline kiểu Google Docs (`src/ui/markdown_preview.rs`)**:
  - Bố cục full-width linh hoạt (tự giãn đầy chiều rộng kể cả khi mở full Preview), nạp font **Bold** thật qua `fontdb` cho tiêu đề & chữ đậm, gỡ bỏ nested `ScrollArea` trong code block giúp cuộn cực mượt, thêm nút **Copy** trên code block.
  - **Floating Outline cố định trong suốt kiểu Google Docs**: bảng mục lục nổi cố định góc trên phải, tự highlight mục đang đọc, có **hiệu ứng flash/highlight 950ms** lên heading khi click (kể cả khi heading đã ở trên màn hình), thu gọn được thành nút `☰ Outline`.
  - **Đồng bộ cuộn 2 chiều (`Editor ↔ Preview`)**: cuộn bên code hoặc cuộn bên preview đều tự động cuộn phía còn lại tới đúng khối tương ứng bằng nội suy tuyến tính thuận/nghịch không bị vòng lặp phản hồi.
  - **Render trực tiếp 23 loại sơ đồ Mermaid (`mermaid-svg` + `resvg`, `src/ui/mermaid.rs`)**: hỗ trợ đầy đủ `flowchart`, `sequence`, `state`, `class`, `ER`, `gantt`, `pie`, `mindmap`, `gitGraph`, `C4`, `architecture`, `kanban`, `timeline`, `xychart`… đồng bộ Dark/Light theme, rasterize 2x HiDPI qua `resvg` (hỗ trợ tiếng Việt) và cache texture GPU (0ms CPU khi cuộn), kèm nút `Diagram | Source` và `Copy`.
  - 3 chế độ `Editor` | `Split` | `Preview` chuyển nhanh trên thanh Breadcrumbs hoặc `Ctrl+Shift+V`, kèm nút bật/tắt **Outline** và **Đồng bộ cuộn (`Sync`)**.
  - **Smooth Block-Mapped Scroll Sync**: cuộn bên code tự động cuộn mượt Preview tới đúng khối tương ứng bằng nội suy tuyến tính theo dải dòng nguồn (`into_offset_iter()`) + damping mượt mà (0% CPU khi dừng).
  - Click link tương đối (`./file.md`, `../file.md#anchor`) mở trực tiếp file trong Draco và cuộn tới heading; click `#anchor` (hỗ trợ cả slug tiếng Việt có dấu & không dấu) cuộn mượt trong trang; click `https://` mở trình duyệt.
  - Sử dụng thư viện chuẩn `pulldown-cmark` (CommonMark + GFM) kết hợp cache theo `doc.version`, hỗ trợ H1–H6, inline formatting, blockquote, task list, bảng GFM và tô màu cú pháp ngay trong code block.
- **Khắc phục hiển thị tiếng Việt / UTF-8 (`src/ui/fonts.rs`, `unicode-width`)**:
  - Tự động nạp font hệ thống hỗ trợ đầy đủ Unicode / tiếng Việt (`NotoSansMono`, `NotoSans`, `DejaVuSansMono`, `DejaVuSans`, `Liberation`, `Ubuntu`, macOS/Windows fallbacks) vào cả `Monospace` và `Proportional`.
  - Dùng `unicode-width` trong `CodeView` để căn tọa độ con trỏ và vùng chọn chính xác cho ký tự Unicode.
- **Multi-Tab Terminal & 4 Display Modes (`src/terminal/manager.rs`, `src/ui/terminal_panel.rs`)**:
  - Hỗ trợ tạo/chuyển/đóng nhiều tab terminal (`+`, `Ctrl+Shift+` `, middle-click).
  - 4 chế độ hiển thị: `Bottom`, `Right`, `Floating`, `Maximized`.
- **Settings & Auto-Save (`src/settings.rs`, `src/ui/settings_modal.rs`)**:
  - Lưu cấu hình bền vững tại `~/.config/draco/settings.json` và giao diện Settings (`Ctrl+,`).
  - **Auto-Save mặc định BẬT** (có nút bật/tắt nhanh ngay trên Status Bar & Settings, tùy chỉnh độ trễ).
- **File Modified / Saved Status Indicators**:
  - Hiển thị rõ `● Modified`, `✓ Saved`, `● Unsaved` trên Breadcrumbs, Status Bar, Tab Bar và chấm `●` trên File Explorer.
- **Plugin System & Command Palette (`src/plugins/`, `src/ui/command_palette.rs`)**:
  - Cho phép viết, cài đặt (`Install Plugin Folder…`), bật/tắt, gỡ bỏ plugin định dạng `plugin.json` và chạy lệnh qua Command Palette (`Ctrl+Shift+P`).
- **Ngôn ngữ PHP**:
  - Hỗ trợ `tree-sitter-php`, file icon PHP, comment `//`, và LSP `intelephense`.
- **Tái cấu trúc & mở rộng bộ test (`tests/unit/`, `tests/feature/`, `tests/e2e/`, `tests/common/`)**:
  - Tách riêng 10 file unit test, 5 file feature test và 4 file headless E2E test với coverage sâu cho UTF-8, CRLF, multi-cursor, 9 ngôn ngữ, settings, plugins, LSP và multi-tab terminal.
- Thư mục tài liệu chuẩn hóa `docs/` gồm Architecture, Specs (001, 002), Guides, Tutorials (thêm ngôn ngữ & viết plugin) và Changelog.
- Quy tắc bảo trì tài liệu trong `AGENTS.md` cho các thay đổi tương lai.

### Fixed
- Sửa lỗi mất chữ bên trong sơ đồ Mermaid trên Linux bằng cách ánh xạ rõ `set_sans_serif_family` / `set_monospace_family` và `opts.font_family` cho `resvg`.
- Sửa lỗi giật/kéo ngược khi cuộn 2 chiều & nhấn Outline bằng quy tắc Master/Follower theo con trỏ chuột và nhảy trực tiếp tới `heading_y_positions`.
- Sửa lỗi link inline trong danh sách Markdown bị rớt dòng, sửa thứ tự nút `Sync | Outline | Editor | Split | Preview` và đo chiều rộng tab theo font metrics thật.
- Sửa triệt để lỗi nhấp nháy ở ranh giới resize giữa Editor và Terminal (thêm vùng đệm 6px cho `CentralPanel` cạnh Terminal để thanh cuộn ngang/dọc không đè lên vùng kéo 5px của panel, chuyển scrollbar `CodeView` sang `ScrollStyle::solid()` 8px, và tránh ghi đè `CursorIcon`), đồng thời giữ active xanh trên Outline cho mục vừa nhấn kể cả khi trang đã chạm đáy cuộn.
- Sửa lỗi không thể tắt Find & Replace và Quick Open từ Activity Bar: nhấn lại icon `Search` / `QuickOpen` trên Activity Bar nay sẽ đóng/mở (toggle) đúng chuẩn; thêm nút `×` và hỗ trợ bấm ra ngoài (click-outside) để đóng Quick Open và Command Palette.

## [0.1.0] - 2026-11-28

### Added
- **Core Editor (`src/core/`)**:
  - Buffer văn bản dựa trên `ropey::Rope` hỗ trợ UTF-8 và CRLF/LF.
  - Hệ thống Undo/Redo gom nhóm (edit coalescing) bảo toàn trạng thái con trỏ.
  - Multi-cursor (`SelectionSet`) hỗ trợ `Alt+Click`, `Ctrl+D`, `Ctrl+Alt+Up/Down`, `Esc`.
  - Editing thông minh: auto-indent, auto-close cặp ngoặc/nháy, skip-over ngoặc đóng, wrap selection, bracket matching, comment toggle (`Ctrl+/`), indent/outdent, di chuyển/nhân bản dòng.
  - Find & Replace hỗ trợ Plain, Case-sensitive, Whole word, Regex và Replace All gom 1 bước Undo.
- **Tree-sitter Syntax Highlighting (`src/syntax/`)**:
  - Parse tăng dần (`tree.edit`) đọc zero-copy từ chunk của `Rope` và truy vấn highlight giới hạn trong vùng byte hiển thị.
  - Tích hợp sẵn 8 ngôn ngữ: Rust, Python, JavaScript, TypeScript, JSON, TOML, C/C++, Markdown.
- **Workspace & Navigation (`src/workspace/`)**:
  - File Explorer lazy-load hỗ trợ tạo file, tạo thư mục, đổi tên, xóa và copy đường dẫn.
  - Background indexer dùng `ignore` tôn trọng `.gitignore` + fuzzy matcher cho Quick Open (`Ctrl+P`).
- **LSP Client (`src/lsp/`)**:
  - Giao tiếp JSON-RPC 2.0 `Content-Length` qua stdio trên luồng nền (`rust-analyzer`, `pyright-langserver`, `typescript-language-server`, `clangd`).
  - Diagnostics (gạch chân + chấm gutter + tooltip), Autocomplete popup (`Ctrl+Space`), Go to Definition (`F12` / `Ctrl+Click`), Hover.
- **Embedded Terminal (`src/terminal/`)**:
  - Terminal nội bộ dùng `portable-pty` + `vte` hỗ trợ ANSI 16/256/truecolor, scrollback 2000 dòng và tự resize.
- **Refined UI (`src/ui/`)**:
  - Custom virtualized `CodeView` với indent guides, diagnostic gutter dots, Welcome overlay và font zoom (`Ctrl+=`/`Ctrl+-`/`Ctrl+0`).
  - Hệ thống icon vector thuần `egui::Painter` (`icons.rs`) cho file theo ngôn ngữ, thư mục và Activity Bar (44px).
  - Tab Bar với file icon, middle-click đóng tab, Tab Context Menu và Breadcrumbs.
  - Context Menu chuột phải đầy đủ cho `CodeView`, `Explorer` và `Tabs`.
  - Dark (Tokyo Night) và Light theme (`Ctrl+Shift+T`).
- **Testing**:
  - 34 automated tests (26 unit tests, 4 feature integration tests, 4 headless E2E UI tests).

### Fixed
- Sửa lỗi dịch offset con trỏ trong `toggle_comment` khi bật/tắt comment đồng thời bằng nhiều con trỏ trên nhiều dòng.
