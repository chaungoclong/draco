# Draco — Technical Specification

## 1. Goals
- Fast startup and low memory footprint.
- Smooth 60–120 FPS editing and scrolling even on 100,000+ line files.
- Clean, modern UI with essential IDE capabilities out of the box.

## 2. Functional Requirements

### 2.1 Core Editor
- Rope-backed text buffer (`ropey::Rope`) supporting arbitrary UTF-8 files.
- Multi-tab document management with dirty indicators (`●`) and native Open/Save dialogs (`rfd`).
- Grouped Undo/Redo history preserving cursor states.
- Virtualized viewport rendering with monospace metrics, line-number gutter, active line highlight, and smooth scrolling.
- Clipboard integration (`arboard`) with cut/copy/paste and line-copy fallback.

### 2.2 Syntax Highlighting
- Incremental Tree-sitter parsing on every document edit.
- Visible-range highlight queries (`QueryCursor::set_byte_range`) for O(visible lines) frame cost.
- Built-in languages: Rust, Python, JavaScript, TypeScript, JSON, TOML, C, Markdown.

### 2.3 Workspace & Navigation
- Collapsible File Explorer sidebar (`Ctrl+B`) with lazy directory expansion.
- Background file indexer honoring `.gitignore` (`ignore` crate).
- Quick Open modal (`Ctrl+P`) with fuzzy scoring and keyboard navigation.
- Find & Replace bar (`Ctrl+F` / `Ctrl+H`) supporting case sensitivity, whole word, regex, match highlighting, and atomic Replace All.

### 2.4 Advanced Editing
- Multi-cursor editing (`Alt+Click`, `Ctrl+D` add next occurrence, `Ctrl+Alt+Up/Down`, `Esc` collapse).
- Auto-indent on `Enter` (preserving indentation and indenting after `{`, `(`, `[`, `:`).
- Auto-closing pairs (`()`, `[]`, `{}`, `""`, `''`) and matching bracket highlight.
- Line comment toggle (`Ctrl+/`), indent/outdent (`Tab`/`Shift+Tab`), move line (`Alt+Up/Down`), duplicate line (`Shift+Alt+Down`).

### 2.5 Language Server Protocol (LSP)
- Automatic server discovery in `PATH` (`rust-analyzer`, `pyright-langserver`/`pylsp`, `typescript-language-server`).
- JSON-RPC 2.0 over stdio on background threads.
- Features: real-time diagnostics (squiggly underlines + hover tooltip), completion popup (`Ctrl+Space` / trigger chars), hover info, and Go to Definition (`F12` / `Ctrl+Click`).

### 2.6 Embedded Terminal
- Toggleable bottom terminal panel (``Ctrl+` ``) backed by `portable-pty`.
- ANSI/VT100 terminal grid (`vte::Perform`) with 256/truecolor SGR, cursor control, scrollback buffer, and automatic PTY resize.

## 3. Non-Functional Targets
- Idle CPU usage: ~0% (reactive repaint).
- Opening & querying visible highlights on a 100,000-line file: < 10ms per frame.
- All core, syntax, workspace, LSP framing, and terminal grid modules covered by automated unit tests (`cargo test`).
