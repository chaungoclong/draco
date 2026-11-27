# AGENTS.md — Draco Code Editor

## Project Overview
**Draco** is a lightweight, fast, GPU-accelerated GUI code editor written in Rust using `eframe` / `egui`, `ropey`, `tree-sitter`, LSP (JSON-RPC over stdio), and an embedded PTY terminal (`portable-pty` + `vte`).

## Core Engineering Principles
1. **Performance First**:
   - Never use `egui::TextEdit` for the main code buffer. Always render code through the custom virtualized `CodeView` (`src/ui/code_view.rs`) backed by `ropey::Rope`, painting only visible lines (`first_visible_line..=last_visible_line`).
   - Keep `eframe` in reactive repaint mode (0% CPU when idle). Background threads call `ctx.request_repaint()` only when new data arrives.
   - Use incremental Tree-sitter parsing (`tree.edit(&InputEdit)`) and restrict `QueryCursor` to the visible byte range.
   - Avoid heavy async runtimes (`tokio`). Use `std::thread` + `std::sync::mpsc` for background I/O (LSP, PTY, file indexing).
2. **Strict Layer Separation**:
   - `src/core/`, `src/syntax/`, `src/workspace/`, `src/lsp/`, and `src/terminal/` must **not** depend on `egui` or `eframe` (except optional lightweight repaint callback closures `Arc<dyn Fn() + Send + Sync>`).
   - This keeps all non-UI logic fast to unit-test headlessly with `cargo test`.
3. **UTF-8 & Offset Safety**:
   - Always respect UTF-8 character boundaries when slicing or moving cursors in `ropey::Rope`.
   - Apply multi-cursor edits in reverse offset order (highest byte offset first) so earlier offsets remain valid.
4. **Modern Keybindings**:
   - Follow standard VS Code / Sublime shortcuts (`Ctrl+S`, `Ctrl+O`, `Ctrl+N`, `Ctrl+W`, `Ctrl+Z`, `Ctrl+Y`/`Ctrl+Shift+Z`, `Ctrl+F`, `Ctrl+H`, `Ctrl+P`, `Ctrl+B`, ``Ctrl+` ``, `Ctrl+/`, `Ctrl+D`, `F12`, `Ctrl+Space`).

## Module Map
- `src/main.rs` — Binary entry point, CLI path args, `eframe` bootstrap.
- `src/app.rs` — Root `DracoApp` state coordinating tabs, workspace, LSP, terminal, modals, and `eframe::App::update`.
- `src/theme.rs` — Color palette (Dark/Light) and syntax highlight token colors.
- `src/keymap.rs` — Action enum and keyboard shortcut dispatch.
- `src/core/` — `Document` (`ropey` + Undo/Redo), `SelectionSet` (multi-cursor), `editing` (auto-indent, brackets, comments, line ops), `search` (plain/regex Find & Replace).
- `src/syntax/` — Tree-sitter `SyntaxEngine`, language registry, incremental parse, visible-range highlight spans.
- `src/workspace/` — Lazy `FileTree`, background `ignore`-based file indexer, fuzzy matcher for Quick Open.
- `src/lsp/` — JSON-RPC stdio transport and `LspManager` (diagnostics, completion, hover, go-to-definition).
- `src/terminal/` — `TerminalGrid` (`vte::Perform` ANSI/VT100 state machine) and `PtySession` (`portable-pty`).
- `src/ui/` — `CodeView`, `explorer`, `tabs`, `quick_open`, `find_bar`, `completion`, `terminal_panel`, `status_bar`.

## Documentation Maintenance
- Keep `docs/` synchronized with code changes:
  - Record user-facing changes, fixes, and new features in `docs/CHANGELOG.md`.
  - Add new technical specs under `docs/specs/NNN-feature-name.md` and list them in `docs/specs/README.md`.
  - Update `docs/guides/keybindings.md`, `docs/architecture.md`, or `docs/tutorials/` whenever shortcuts, architecture, or extension points change.

## Commands
- Check: `cargo check`
- Test: `cargo test`
- Build release: `cargo build --release`
- Run: `cargo run --release -- [optional_path]`
