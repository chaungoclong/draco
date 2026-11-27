# Tutorial: Viết & Cài đặt Plugin cho Draco

Draco hỗ trợ hệ thống **Command Plugin** cực gọn nhẹ — bạn có thể viết plugin bằng bất kỳ ngôn ngữ nào (Python, Bash, Node.js, Rust, PHP…) mà không cần biên dịch lại Draco.

## 1. Cấu trúc một Plugin

Tạo một thư mục chứa file `plugin.json` (và các script kèm theo nếu cần):

```text
my-json-formatter/
└── plugin.json
```

Ví dụ `plugin.json`:

```json
{
  "id": "json-formatter",
  "name": "JSON Formatter",
  "version": "1.0.0",
  "description": "Format nội dung JSON hiện tại",
  "author": "Your Name",
  "commands": [
    {
      "id": "format",
      "title": "Format JSON",
      "executable": "python3",
      "args": ["-m", "json.tool"],
      "input_mode": "selection_or_document",
      "output_mode": "replace_document"
    }
  ]
}
```

## 2. Các chế độ `input_mode` và `output_mode`

| `input_mode` | Dữ liệu truyền vào `stdin` của plugin |
|---|---|
| `none` | Không truyền stdin |
| `selection` | Chỉ truyền vùng đang bôi đen |
| `document` | Truyền toàn bộ nội dung file |
| `selection_or_document` | Truyền vùng chọn nếu có, ngược lại truyền cả file |

| `output_mode` | Cách Draco áp dụng `stdout` trả về |
|---|---|
| `status` | Hiển thị dòng đầu tiên lên thanh Status Bar |
| `insert` | Chèn tại vị trí con trỏ |
| `replace_selection` | Thay thế vùng đang chọn |
| `replace_document` | Thay thế toàn bộ nội dung file (có hỗ trợ `Ctrl+Z` Undo) |

## 3. Biến môi trường được Draco cung cấp
Khi chạy lệnh plugin, Draco tự động gán:
- `DRACO_FILE_PATH`: Đường dẫn file hiện tại (nếu đã lưu)
- `DRACO_WORKSPACE_ROOT`: Thư mục gốc workspace
- `DRACO_LANGUAGE`: Ngôn ngữ hiện tại (vd: `Rust`, `PHP`, `Python`)
- `DRACO_LINE`, `DRACO_COL`: Dòng và cột hiện tại (1-based)

## 4. Cài đặt & Sử dụng Plugin
1. Mở **Settings (`Ctrl+,`)** → chuyển sang tab **Plugins**.
2. Nhấn **Install Plugin Folder…** và chọn thư mục chứa `plugin.json` (Draco sẽ sao chép vào `~/.config/draco/plugins/<id>/`).
3. Bạn cũng có thể đặt plugin cục bộ cho riêng dự án tại `.draco/plugins/<id>/plugin.json`.
4. Nhấn **`Ctrl+Shift+P`** mở **Command Palette**, gõ tên lệnh plugin và nhấn `Enter` để chạy!
