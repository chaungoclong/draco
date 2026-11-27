use eframe::egui::{self, Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    NewFile,
    OpenFile,
    OpenFolder,
    Save,
    SaveAs,
    CloseTab,
    NextTab,
    PrevTab,
    ToggleSidebar,
    ToggleTerminal,
    NewTerminalTab,
    ToggleTheme,
    OpenQuickOpen,
    OpenCommandPalette,
    OpenSettings,
    OpenFind,
    OpenReplace,
    ToggleMarkdownPreview,
    GoToDefinition,
    TriggerCompletion,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

/// Checks global window-level shortcuts (Ctrl+S, Ctrl+P, Ctrl+B, Ctrl+`, etc.).
pub fn poll_global_action(ctx: &egui::Context) -> Option<AppAction> {
    ctx.input_mut(|i| {
        if i.consume_key(Modifiers::COMMAND, Key::N) {
            return Some(AppAction::NewFile);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::O) {
            return Some(AppAction::OpenFolder);
        }
        if i.consume_key(Modifiers::COMMAND, Key::O) {
            return Some(AppAction::OpenFile);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            return Some(AppAction::SaveAs);
        }
        if i.consume_key(Modifiers::COMMAND, Key::S) {
            return Some(AppAction::Save);
        }
        if i.consume_key(Modifiers::COMMAND, Key::W) {
            return Some(AppAction::CloseTab);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::P) {
            return Some(AppAction::OpenCommandPalette);
        }
        if i.consume_key(Modifiers::COMMAND, Key::P) {
            return Some(AppAction::OpenQuickOpen);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Comma) {
            return Some(AppAction::OpenSettings);
        }
        if i.consume_key(Modifiers::COMMAND, Key::F) {
            return Some(AppAction::OpenFind);
        }
        if i.consume_key(Modifiers::COMMAND, Key::H) {
            return Some(AppAction::OpenReplace);
        }
        if i.consume_key(Modifiers::COMMAND, Key::B) {
            return Some(AppAction::ToggleSidebar);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Backtick) {
            return Some(AppAction::NewTerminalTab);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Backtick) {
            return Some(AppAction::ToggleTerminal);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::V) {
            return Some(AppAction::ToggleMarkdownPreview);
        }
        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::T) {
            return Some(AppAction::ToggleTheme);
        }
        if i.consume_key(Modifiers::CTRL, Key::Tab) {
            return Some(AppAction::NextTab);
        }
        if i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab) {
            return Some(AppAction::PrevTab);
        }
        if i.consume_key(Modifiers::NONE, Key::F12) {
            return Some(AppAction::GoToDefinition);
        }
        if i.consume_key(Modifiers::CTRL, Key::Space) {
            return Some(AppAction::TriggerCompletion);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Plus)
            || i.consume_key(Modifiers::COMMAND, Key::Equals)
        {
            return Some(AppAction::ZoomIn);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Minus) {
            return Some(AppAction::ZoomOut);
        }
        if i.consume_key(Modifiers::COMMAND, Key::Num0) {
            return Some(AppAction::ZoomReset);
        }
        None
    })
}
