# Task Workflow — Draco

Áp dụng cho mọi feature, bugfix hoặc refactor. Không nhảy cóc trạng thái:

```text
analyzing → planning → in_progress → ready_for_test → ready_for_review → done (hoặc blocked)
```

## Quy tắc cứng (Non-Negotiable)
1. **Nhánh từ `main`**: `main` là protected. Luôn `git fetch origin main` và checkout nhánh mới rõ nghĩa (`feat/<topic>`, `fix/<topic>`, `ci/<topic>`, `docs/<topic>`).
2. **Luôn hỏi approve trước khi `git push`**: Tuyệt đối không tự ý `git push` khi chưa có xác nhận rõ ràng của người dùng (trừ khi người dùng yêu cầu push trực tiếp trong lệnh hiện tại).
3. **Test Integrity**: Không sửa, làm yếu, bỏ qua hay xóa test có sẵn để pass. Mọi thay đổi đều phải có test tương ứng; nếu test cũ fail, sửa implementation (trừ khi yêu cầu mới cố ý đổi hành vi — lúc đó dừng lại hỏi người dùng).
4. **Kiến trúc Draco**:
   - Không phụ thuộc `egui`/`eframe` trong `src/core/`, `src/syntax/`, `src/workspace/`, `src/lsp/`, `src/terminal/`, `src/plugins/`, `src/settings.rs`.
   - Không dùng `egui::TextEdit` cho main buffer; giữ reactive repaint (0% idle CPU); không thêm `tokio`.
   - An toàn UTF-8; áp dụng multi-edit / LSP `WorkspaceEdit` theo thứ tự offset ngược.

---

## Các bước thực thi

### 1. `analyzing` → `planning`
- Đọc `AGENTS.md`, spec liên quan trong `docs/specs/` và code mục tiêu.
- Xác định phạm vi (file sửa, keybinding không xung đột, hiệu năng O(visible) / O(depth), test cần thêm).
- Nếu là tính năng mới/lớn: tạo spec tách riêng theo **module/chức năng** tại `docs/specs/NNN-<module-or-feature>.md` (không gom nhiều mảng vào 1 file).

### 2. `in_progress`
- Triển khai vertical slice gọn, thật (không mock/fake trong production code).
- Tự rà diff trước khi test: đúng phạm vi, không lọt secret / file tạm (`tmp/`, `*.mp4`, `.agents/`), tương thích ngược.

### 3. `ready_for_test` (Quality Gates & Docs Sync)
Chạy các cổng kiểm tra:
```bash
cargo check --all-targets          # Gate 1: 0 errors, 0 warnings
cargo test -- --test-threads=4     # Gate 2: lib + unit + feature + headless E2E
cargo build --release              # Gate 3: khi cần kiểm tra bản release/UI
```

**Cập nhật mọi tài liệu liên quan trong cùng nhánh:**
| Khi thay đổi | Tài liệu bắt buộc cập nhật |
|---|---|
| Mọi feature / bugfix / UX tweak | `docs/CHANGELOG.md` |
| Tính năng, luồng UI, phím tắt người dùng | `docs/guides/user-guide.md` |
| Phím tắt / thao tác chuột / context menu | `docs/guides/keybindings.md` |
| Spec tính năng hoặc module mới | `docs/specs/NNN-<feature>.md`, `docs/specs/README.md`, `docs/README.md` |
| Kiến trúc, module mới, điểm mở rộng | `docs/architecture.md`, `docs/tutorials/`, `AGENTS.md` (Module Map) |

### 4. `ready_for_review` → `done`
- Báo cáo ngắn gọn kết quả test (`PASS / FAIL / NOT RUN` kèm số lượng) và commit local.
- **Hỏi approve người dùng trước khi `git push`**, sau đó hỗ trợ mô tả PR vào `main`.
- Chỉ coi là `done` sau khi CI pass và PR được duyệt/merge.

### 5. `blocked`
- Ghi rõ lỗi cụ thể và phương án đã thử; chỉ hỏi người dùng khi cần quyết định sản phẩm, quyền truy cập hoặc xung đột yêu cầu.
