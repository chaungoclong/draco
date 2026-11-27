# Keybindings & Thao tác chuột

Draco sử dụng bộ phím tắt chuẩn hiện đại (VS Code / Sublime). Trên macOS, thay `Ctrl` bằng `Cmd` cho các lệnh `COMMAND`.

## File & Tab
| Phím tắt | Chức năng |
|---|---|
| `Ctrl+N` | Tạo file trống mới |
| `Ctrl+O` | Mở file |
| `Ctrl+Shift+O` | Mở thư mục workspace |
| `Ctrl+S` / `Ctrl+Shift+S` | Lưu / Lưu dưới tên khác |
| `Ctrl+W` hoặc Middle-click tab | Đóng tab hiện tại |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Chuyển tab tiếp theo / trước đó |

## Điều hướng & Giao diện
| Phím tắt | Chức năng |
|---|---|
| `Ctrl+P` | Mở Quick Open (tab `Files` — chuyển tab `Files \| Recent \| Symbols` bằng `Tab`) |
| `Ctrl+E` | Mở Recent Files (danh sách file vừa làm việc theo thứ tự MRU) |
| `Ctrl+R` | Mở Go to Symbol (nhảy nhanh tới hàm, struct, class, heading trong file hiện tại) |
| `Ctrl+Shift+P` | Mở Command Palette (chạy lệnh & plugin) |
| `Ctrl+,` | Mở Settings & Plugin Manager |
| `Ctrl+B` | Ẩn/hiện File Explorer |
| ``Ctrl+` `` | Bật/tắt Terminal nội bộ |
| ``Ctrl+Shift+` `` | Tạo tab Terminal mới |
| `Ctrl+Shift+T` | Đổi Dark / Light theme |
| `Ctrl+Shift+V` | Chuyển chế độ Markdown (`Editor` → `Split` → `Preview`) |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Phóng to / Thu nhỏ / Đặt lại cỡ chữ |

## Soạn thảo & Multi-cursor
| Phím tắt | Chức năng |
|---|---|
| `Ctrl+Z` / `Ctrl+Y` (`Ctrl+Shift+Z`) | Undo / Redo |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` / `Ctrl+A` | Copy / Cut / Paste / Select All |
| `Ctrl+/` | Bật/tắt comment dòng |
| `Tab` / `Shift+Tab` | Thụt lề / Giảm thụt lề |
| `Alt+Up` / `Alt+Down` | Di chuyển dòng lên / xuống |
| `Shift+Alt+Down` | Nhân bản dòng |
| `Shift+Alt+Right` / `Shift+Alt+Left` | Mở rộng / Thu gọn vùng chọn theo cú pháp AST (Tree-sitter) |
| `Ctrl+D` | Chọn từ hiện tại / thêm lần xuất hiện tiếp theo |
| `Ctrl+Alt+Up` / `Ctrl+Alt+Down` | Thêm con trỏ lên / xuống |
| `Alt+Click` | Đặt thêm con trỏ tại vị trí chuột |
| `Esc` | Thu gọn về 1 con trỏ chính (hoặc đóng popup) |

## Tìm kiếm & LSP
| Phím tắt | Chức năng |
|---|---|
| `Ctrl+F` / `Ctrl+H` | Mở Find / Find & Replace |
| `Ctrl+Space` | Gọi gợi ý LSP Autocomplete |
| `F12` hoặc `Ctrl+Click` | Go to Definition |

## Context Menu (Chuột phải)
- **Trong CodeView**: Cut, Copy, Paste, Select All, Add Next Match, Toggle Comment, Go to Definition, Autocomplete, Find, Replace.
- **Trong File Explorer**: New File, New Folder, Rename, Delete, Copy Path, Copy Relative Path, Collapse All, Refresh.
- **Trên Tab Bar**: Save, Close, Close Others, Close All, Copy Path.
