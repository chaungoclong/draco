# Task Workflow — Draco

Áp dụng cho mọi feature, bugfix hoặc refactor. Không nhảy cóc trạng thái:

```text
analyzing → planning → in_progress → ready_for_test → ready_for_review → done (hoặc blocked)
```

## Quy tắc cứng (Non-Negotiable)
1. **Nhánh từ `main`**: `main` là protected. Luôn `git fetch origin main` và checkout nhánh mới rõ nghĩa (`feat/<topic>`, `fix/<topic>`, `ci/<topic>`, `docs/<topic>`).
2. **Mỗi task phải có file theo dõi trong `tasks/`**: Tạo/cập nhật 1 file đánh số `tasks/NNN-<ten-task>.md` (ghi rõ status, phạm vi, acceptance criteria, kế hoạch & báo cáo test) và đồng bộ chỉ mục tại `tasks/INDEX.md`.
3. **Luôn hỏi approve trước khi `git push`**: Tuyệt đối không tự ý `git push` khi chưa có xác nhận rõ ràng của dev (trừ khi dev yêu cầu push trực tiếp trong lệnh hiện tại).
4. **Test Integrity**:
   - Không làm yếu, bỏ qua hay xóa test có sẵn để pass; mọi task mới đều phải có test tương ứng.
   - **Chỉ được sửa test cũ khi spec thay đổi và test cũ không còn phù hợp với spec mới — nhưng BẮT BUỘC phải hỏi và được dev đồng ý trước khi sửa.**
5. **Kiến trúc Draco**:
   - Không phụ thuộc `egui`/`eframe` trong `src/core/`, `src/syntax/`, `src/workspace/`, `src/lsp/`, `src/terminal/`, `src/plugins/`, `src/settings.rs`.
   - Không dùng `egui::TextEdit` cho main buffer; giữ reactive repaint (0% idle CPU); không thêm `tokio`.
   - An toàn UTF-8; áp dụng multi-edit / LSP `WorkspaceEdit` theo thứ tự offset ngược.

---

## Các bước thực thi

### 1. `analyzing` → `planning`
- Đọc `AGENTS.md`, spec liên quan trong `docs/specs/` và code mục tiêu.
- Tạo file task `tasks/NNN-<ten-task>.md` (status `analyzing` → `planning`) và thêm dòng vào `tasks/INDEX.md`.
- Xác định phạm vi (file sửa, keybinding không xung đột, hiệu năng O(visible) / O(depth), test cần thêm).
- Nếu là tính năng mới/lớn: tạo spec tách riêng theo **module/chức năng** tại `docs/specs/NNN-<module-or-feature>.md` (không gom nhiều mảng vào 1 file).

### 2. `in_progress`
- Cập nhật status file task sang `in_progress`.
- Triển khai vertical slice gọn, thật (không mock/fake trong production code).
- Tự rà diff trước khi test: đúng phạm vi, không lọt secret / file tạm (`tmp/`, `*.mp4`, `.agents/`), tương thích ngược.

### 3. `ready_for_test` (Quality Gates & Docs Sync)
Chạy các cổng kiểm tra:
```bash
cargo check --all-targets                  # Gate 1: 0 errors, 0 warnings
cargo clippy --all-targets -- -D warnings  # Gate 2: Rust linter (0 warnings)
cargo test -- --test-threads=4             # Gate 3: lib + unit + feature + headless E2E
cargo build --release                      # Gate 4: khi cần kiểm tra bản release/UI
```

**Cập nhật mọi tài liệu liên quan trong cùng nhánh:**
| Khi thay đổi | Tài liệu bắt buộc cập nhật |
|---|---|
| Mọi task | `tasks/NNN-<ten-task>.md` (kèm Test Report), `tasks/INDEX.md` |
| Mọi feature / bugfix / UX tweak | `docs/CHANGELOG.md` |
| Tính năng, luồng UI, phím tắt người dùng | `docs/guides/user-guide.md` |
| Phím tắt / thao tác chuột / context menu | `docs/guides/keybindings.md` |
| Spec tính năng hoặc module mới | `docs/specs/NNN-<feature>.md`, `docs/specs/README.md`, `docs/README.md` |
| Kiến trúc, module mới, điểm mở rộng | `docs/architecture.md`, `docs/tutorials/`, `AGENTS.md` (Module Map) |

### 4. `ready_for_review` → `done`
- Ghi rõ kết quả test (`PASS / FAIL / NOT RUN` kèm số lượng) vào file task, chuyển status sang `ready_for_review`, và commit local.
- **Hỏi approve dev trước khi `git push`**, sau đó hỗ trợ mô tả PR vào `main`.
- Chỉ chuyển sang `done` sau khi CI pass và PR được duyệt/merge.

### 5. `blocked`
- Đổi status file task sang `blocked`, ghi rõ lỗi cụ thể và phương án đã thử; chỉ hỏi dev khi cần quyết định sản phẩm, quyền truy cập hoặc xin đồng ý sửa test cũ theo spec mới.
