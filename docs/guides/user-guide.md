# Hướng Dẫn Sử Dụng Draco (End-User Tutorial)

> **Cập nhật lần cuối**: 2026-11-28  
> Tài liệu hướng dẫn chi tiết từng bước dành cho người dùng cuối của **Draco** — trình soạn thảo mã nguồn nhẹ, nhanh, tăng tốc GPU viết bằng Rust, kết hợp sự gọn nhẹ của VS Code và khả năng điều hướng/chỉnh sửa thông minh của JetBrains.

---

## Mục lục
1. [Khởi chạy & Mở dự án](#1-khởi-chạy--mở-dự-án)
2. [Làm quen với giao diện Draco](#2-làm-quen-với-giao-diện-draco)
3. [Soạn thảo thông minh, Multi-Cursor & AST Selection](#3-soạn-thảo-thông-minh-multi-cursor--ast-selection)
4. [Điều hướng nhanh (Quick Open, Recent Files, Symbols, Sticky Scroll, Overview Ruler)](#4-điều-hướng-nhanh)
5. [Tìm kiếm & Thay thế (Trong file & Toàn Workspace)](#5-tìm-kiếm--thay-thế)
6. [Chia đôi màn hình (Split Editor)](#6-chia-đôi-màn-hình-split-editor)
7. [Markdown Preview, Mục lục nổi & Sơ đồ Mermaid](#7-markdown-preview-mục-lục-nổi--sơ-đồ-mermaid)
8. [Terminal tích hợp nhiều tab](#8-terminal-tích-hợp-nhiều-tab)
9. [Theo dõi thay đổi Git ngay trên lề (Git Gutter)](#9-theo-dõi-thay-đổi-git-ngay-trên-lề-git-gutter)
10. [Trợ lý ngôn ngữ thông minh (LSP: Autocomplete, Quick-Fix, Rename, Inlay Hints)](#10-trợ-lý-ngôn-ngữ-thông-minh-lsp)
11. [Cài đặt, Auto-Save, Giao diện & Plugin](#11-cài-đặt-auto-save-giao-diện--plugin)
12. [Mẹo xử lý sự cố nhanh](#12-mẹo-xử-lý-sự-cố-nhanh)

---

## 1. Khởi chạy & Mở dự án

Bạn có thể mở Draco kèm thư mục dự án hoặc một file cụ thể ngay từ terminal:

```bash
# Mở cả thư mục hiện tại làm workspace
draco .

# Mở một thư mục dự án bất kỳ
draco /duong/dan/toi/du-an

# Mở trực tiếp một hoặc nhiều file
draco src/main.rs docs/README.md
```

Hoặc sau khi đã mở cửa sổ Draco:
- **Mở thư mục (Workspace)**: nhấn `Ctrl+Shift+O` (hoặc bấm nút mở thư mục trong File Explorer). Khi chuyển sang thư mục mới, Draco tự động đóng các tab và dọn sạch lịch sử/kết quả tìm kiếm của thư mục cũ.
- **Mở 1 file lẻ**: nhấn `Ctrl+O`.
- **Tạo file mới**: nhấn `Ctrl+N`.
- **Lưu file**: nhấn `Ctrl+S` (`Ctrl+Shift+S` để Lưu dưới tên khác). Mặc định **Auto-Save đang BẬT**, Draco tự lưu file sau khi bạn ngừng gõ.

---

## 2. Làm quen với giao diện Draco

Cửa sổ Draco gồm 5 khu vực tối giản:
1. **Activity Bar (cột icon mảnh bên trái, 44px)**:
   - **Files (`Ctrl+B`)**: bật/tắt Sidebar trái.
   - **Search (`Ctrl+F`)**: bật/tắt thanh tìm kiếm trong file hiện tại.
   - **Quick Open (`Ctrl+P`)**: mở/đóng hộp điều hướng nhanh.
   - **Terminal (``Ctrl+` ``)**: bật/tắt Terminal nội bộ.
   - **Theme (`Ctrl+Shift+T`)** & **Settings (`Ctrl+,`)** ở góc dưới cùng.
2. **Sidebar trái (`Explorer | Search`)**:
   - Chuyển nhanh giữa cây thư mục **Explorer** (chuột phải để Tạo file, Tạo thư mục, Đổi tên, Xóa, Copy đường dẫn) và **Search** (tìm kiếm/thay thế toàn workspace).
3. **Tab Bar & Breadcrumbs (phía trên)**:
   - Chấm tròn `●` cạnh tên file báo hiệu file chưa lưu (**Modified**); nhấn **Middle-click** hoặc `Ctrl+W` để đóng tab; chuột phải lên tab để mở menu nhanh.
4. **Khu vực soạn thảo chính (`CodeView` / Split / Markdown Preview)**:
   - Hỗ trợ đầy đủ tiếng Việt / UTF-8, tô màu cú pháp tức thì cho 9 ngôn ngữ (Rust, Python, JavaScript, TypeScript, PHP, C/C++, JSON, TOML, Markdown) và 5 định dạng cấu hình (`YAML`, `.env`, `INI/CONF/nginx`, `Shell`, `Dockerfile`).
5. **Status Bar (thanh trạng thái đáy)**:
   - Hiển thị dòng/cột hiện tại, số con trỏ, ngôn ngữ, trạng thái LSP, số lỗi/cảnh báo, trạng thái `✓ Saved` / `● Modified` và nút bật/tắt nhanh **Auto-Save**, **Theme**, **Terminal**, **Settings**.

---

## 3. Soạn thảo thông minh, Multi-Cursor & AST Selection

### Chỉnh sửa cơ bản & Tự động hỗ trợ
- **Auto-close & Wrap**: gõ `(`, `[`, `{`, `"`, `'` sẽ tự đóng cặp; bôi đen một đoạn rồi gõ `"` hoặc `(` sẽ bọc quanh đoạn đó.
- **Auto-indent**: nhấn `Enter` sau `{` hoặc `:` tự động thụt lề đúng chuẩn.
- **Comment nhanh**: nhấn `Ctrl+/` để bật/tắt comment cho dòng hiện tại hoặc nhiều dòng đang chọn.
- **Di chuyển / Nhân bản dòng**:
  - `Alt+Up` / `Alt+Down`: di chuyển dòng lên/xuống.
  - `Shift+Alt+Down`: nhân bản dòng xuống dưới.
- **Thụt lề**: `Tab` để thụt vào, `Shift+Tab` để đẩy ra.

### Multi-Cursor (Sửa nhiều chỗ cùng lúc)
- **`Ctrl+D`**: chọn từ tại con trỏ; nhấn tiếp `Ctrl+D` để chọn thêm các lần xuất hiện kế tiếp và sửa đồng thời.
- **`Alt+Click`**: đặt thêm một con trỏ mới tại bất kỳ vị trí nào bạn click.
- **`Ctrl+Alt+Up` / `Ctrl+Alt+Down`**: thêm con trỏ lên dòng trên / xuống dòng dưới.
- **`Esc`**: thu gọn tất cả về 1 con trỏ chính.

### AST Extend / Shrink Selection (Chọn thông minh theo cú pháp — kiểu JetBrains/VS Code)
- Đặt con trỏ vào bất kỳ từ/biểu thức nào trong file code:
  - Nhấn **`Shift+Alt+Right`** liên tiếp để mở rộng vùng chọn theo từng cấp cây cú pháp Tree-sitter: `biến → biểu thức → câu lệnh → khối { ... } → toàn bộ hàm`.
  - Nhấn **`Shift+Alt+Left`** để thu gọn ngược lại đúng từng bước trước đó.

---

## 4. Điều hướng nhanh

### Hộp điều hướng hợp nhất (`Files | Recent | Symbols`)
Draco gộp 3 công cụ tìm nhanh vào một hộp thoại duy nhất; khi hộp đang mở bạn có thể nhấn phím **`Tab`** (hoặc click) để chuyển qua lại giữa 3 tab, dùng `↑`/`↓` + `Enter` để chọn, và nhấn `Esc` hoặc bấm ra ngoài để đóng:

- **`Ctrl+P` — Tab `Files` (Quick Open)**: tìm file mờ (fuzzy) toàn workspace theo chuẩn tốc độ cao (`nucleo-matcher`), tự bỏ qua các file trong `.gitignore`.
- **`Ctrl+E` — Tab `Recent` (Recent Files)**: mở danh sách các file bạn vừa làm việc theo thứ tự mới nhất trước — cực nhanh khi chuyển qua lại giữa 2–3 file.
- **`Ctrl+R` — Tab `Symbols` (Go to Symbol / Code Outline)**: liệt kê toàn bộ `struct`, `enum`, `trait`, `impl`, `fn`, `class`, `method`, `interface`, hoặc tiêu đề Markdown trong file hiện tại; gõ vài ký tự và `Enter` để nhảy thẳng tới dòng định nghĩa.

### Sticky Scroll (Ghim ngữ cảnh ở đầu khung code)
- Khi bạn cuộn sâu vào trong một hàm, `impl` hoặc `class` dài, Draco tự động ghim tối đa **3 dòng tiêu đề scope bao quanh** ở mép trên khung code.
- **Click vào bất kỳ dòng ghim nào** để nhảy ngay về dòng mở đầu của khối đó.

### Overview Ruler (Thanh định vị mép phải)
- Dải dọc mảnh 10px bên mép phải `CodeView` hiển thị bản đồ toàn file:
  - Vạch **đỏ**: lỗi LSP.
  - Vạch **vàng**: cảnh báo LSP.
  - Vạch **xanh accent**: vị trí kết quả Find (`Ctrl+F`).
  - Vạch **sáng**: vị trí con trỏ hiện tại.
- **Click hoặc kéo chuột** dọc thanh Overview Ruler để nhảy tức thì tới bất kỳ phần nào của file.

---

## 5. Tìm kiếm & Thay thế

### Trong file hiện tại (`Ctrl+F` / `Ctrl+H`)
- Nhấn `Ctrl+F` để mở thanh tìm kiếm; `Ctrl+H` để mở kèm ô thay thế.
- Bật/tắt tùy chọn: `Aa` (phân biệt hoa/thường), `W` (nguyên từ), `.*` (biểu thức chính quy Regex).
- Nhấn `Enter` / `Shift+Enter` (hoặc nút `↓` / `↑`) để nhảy giữa các kết quả; `Replace` thay 1 chỗ, `Replace All` thay toàn bộ và gom thành **1 bước Undo duy nhất** (`Ctrl+Z` hoàn tác được tất cả).

### Toàn bộ Workspace (`Ctrl+Shift+F`)
- Nhấn **`Ctrl+Shift+F`** (hoặc bấm tab **Search** ở đầu Sidebar trái).
- Gõ từ khóa cần tìm — kết quả được quét trên luồng nền, tự động tuân theo `.gitignore` và bỏ qua file nhị phân.
- Bấm **`⋯`** để lọc theo **Include** (vd: `*.rs, src/`) và **Exclude** (vd: `tests/, *.lock`).
- Click vào bất kỳ dòng kết quả nào để mở file và nhảy đúng dòng/cột.
- Bấm **`⇄`** để mở ô **Replace** và thay thế hàng loạt trên toàn workspace (các tab đang mở chưa chỉnh sửa cục bộ sẽ tự reload nội dung mới).

---

## 6. Chia đôi màn hình (Split Editor)

- Nhấn **`Ctrl+\`** để chia đôi khung soạn thảo thành 2 pane trái & phải.
- Bạn có thể:
  - Xem và sửa **2 file khác nhau** song song.
  - Hoặc xem **2 vị trí khác nhau của cùng một file** (cuộn độc lập từng pane).
- Click vào pane nào thì viền màu accent ở mép trên pane đó sẽ sáng lên để báo hiệu pane đang nhận bàn phím.
- Trên thanh tiêu đề pane phải, dùng **`‹`** / **`›`** để đổi file hiển thị hoặc bấm **`×`** (hay `Ctrl+\`) để đóng chế độ chia đôi.

---

## 7. Markdown Preview, Mục lục nổi & Sơ đồ Mermaid

Khi mở bất kỳ file `.md` nào, thanh công cụ góc phải Breadcrumbs cung cấp:
- **Chế độ hiển thị (`Ctrl+Shift+V`)**: `Editor` (chỉ code) | `Split` (chia đôi) | `Preview` (xem toàn chiều rộng).
- **Đồng bộ cuộn 2 chiều (`Sync`)**: cuộn bên code thì preview tự cuộn theo, cuộn bên preview thì code tự cuộn theo mượt mà.
- **Mục lục nổi (`Outline`) kiểu Google Docs**: ghim gọn ở góc trên phải, thu gọn được thành nút `☰ Outline`; click vào một mục sẽ cuộn thẳng tới tiêu đề đó, giữ trạng thái active xanh trên Outline và tạo hiệu ứng nhấn sáng (flash) tiêu đề trong văn bản.
- **Điều hướng link trực tiếp**: click link tương đối (`./file.md`, `../doc.md#muc`) để mở thẳng trong Draco; click `#anchor` (hỗ trợ cả tiếng Việt có dấu) để cuộn trong trang; click `https://...` để mở trình duyệt.
- **Render trực tiếp 23 loại sơ đồ Mermaid**: viết khối ```` ```mermaid ```` (flowchart, sequence, class, state, ER, gantt, pie, mindmap, gitGraph, timeline…) và xem sơ đồ sắc nét 2x HiDPI ngay trong Preview, kèm nút chuyển `Diagram | Source` và `Copy`.

---

## 8. Terminal tích hợp nhiều tab

- Nhấn **``Ctrl+` ``** để bật/tắt Terminal (dùng shell thật của hệ thống với đầy đủ màu ANSI).
- Nhấn **``Ctrl+Shift+` ``** (hoặc nút **`+`**) để mở thêm nhiều tab Terminal; bấm **Middle-click** hoặc **`×`** để đóng tab.
- Chuyển nhanh giữa **4 kiểu bố trí** ngay góc phải tiêu đề Terminal:
  - `Bottom` (gắn phía dưới — kéo mép trên để đổi chiều cao).
  - `Right` (gắn bên phải).
  - `Float` (cửa sổ nổi tự do).
  - `Max` (phóng to toàn khung).

---

## 9. Theo dõi thay đổi Git ngay trên lề (Git Gutter)

Khi bạn làm việc trong một thư mục có Git, Draco tự động chạy `git diff` trên luồng nền mỗi khi mở hoặc lưu file và vẽ vạch màu 3px ngay sát cột số dòng:
- **Vạch xanh lá**: dòng mới thêm so với commit gần nhất (`HEAD`).
- **Vạch xanh dương**: dòng đã bị chỉnh sửa.
- **Vạch đỏ nhỏ ở chân dòng**: vị trí có dòng bị xóa.

---

## 10. Trợ lý ngôn ngữ thông minh (LSP)

Draco tự động kết nối tới Language Server đã cài trên máy (theo ngôn ngữ của file đang mở):

| Ngôn ngữ | LSP Server mặc định |
|---|---|
| Rust | `rust-analyzer` |
| Python | `pyright-langserver --stdio` |
| JavaScript / TypeScript | `typescript-language-server --stdio` |
| PHP | `intelephense --stdio` |
| C / C++ | `clangd` |

Khi Status Bar hiển thị **`LSP: Ready`**, bạn có đầy đủ các tính năng IDE:
- **Diagnostics**: gạch chân lượn sóng đỏ/vàng, chấm màu trên gutter, vạch trên Overview Ruler và di chuột để xem chi tiết lỗi.
- **Autocomplete (`Ctrl+Space` hoặc gõ `.`, `:`)**: gợi ý hoàn thành mã nguồn bằng `↑`/`↓` + `Enter`.
- **Go to Definition (`F12` hoặc `Ctrl+Click`)**: nhảy tới nơi định nghĩa hàm/struct/biến.
- **Quick-Fix / Code Actions (`Alt+Enter`)**: mở menu gợi ý sửa lỗi hoặc refactor tức thì tại con trỏ.
- **Rename Symbol (`F2`)**: đổi tên biến/hàm/class an toàn đồng bộ trên toàn workspace.
- **Inlay Hints**: tự hiển thị nhãn kiểu dữ liệu & tên tham số dạng pill mờ cuối dòng.
- **Signature Help**: khi gõ `(` hoặc `,` trong lời gọi hàm, hiện tooltip chữ ký hàm ngay phía trên con trỏ (`Esc` để ẩn).

---

## 11. Cài đặt, Auto-Save, Giao diện & Plugin

Nhấn **`Ctrl+,`** để mở cửa sổ **Settings & Plugins** (lưu tại thư mục cấu hình chuẩn OS, vd `~/.config/draco/settings.json` trên Linux):
- **Tab `Editor`**: bật/tắt **Auto-Save**, chỉnh độ trễ lưu tự động (ms), cỡ chữ (`Ctrl+=` / `Ctrl+-` / `Ctrl+0`), độ rộng Tab, Dark / Light theme (`Ctrl+Shift+T`).
- **Tab `Terminal`**: chọn bố cục Terminal mặc định (`Bottom`, `Right`, `Floating`, `Maximized`).
- **Tab `Plugins`**: cài đặt thư mục plugin (`Install Plugin Folder…`), bật/tắt hoặc gỡ bỏ plugin; chạy lệnh plugin qua **Command Palette (`Ctrl+Shift+P`)**.
- **Tự động reload file ngoài**: nếu file đang mở bị chỉnh sửa từ bên ngoài (vd `git checkout`, formatter) và bạn chưa có thay đổi cục bộ chưa lưu, Draco sẽ tự động cập nhật nội dung mới.

---

## 12. Mẹo xử lý sự cố nhanh

- **Không thấy LSP Autocomplete / Quick-Fix / Rename**: kiểm tra xem LSP server của ngôn ngữ đó đã có trong `PATH` chưa (vd chạy `which rust-analyzer` trong Terminal) và xem thông báo ở Status Bar góc dưới.
- **Không thấy vạch Git Gutter**: đảm bảo thư mục đang là một Git repository đã có ít nhất 1 commit và file đã được lưu (`Ctrl+S` hoặc đợi Auto-Save).
- **Muốn xem nhanh toàn bộ phím tắt**: tham khảo bảng tra cứu gọn tại [keybindings.md](./keybindings.md).
