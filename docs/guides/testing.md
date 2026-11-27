# Hướng dẫn kiểm thử (Testing Guide)

Draco tổ chức kiểm thử theo **3 thư mục tách biệt rõ ràng** trong `tests/` kèm helper dùng chung `tests/common/mod.rs` và chạy hoàn toàn **headless** (không cần X11/Wayland):

```text
tests/
├── common/mod.rs                 # TempDir (RAII auto-cleanup), headless egui RawInput helpers
├── unit.rs                       # Entry point cho tầng Unit Tests
├── unit/
│   ├── buffer_tests.rs           # Rope, UTF-8/emoji, CRLF, Undo/Redo, Save/Save As, PHP detection
│   ├── selection_tests.rs        # Multi-cursor merge, clamp, reversed, collapse, empty fallback
│   ├── editing_tests.rs          # Auto-indent, auto-close, comments (Rust/Py/PHP/Plain), line ops, page nav
│   ├── search_tests.rs           # Plain, case, whole-word, UTF-8 regex, wrap-around, atomic replace undo
│   ├── syntax_tests.rs           # Cả 9 ngôn ngữ (gồm PHP), chuyển ngôn ngữ động, incremental undo sync
│   ├── workspace_tests.rs        # FileTree lazy-load, ẩn .git, sắp xếp, fuzzy score edge cases
│   ├── settings_tests.rs         # Auto-Save mặc định ON, partial/corrupt JSON fallback, plugin flags
│   ├── plugins_tests.rs          # Reject id/manifest lỗi, disable plugin ẩn & chặn thực thi
│   ├── lsp_transport_tests.rs    # LF/CRLF headers, UTF-8 byte length, truncated body error
│   └── terminal_grid_tests.rs    # ANSI bright/256/truecolor, tab/backspace, line wrap, scrollback cap
├── feature.rs                    # Entry point cho tầng Feature Integration Tests
├── feature/
│   ├── editing_workflow.rs       # Luồng hoàn chỉnh Rust & PHP (open -> edit -> highlight -> save -> reload)
│   ├── workspace_quick_open.rs   # Workspace + .gitignore + lazy FileTree + background indexer + Quick Open
│   ├── terminal_pty.rs           # PTY shell chạy lệnh thật, resize, multi-tab TerminalManager & 4 layouts
│   ├── lsp_lifecycle.rs          # JSON-RPC 2.0 thật với mock LSP server (init, diagnostics, completion, def, hover)
│   └── plugin_lifecycle.rs       # Install global plugin + workspace `.draco/plugins`, env `DRACO_*`, uninstall
└── e2e.rs                        # Entry point cho tầng Headless E2E GUI Tests
    └── e2e/
        ├── code_view_e2e.rs      # Giả lập phím thật qua CodeView (Rust & PHP, auto-indent, Ctrl+/, Ctrl+Z/Y)
        ├── panels_e2e.rs         # Explorer (chấm dirty), Tabs, Breadcrumbs, Activity Bar, Status Bar, 4 layouts, Dark/Light
        ├── modals_e2e.rs         # Quick Open, Completion Popup, Command Palette, Settings Modal (3 tabs), FindBar
        └── performance_e2e.rs    # Benchmark render & chỉnh sửa tại dòng 50.000 trên file 100.000 dòng
```

## Lệnh chạy kiểm thử

```bash
# Chạy toàn bộ test (src inline + tests/unit + tests/feature + tests/e2e)
cargo test

# Chạy riêng từng tầng
cargo test --test unit
cargo test --test feature
cargo test --test e2e

# Chạy riêng 1 module cụ thể
cargo test --test unit buffer_tests
cargo test --test feature plugin_lifecycle
cargo test --test e2e modals_e2e
```
