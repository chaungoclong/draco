# Spec 003 — Config Highlighting & High-Performance Markdown Preview

- **Trạng thái**: `Implemented` (v0.3.0)
- **Phạm vi**: `src/syntax/config_lexer.rs`, `src/ui/markdown_preview.rs`, `src/ui/icons.rs`, `src/core/mod.rs`, `src/app.rs`

## 1. Config File Highlighting (`src/syntax/config_lexer.rs`)
- Hỗ trợ tô màu cú pháp nhanh O(visible lines) cho các định dạng cấu hình phổ biến:
  - **YAML** (`.yml`, `.yaml`): document markers (`---`, `...`), keys, strings, booleans/null, numbers, anchors/aliases, comments.
  - **DotEnv** (`.env`, `.env.*`): `export`, keys, `=`, strings, `${VAR}` interpolation, booleans, numbers, comments.
  - **INI / CONF / Nginx / Properties** (`.conf`, `.ini`, `.cfg`, `.cnf`, `.properties`, `.editorconfig`, `.gitconfig`, `.npmrc`): `[section]`, `key = value`, nginx directives/blocks, `#` & `;` comments, numbers + units (`60s`, `512mb`).
  - **Shell** (`.sh`, `.bash`, `.zsh`): shebang, keywords, `$VAR`/`${VAR}`, strings, comments.
  - **Dockerfile** (`Dockerfile`, `Dockerfile.*`, `*.dockerfile`): instructions (`FROM`, `RUN`, `COPY`, `ENV`, `EXPOSE`...), strings, variables, numbers.
- Icon vector có mã màu riêng trong `src/ui/icons.rs`.

## 2. PhpStorm-Style Markdown Preview (`src/ui/markdown_preview.rs`)
- 3 chế độ chuyển nhanh ngay góc trên phải vùng editor (hoặc `Ctrl+Shift+V`): **`Editor`** | **`Split`** (mặc định cho `.md`) | **`Preview`**.
- **Version-cached AST + `pulldown-cmark`**: parse chuẩn CommonMark + GFM bằng `pulldown-cmark` và chỉ parse lại khi `(doc.id, doc.version)` thay đổi → 0ms parse khi cuộn/di chuột.
- **Interactive Links & Floating Google Docs-style Outline (`Outline`)**:
  - Bảng Mục lục (Outline) nổi cố định góc trên phải với nền bán trong suốt (`alpha=212`), tự highlight mục đang đọc theo vị trí cuộn, có thể ẩn/thu gọn thành nút nhỏ `☰ Outline` không che nội dung.
  - **Heading Flash/Pulse Feedback**: khi nhấn vào một mục trên Outline (hoặc link `#anchor`), kể cả khi mục đó đã nằm trong màn hình, heading tương ứng sẽ phát sáng một nhịp với vạch accent và tắt dần mượt trong 950ms (sau đó trả về 0% CPU).
  - Hỗ trợ cuộn tới heading bằng slug tiếng Việt có dấu/không dấu, mở trực tiếp file nội bộ từ link tương đối (`./file.md#section`) và mở URL ngoài bằng trình duyệt.
- **Bidirectional Block-Mapped Scroll Sync (`Sync`)**:
  - Đồng bộ 2 chiều **Editor ↔ Preview** bằng nội suy tuyến tính thuận/nghịch (`compute_preview_scroll_y` & `compute_editor_line_from_preview_y`) theo dải dòng nguồn từ `pulldown-cmark::into_offset_iter()`, có cơ chế chống vòng lặp phản hồi (echo suppression).
- **Mermaid Rendering (`mermaid-svg` + `resvg`, `src/ui/mermaid.rs`)**:
  - Render **23 loại sơ đồ Mermaid** (`flowchart`, `sequence`, `state`, `class`, `ER`, `gantt`, `pie`, `mindmap`, `gitGraph`, `C4`, `architecture`, `kanban`, `timeline`, `xychart`…) bằng thư viện thuần Rust `mermaid-svg` (0 `unsafe`, 0 `build.rs`, chỉ 1 dependency `thiserror`, MIT) → rasterize SVG ở 2x HiDPI qua `resvg` (chính thức từ Linebender, dùng `fontdb` hệ thống hỗ trợ tiếng Việt) → cache GPU `egui::TextureHandle` (120 FPS, 0ms CPU khi cuộn), kèm nút `Diagram | Source` và `Copy`.
- **Full-Width Responsive Layout & Lag-Free Scrolling**:
  - Giãn đầy chiều rộng vùng Preview (cả chế độ `Split` và `Preview` toàn màn hình), nạp font **Bold** thật qua `fontdb` (`draco_bold`) cho tiêu đề & `**bold**`, gỡ bỏ nested `ScrollArea` trong code block để cuộn mượt không giật, kèm nút **Copy** trên mỗi code block.
- **Smooth Block-Mapped Scroll Sync (`Sync`)**:
  - `pulldown-cmark::Parser::into_offset_iter()` ánh xạ dải dòng nguồn chính xác (`start_line..=end_line`) cho từng `MdBlock`.
  - Ghi nhận tọa độ `top_y..bottom_y` thực tế của từng khối trong Preview, nội suy tuyến tính từng đoạn (`compute_preview_scroll_y`) theo `top_visible_line` dạng số thực từ `CodeView`.
  - Làm mượt bằng nội suy tắt dần (exponential damping), tự dừng repaint khi ổn định (0% CPU lúc đứng yên) và không kéo ngược khi người dùng tự cuộn bên Preview.
- Hỗ trợ H1–H6, **bold**, *italic*, `inline code`, ~~strikethrough~~, `[link](url)`, Blockquote, Task list (`- [x]`/`- [ ]`), Bullet/Ordered list, GFM Table, Horizontal Rule và **tô màu cú pháp ngay bên trong fenced code block**.

## 3. UTF-8 / Vietnamese Font Loading (`src/ui/fonts.rs`)
- Nạp tự động font hệ thống hỗ trợ đầy đủ tiếng Việt (`U+1E00..U+1EFF`) vào `FontFamily::Monospace` và `FontFamily::Proportional` cùng `unicode-width` trong `CodeView`.
