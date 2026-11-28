# AGENTS.md — Draco Code Editor

## Project Overview
**Draco** is a lightweight, fast, GPU-accelerated GUI code editor written in Rust using `eframe` / `egui`, `ropey`, `tree-sitter`, LSP (JSON-RPC over stdio), and an embedded PTY terminal (`portable-pty` + `vte`).

## Task Execution Workflow (Mandatory)
- Follow [docs/task-workflow.md](docs/task-workflow.md) for every feature, bugfix, or refactor (`analyzing → planning → in_progress → ready_for_test → ready_for_review → done`).
- `main` is protected: always sync `origin/main` first and checkout a clear, descriptive branch (e.g., `feat/<topic>`, `fix/<topic>`, `ci/<topic>`, `docs/<topic>`).
- **Always ask for approval before `git push`**: never run `git push` without explicit user confirmation (unless the user explicitly requested pushing in the current prompt).
- **Test integrity (Non-Negotiable)**: never weaken, skip, or delete existing tests to pass; add dedicated tests for every new feature/fix and fix the implementation if a test fails.

## Core Engineering Principles
1. **Performance First**:
   - Never use `egui::TextEdit` for the main code buffer. Always render code through the custom virtualized `CodeView` (`src/ui/code_view.rs`) backed by `ropey::Rope`, painting only visible lines (`first_visible_line..=last_visible_line`).
   - Keep `eframe` in reactive repaint mode (0% CPU when idle). Background threads call `ctx.request_repaint()` only when new data arrives.
   - Use incremental Tree-sitter parsing (`tree.edit(&InputEdit)`) and restrict `QueryCursor` to the visible byte range (or O(tree depth) parent walks for Sticky Scroll / AST selection).
   - Avoid heavy async runtimes (`tokio`). Use `std::thread` + `std::sync::mpsc` for background I/O (LSP, PTY, file indexing, workspace search, git diff).
2. **Strict Layer Separation**:
   - `src/core/`, `src/syntax/`, `src/workspace/`, `src/lsp/`, `src/terminal/`, `src/plugins/`, and `src/settings.rs` must **not** depend on `egui` or `eframe` (except optional lightweight repaint callback closures `Arc<dyn Fn() + Send + Sync>`).
   - This keeps all non-UI logic fast to unit-test headlessly with `cargo test`.
3. **UTF-8 & Offset Safety**:
   - Always respect UTF-8 character boundaries when slicing or moving cursors in `ropey::Rope`.
   - Apply multi-cursor edits and LSP `WorkspaceEdit`s in reverse offset order (highest offset first) so earlier offsets remain valid.
4. **Modern Keybindings**:
   - Follow standard VS Code / JetBrains / Sublime shortcuts without conflicts (`Ctrl+S`, `Ctrl+O`, `Ctrl+N`, `Ctrl+W`, `Ctrl+Z`, `Ctrl+Y`/`Ctrl+Shift+Z`, `Ctrl+F`, `Ctrl+H`, `Ctrl+Shift+F`, `Ctrl+P`, `Ctrl+E`, `Ctrl+R`, `Ctrl+Shift+P`, `Ctrl+B`, `Ctrl+\`, ``Ctrl+` ``, `Ctrl+/`, `Ctrl+D`, `Shift+Alt+Right`/`Left`, `Alt+Enter`, `F2`, `F12`, `Ctrl+Space`).

## Module Map
- `src/main.rs` — Binary entry point, CLI path args, `eframe` bootstrap.
- `src/app.rs` — Root `DracoApp` state coordinating tabs, split editor, workspace, LSP, terminal, modals, and `eframe::App::update`.
- `src/theme.rs` — Color palette (Dark/Light) and syntax highlight token colors.
- `src/keymap.rs` — Action enum and global keyboard shortcut dispatch.
- `src/settings.rs` — OS-standard config persistence (`directories`) and Auto-Save/Terminal/Plugin settings.
- `src/plugins/` — Local plugin manifest loader, installer, and command runner.
- `src/core/` — `Document` (`ropey` + Undo/Redo), `SelectionSet` (multi-cursor), `editing` (auto-indent, brackets, comments, line ops), `search` (plain/regex Find & Replace).
- `src/syntax/` — Tree-sitter `SyntaxState`, language registry, `config_lexer` (YAML/ENV/INI/Shell/Dockerfile), `ast` (AST Extend/Shrink Selection), `symbols` (Code Outline & O(depth) Sticky Scroll).
- `src/workspace/` — Lazy `FileTree`, background `ignore`-based file indexer + `nucleo-matcher` fuzzy matcher, `watcher` (`notify` auto-reload), `search` (Workspace Find & Replace), `git_diff` (Git Gutter).
- `src/lsp/` — JSON-RPC stdio transport and `LspManager` (diagnostics, completion, hover, go-to-definition, code actions, rename, inlay hints, signature help).
- `src/terminal/` — `TerminalGrid` (`vte::Perform` ANSI/VT100 state machine), `PtySession` (`portable-pty`), and `TerminalManager` (multi-tab + 4 layouts).
- `src/ui/` — `code_view` (virtualized editor, Sticky Scroll, Overview Ruler, Git Gutter, Inlay Hints), `explorer`, `workspace_search`, `tabs`, `quick_open` (Files/Recent/Symbols), `command_palette`, `find_bar`, `completion`, `lsp_popups`, `markdown_preview` (PhpStorm-style + Google Docs Outline + Mermaid), `mermaid`, `terminal_panel`, `settings_modal`, `status_bar`, `fonts`, `icons`.

## Documentation Maintenance (Mandatory on Every Change)
Whenever the application changes (feature, bugfix, UI/UX tweak, shortcut, config, or architecture), you **MUST** update **all** relevant documentation in the same branch/PR before marking a task ready for review:
1. **`docs/CHANGELOG.md`** — Record every user-facing feature, change, and bugfix.
2. **`docs/guides/user-guide.md`** — Keep the end-user handbook accurate whenever user-facing features, UI flows, or shortcuts are added or changed.
3. **`docs/guides/keybindings.md`** — Update whenever any keyboard shortcut or mouse/context-menu interaction is added or modified.
4. **`docs/specs/NNN-<module-or-feature>.md` & `docs/specs/README.md`** — Create/update focused technical specs **split by module/feature** (avoid bundling unrelated features into one monolithic spec) and keep `docs/specs/README.md` synchronized.
5. **`docs/README.md`** — Keep the Documentation Hub index updated with all specs, guides, and tutorials.
6. **`docs/architecture.md`, `docs/tutorials/`, & `AGENTS.md` (Module Map)** — Update whenever modules, threading/rendering architecture, or extension points change.

## Commands
- Check: `cargo check --all-targets`
- Test: `cargo test`
- Build release: `cargo build --release`
- Run: `cargo run --release -- [optional_path]`
