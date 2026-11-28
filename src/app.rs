use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::core::buffer::{Document, EditKind};
use crate::core::editing;
use crate::core::search;
use crate::core::selection::{Selection, SelectionSet};
use crate::core::LanguageId;
use crate::keymap::{self, AppAction};
use crate::lsp::client::{DiagnosticItem, LspEvent, LspManager};
use crate::plugins::{PluginContext, PluginInputMode, PluginManager, PluginOutputMode};
use crate::settings::{Settings, TerminalLayout};
use crate::syntax::SyntaxState;
use crate::terminal::manager::TerminalManager;
use crate::theme::{Theme, ThemeKind};
use crate::ui::code_view::{self, CodeViewContextAction, CodeViewState};
use crate::ui::command_palette::{self, CommandPaletteState, PaletteCommand, PaletteEntry};
use crate::ui::completion::{self, CompletionState};
use crate::ui::explorer::{self, ExplorerAction, ExplorerUiState};
use crate::ui::find_bar::{self, FindAction, FindBarState};
use crate::ui::fonts;
use crate::ui::icons::{self, ActivityIcon};
use crate::ui::markdown_preview::{self, MarkdownPreviewCache, MarkdownViewMode, PreviewAction};
use crate::ui::quick_open::{self, QuickOpenAction, QuickOpenState, QuickOpenTab};
use crate::ui::settings_modal::{self, SettingsModalAction, SettingsModalState};
use crate::ui::status_bar::{self, StatusAction, StatusInfo};
use crate::ui::tabs::{self, TabAction, TabInfo};
use crate::ui::terminal_panel::{self, TerminalUiAction};
use crate::ui::workspace_search::{self, WorkspaceSearchAction, WorkspaceSearchUiState};
use crate::workspace::file_tree::FileTree;
use crate::workspace::fuzzy::WorkspaceIndexer;
use crate::workspace::search::{replace_in_workspace, WorkspaceSearcher};
use crate::workspace::watcher::{WatchEvent, WorkspaceWatcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SidebarMode {
    #[default]
    Explorer,
    Search,
}

struct OpenTab {
    doc: Document,
    syntax: SyntaxState,
    view: CodeViewState,
    last_edit: Option<Instant>,
}

impl OpenTab {
    fn from_doc(doc: Document, font_size: f32) -> Self {
        let lang = doc.language;
        let mut view = CodeViewState::default();
        view.font_size = font_size;
        Self {
            doc,
            syntax: SyntaxState::new(lang),
            view,
            last_edit: None,
        }
    }
}

pub struct DracoApp {
    tabs: Vec<OpenTab>,
    active_tab: usize,
    next_doc_id: u64,
    settings: Settings,
    settings_modal: SettingsModalState,
    plugins: PluginManager,
    command_palette: CommandPaletteState,
    file_tree: Option<FileTree>,
    explorer_ui: ExplorerUiState,
    sidebar_mode: SidebarMode,
    workspace_search_ui: WorkspaceSearchUiState,
    workspace_searcher: WorkspaceSearcher,
    indexer: WorkspaceIndexer,
    watcher: Option<WorkspaceWatcher>,
    last_tree_refresh: Instant,
    lsp: LspManager,
    diagnostics: HashMap<PathBuf, Vec<DiagnosticItem>>,
    terminal: TerminalManager,
    show_sidebar: bool,
    show_terminal: bool,
    terminal_focused: bool,
    split_tab: Option<usize>,
    split_right_focused: bool,
    split_view: CodeViewState,
    theme: Theme,
    quick_open: QuickOpenState,
    recent_files: Vec<PathBuf>,
    find_bar: FindBarState,
    completion: CompletionState,
    md_mode: MarkdownViewMode,
    md_cache: MarkdownPreviewCache,
    clipboard: Option<arboard::Clipboard>,
    status_message: Option<String>,
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl DracoApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_paths: Vec<PathBuf>) -> Self {
        let ctx_clone = cc.egui_ctx.clone();
        let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            ctx_clone.request_repaint();
        });

        let settings = Settings::load_default();
        let theme = if settings.theme == "light" {
            Theme::light()
        } else {
            Theme::dark()
        };
        fonts::configure_utf8_fonts(&cc.egui_ctx);
        apply_egui_visuals(&cc.egui_ctx, &theme);

        let terminal_layout = settings.terminal_layout;
        let mut plugins = PluginManager::default_manager();
        plugins.reload(None, &settings);

        let mut split_view = CodeViewState::default();
        split_view.font_size = settings.font_size;
        split_view.scroll_salt = 1;

        let mut app = Self {
            tabs: Vec::new(),
            active_tab: 0,
            next_doc_id: 1,
            settings,
            settings_modal: SettingsModalState::default(),
            plugins,
            command_palette: CommandPaletteState::default(),
            file_tree: None,
            explorer_ui: ExplorerUiState::default(),
            sidebar_mode: SidebarMode::Explorer,
            workspace_search_ui: WorkspaceSearchUiState::default(),
            workspace_searcher: WorkspaceSearcher::default(),
            indexer: WorkspaceIndexer::default(),
            watcher: None,
            last_tree_refresh: Instant::now(),
            lsp: LspManager::new(Arc::clone(&repaint)),
            diagnostics: HashMap::new(),
            terminal: TerminalManager::new(terminal_layout),
            show_sidebar: true,
            show_terminal: false,
            terminal_focused: false,
            split_tab: None,
            split_right_focused: false,
            split_view,
            theme,
            quick_open: QuickOpenState::default(),
            recent_files: Vec::new(),
            find_bar: FindBarState::default(),
            completion: CompletionState::default(),
            md_mode: MarkdownViewMode::Split,
            md_cache: MarkdownPreviewCache::default(),
            clipboard: arboard::Clipboard::new().ok(),
            status_message: Some("Ready".to_string()),
            repaint,
        };

        for p in initial_paths {
            let abs = fs::canonicalize(&p).unwrap_or(p);
            if abs.is_dir() {
                app.open_workspace_folder(abs);
            } else if abs.is_file() {
                if app.file_tree.is_none() {
                    if let Some(parent) = abs.parent() {
                        app.open_workspace_folder(parent.to_path_buf());
                    }
                }
                app.open_file_path(abs);
            }
        }

        if app.file_tree.is_none() {
            if let Ok(cwd) = std::env::current_dir() {
                app.open_workspace_folder(cwd);
            }
        }

        if app.tabs.is_empty() {
            app.new_empty_tab();
        }

        app
    }

    fn alloc_doc_id(&mut self) -> u64 {
        let id = self.next_doc_id;
        self.next_doc_id += 1;
        id
    }

    fn copy_string(&mut self, ctx: &egui::Context, text: String) {
        if let Some(cb) = self.clipboard.as_mut() {
            let _ = cb.set_text(text.clone());
        } else {
            ctx.copy_text(text.clone());
        }
        self.status_message = Some(format!("Copied: {text}"));
    }

    fn new_empty_tab(&mut self) {
        let id = self.alloc_doc_id();
        self.tabs.push(OpenTab::from_doc(
            Document::new_empty(id),
            self.settings.font_size,
        ));
        self.active_tab = self.tabs.len() - 1;
        self.terminal_focused = false;
    }

    fn open_workspace_folder(&mut self, root: PathBuf) {
        self.lsp.set_root(root.clone());
        self.indexer
            .start_index(root.clone(), Arc::clone(&self.repaint));
        self.plugins.reload(Some(&root), &self.settings);
        self.watcher = WorkspaceWatcher::watch(&root, Arc::clone(&self.repaint)).ok();
        self.file_tree = Some(FileTree::open(root));
        self.status_message = Some("Workspace opened".to_string());
    }

    fn handle_watch_events(&mut self) {
        let events = match self.watcher.as_ref() {
            Some(w) => w.poll_events(),
            None => return,
        };
        if events.is_empty() {
            return;
        }
        let mut tree_dirty = false;
        for ev in events {
            match ev {
                WatchEvent::FileModified(path) => {
                    let canon = fs::canonicalize(&path).unwrap_or(path);
                    for tab in &mut self.tabs {
                        if !tab.doc.dirty && tab.doc.path.as_deref() == Some(canon.as_path()) {
                            if let Ok(true) = tab.doc.reload_from_disk() {
                                let text = tab.doc.rope.to_string();
                                self.lsp
                                    .did_change(tab.doc.language, &canon, tab.doc.version, &text);
                                self.status_message = Some(format!(
                                    "Reloaded {} (changed on disk)",
                                    tab.doc.title()
                                ));
                            }
                        }
                    }
                }
                WatchEvent::TreeChanged => {
                    tree_dirty = true;
                }
            }
        }
        if tree_dirty && self.last_tree_refresh.elapsed() >= Duration::from_millis(600) {
            self.last_tree_refresh = Instant::now();
            self.refresh_workspace();
        }
    }

    fn refresh_workspace(&mut self) {
        if let Some(tree) = self.file_tree.as_mut() {
            tree.refresh();
            self.indexer
                .start_index(tree.root.clone(), Arc::clone(&self.repaint));
        }
    }

    fn record_recent_file(&mut self, path: &Path) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_path_buf());
        if self.recent_files.len() > 30 {
            self.recent_files.truncate(30);
        }
    }

    fn open_file_path(&mut self, path: PathBuf) {
        let canonical = fs::canonicalize(&path).unwrap_or(path);
        self.record_recent_file(&canonical);
        if let Some(existing) = self
            .tabs
            .iter()
            .position(|t| t.doc.path.as_deref() == Some(canonical.as_path()))
        {
            self.active_tab = existing;
            self.terminal_focused = false;
            return;
        }

        let id = self.alloc_doc_id();
        match Document::from_file(id, canonical.clone()) {
            Ok(doc) => {
                let text = doc.rope.to_string();
                self.lsp.did_open(doc.language, &canonical, doc.version, &text);
                if self.tabs.len() == 1
                    && self.tabs[0].doc.path.is_none()
                    && !self.tabs[0].doc.dirty
                    && self.tabs[0].doc.len_chars() == 0
                {
                    self.tabs[0] = OpenTab::from_doc(doc, self.settings.font_size);
                    self.active_tab = 0;
                } else {
                    self.tabs
                        .push(OpenTab::from_doc(doc, self.settings.font_size));
                    self.active_tab = self.tabs.len() - 1;
                }
                self.terminal_focused = false;
                self.recompute_search();
                self.status_message = Some(format!("Opened {}", canonical.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Open failed: {e}"));
            }
        }
    }

    fn save_tab(&mut self, idx: usize, force_dialog: bool) {
        let Some(tab) = self.tabs.get_mut(idx) else {
            return;
        };
        let need_dialog = force_dialog || tab.doc.path.is_none();
        let res = if need_dialog {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                tab.doc.save_as(path)
            } else {
                return;
            }
        } else {
            tab.doc.save()
        };

        match res {
            Ok(()) => {
                tab.last_edit = None;
                if let Some(p) = &tab.doc.path {
                    self.lsp.did_save(tab.doc.language, p);
                    self.status_message = Some(format!("Saved {}", p.display()));
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Save error: {e}"));
            }
        }
    }

    /// Saves modified file-backed tabs after `auto_save_delay_ms` when Auto-Save is enabled.
    fn process_auto_save(&mut self, ctx: &egui::Context) {
        if !self.settings.auto_save {
            return;
        }
        let delay = Duration::from_millis(self.settings.auto_save_delay_ms.max(100));
        let mut soonest_remaining: Option<Duration> = None;

        for tab in &mut self.tabs {
            if tab.doc.dirty && tab.doc.path.is_some() {
                if let Some(ts) = tab.last_edit {
                    let elapsed = ts.elapsed();
                    if elapsed >= delay {
                        if tab.doc.save().is_ok() {
                            tab.last_edit = None;
                            if let Some(p) = &tab.doc.path {
                                self.lsp.did_save(tab.doc.language, p);
                            }
                        }
                    } else {
                        let rem = delay - elapsed;
                        soonest_remaining = Some(match soonest_remaining {
                            Some(cur) => cur.min(rem),
                            None => rem,
                        });
                    }
                }
            }
        }

        if let Some(rem) = soonest_remaining {
            ctx.request_repaint_after(rem);
        }
    }

    fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.new_empty_tab();
            self.split_tab = None;
            self.split_right_focused = false;
        } else {
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            if let Some(st) = self.split_tab {
                if st >= self.tabs.len() {
                    self.split_tab = Some(self.tabs.len() - 1);
                }
            }
        }
        self.recompute_search();
    }

    fn close_other_tabs(&mut self, keep_idx: usize) {
        if keep_idx >= self.tabs.len() {
            return;
        }
        let kept = self.tabs.remove(keep_idx);
        self.tabs.clear();
        self.tabs.push(kept);
        self.active_tab = 0;
        self.split_tab = None;
        self.split_right_focused = false;
        self.recompute_search();
    }

    fn close_all_tabs(&mut self) {
        self.tabs.clear();
        self.new_empty_tab();
        self.split_tab = None;
        self.split_right_focused = false;
        self.recompute_search();
    }

    fn sync_settings_to_app(&mut self, ctx: &egui::Context) {
        let desired_kind = if self.settings.theme == "light" {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        };
        if self.theme.kind != desired_kind {
            self.theme = match desired_kind {
                ThemeKind::Dark => Theme::dark(),
                ThemeKind::Light => Theme::light(),
            };
            apply_egui_visuals(ctx, &self.theme);
        }
        self.settings.font_size = self.settings.font_size.clamp(10.0, 28.0);
        for tab in &mut self.tabs {
            tab.view.font_size = self.settings.font_size;
        }
        self.terminal.layout = self.settings.terminal_layout;
        let _ = self.settings.save_default();
    }

    fn spawn_new_terminal_tab(&mut self) {
        let cwd = self.file_tree.as_ref().map(|t| t.root.as_path());
        match self.terminal.spawn_tab(cwd, Arc::clone(&self.repaint)) {
            Ok(_) => {
                self.show_terminal = true;
                self.terminal_focused = true;
            }
            Err(e) => {
                self.status_message = Some(format!("Terminal error: {e}"));
            }
        }
    }

    fn ensure_terminal(&mut self) {
        if self.terminal.is_empty() {
            self.spawn_new_terminal_tab();
        }
    }

    fn handle_terminal_action(&mut self, ctx: &egui::Context, act: TerminalUiAction) {
        match act {
            TerminalUiAction::NewTab => self.spawn_new_terminal_tab(),
            TerminalUiAction::SelectTab(i) => {
                self.terminal.select_tab(i);
                self.terminal_focused = true;
            }
            TerminalUiAction::CloseTab(i) => {
                self.terminal.close_tab(i);
                if self.terminal.is_empty() {
                    self.show_terminal = false;
                    self.terminal_focused = false;
                }
            }
            TerminalUiAction::SetLayout(mode) => {
                self.terminal.layout = mode;
                self.settings.terminal_layout = mode;
                self.sync_settings_to_app(ctx);
            }
            TerminalUiAction::Hide => {
                self.show_terminal = false;
                self.terminal_focused = false;
            }
        }
    }

    fn execute_plugin_command(&mut self, plugin_id: &str, command_id: &str) {
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let input_mode = self
            .plugins
            .plugins
            .iter()
            .find(|p| p.manifest.id == plugin_id)
            .and_then(|p| p.manifest.commands.iter().find(|c| c.id == command_id))
            .map(|c| c.input_mode)
            .unwrap_or(PluginInputMode::None);

        let sel = tab.doc.selections.primary();
        let stdin_text = match input_mode {
            PluginInputMode::None => String::new(),
            PluginInputMode::Selection => {
                if sel.is_empty() {
                    String::new()
                } else {
                    tab.doc.rope.slice(sel.range()).to_string()
                }
            }
            PluginInputMode::Document => tab.doc.rope.to_string(),
            PluginInputMode::SelectionOrDocument => {
                if sel.is_empty() {
                    tab.doc.rope.to_string()
                } else {
                    tab.doc.rope.slice(sel.range()).to_string()
                }
            }
        };

        let (line, col) = tab.doc.char_to_line_col(sel.head);
        let pctx = PluginContext {
            file_path: tab.doc.path.as_deref(),
            workspace_root: self.file_tree.as_ref().map(|t| t.root.as_path()),
            language: tab.doc.language.display_name(),
            line,
            col,
            stdin_text,
        };

        match self.plugins.execute_command(plugin_id, command_id, &pctx) {
            Ok(res) => {
                if !res.success {
                    self.status_message = Some(format!(
                        "Plugin error: {}",
                        res.stderr.lines().next().unwrap_or("failed")
                    ));
                    return;
                }
                match res.output_mode {
                    PluginOutputMode::Status => {
                        let first = res.stdout.lines().next().unwrap_or("Plugin finished");
                        self.status_message = Some(first.to_string());
                    }
                    PluginOutputMode::Insert => {
                        editing::insert_text(&mut tab.doc, &res.stdout);
                        tab.last_edit = Some(Instant::now());
                    }
                    PluginOutputMode::ReplaceSelection => {
                        editing::insert_text(&mut tab.doc, &res.stdout);
                        tab.last_edit = Some(Instant::now());
                    }
                    PluginOutputMode::ReplaceDocument => {
                        let len = tab.doc.len_chars();
                        tab.doc.apply_edits(
                            vec![(0..len, res.stdout)],
                            SelectionSet::single(Selection::caret(0)),
                            EditKind::Other,
                        );
                        tab.last_edit = Some(Instant::now());
                    }
                }
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
    }

    fn build_palette_entries(&self) -> Vec<PaletteEntry> {
        let mut entries = vec![
            PaletteEntry {
                title: if self.settings.auto_save {
                    "Disable Auto-Save".into()
                } else {
                    "Enable Auto-Save".into()
                },
                category: "File".into(),
                command: PaletteCommand::ToggleAutoSave,
            },
            PaletteEntry {
                title: "Open Settings & Plugins".into(),
                category: "Preferences".into(),
                command: PaletteCommand::OpenSettings,
            },
            PaletteEntry {
                title: "New Terminal Tab".into(),
                category: "Terminal".into(),
                command: PaletteCommand::NewTerminalTab,
            },
            PaletteEntry {
                title: "Layout: Bottom".into(),
                category: "Terminal".into(),
                command: PaletteCommand::TerminalBottom,
            },
            PaletteEntry {
                title: "Layout: Right".into(),
                category: "Terminal".into(),
                command: PaletteCommand::TerminalRight,
            },
            PaletteEntry {
                title: "Layout: Floating".into(),
                category: "Terminal".into(),
                command: PaletteCommand::TerminalFloating,
            },
            PaletteEntry {
                title: "Layout: Maximized".into(),
                category: "Terminal".into(),
                command: PaletteCommand::TerminalMaximized,
            },
            PaletteEntry {
                title: "Toggle Dark / Light Theme".into(),
                category: "View".into(),
                command: PaletteCommand::ToggleTheme,
            },
        ];

        for (pid, pname, cmd) in self.plugins.all_enabled_commands() {
            entries.push(PaletteEntry {
                title: cmd.title,
                category: format!("Plugin: {pname}"),
                command: PaletteCommand::PluginCommand {
                    plugin_id: pid,
                    command_id: cmd.id,
                },
            });
        }
        entries
    }

    fn recompute_search(&mut self) {
        if !self.find_bar.open || self.find_bar.query.is_empty() {
            self.find_bar.matches.clear();
            self.find_bar.error = None;
            return;
        }
        let Some(tab) = self.tabs.get(self.active_tab) else {
            return;
        };
        match search::find_all(&tab.doc.rope, &self.find_bar.query, &self.find_bar.options) {
            Ok(m) => {
                self.find_bar.matches = m;
                self.find_bar.error = None;
            }
            Err(e) => {
                self.find_bar.matches.clear();
                self.find_bar.error = Some(e);
            }
        }
    }

    fn jump_to_match(&mut self, next: bool) {
        if self.find_bar.matches.is_empty() {
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return;
        };
        let cur = tab.doc.selections.primary();
        let idx = if next {
            search::find_next(&self.find_bar.matches, cur.end())
        } else {
            search::find_prev(&self.find_bar.matches, cur.start())
        };
        if let Some(i) = idx {
            let r = self.find_bar.matches[i].clone();
            let line = tab.doc.rope.char_to_line(r.start);
            tab.doc
                .selections
                .set_single(Selection::new(r.start, r.end), tab.doc.len_chars());
            tab.view.scroll_to_line = Some(line);
        }
    }

    fn handle_lsp_events(&mut self) {
        for ev in self.lsp.poll_events() {
            match ev {
                LspEvent::Diagnostics { path, items } => {
                    let canon = fs::canonicalize(&path).unwrap_or(path);
                    self.diagnostics.insert(canon, items);
                }
                LspEvent::Completion { items } => {
                    self.completion.show_items(items);
                }
                LspEvent::Hover { text } => {
                    self.status_message = Some(text.lines().next().unwrap_or("").to_string());
                }
                LspEvent::Definition { path, line, col } => {
                    self.open_file_path(path);
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.doc.set_cursor_at_line_col(line, col);
                        tab.view.scroll_to_line = Some(line);
                    }
                }
                LspEvent::Status { status, .. } => {
                    self.status_message = Some(status);
                }
            }
        }
    }

    fn handle_global_action(&mut self, ctx: &egui::Context, action: AppAction) {
        match action {
            AppAction::NewFile => self.new_empty_tab(),
            AppAction::OpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.open_file_path(path);
                }
            }
            AppAction::OpenFolder => {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.open_workspace_folder(folder);
                }
            }
            AppAction::Save => self.save_tab(self.active_tab, false),
            AppAction::SaveAs => self.save_tab(self.active_tab, true),
            AppAction::CloseTab => self.close_tab(self.active_tab),
            AppAction::NextTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    self.recompute_search();
                }
            }
            AppAction::PrevTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = if self.active_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                    self.recompute_search();
                }
            }
            AppAction::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            AppAction::ToggleTerminal => {
                self.show_terminal = !self.show_terminal;
                if self.show_terminal {
                    self.ensure_terminal();
                    self.terminal_focused = true;
                } else {
                    self.terminal_focused = false;
                }
            }
            AppAction::NewTerminalTab => {
                self.spawn_new_terminal_tab();
            }
            AppAction::ToggleTheme => {
                self.settings.theme = if self.settings.theme == "dark" {
                    "light".to_string()
                } else {
                    "dark".to_string()
                };
                self.sync_settings_to_app(ctx);
            }
            AppAction::OpenQuickOpen => {
                self.command_palette.open = false;
                self.quick_open.show_tab(QuickOpenTab::Files);
                self.terminal_focused = false;
            }
            AppAction::OpenRecentFiles => {
                self.command_palette.open = false;
                self.quick_open.show_tab(QuickOpenTab::Recent);
                self.terminal_focused = false;
            }
            AppAction::OpenSymbolPicker => {
                self.command_palette.open = false;
                self.quick_open.show_tab(QuickOpenTab::Symbols);
                self.terminal_focused = false;
            }
            AppAction::OpenCommandPalette => {
                self.quick_open.open = false;
                self.command_palette.show_modal();
                self.terminal_focused = false;
            }
            AppAction::OpenSettings => {
                self.settings_modal.open = !self.settings_modal.open;
                self.terminal_focused = false;
            }
            AppAction::OpenFind => {
                self.find_bar.open = true;
                self.find_bar.show_replace = false;
                self.find_bar.request_focus = true;
                self.terminal_focused = false;
                self.recompute_search();
            }
            AppAction::OpenReplace => {
                self.find_bar.open = true;
                self.find_bar.show_replace = true;
                self.find_bar.request_focus = true;
                self.terminal_focused = false;
                self.recompute_search();
            }
            AppAction::OpenWorkspaceSearch => {
                self.show_sidebar = true;
                self.sidebar_mode = SidebarMode::Search;
                self.workspace_search_ui.request_focus = true;
                self.terminal_focused = false;
            }
            AppAction::ToggleSplitEditor => {
                if self.split_tab.is_some() {
                    self.split_tab = None;
                    self.split_right_focused = false;
                } else if !self.tabs.is_empty() {
                    let right_idx = if self.tabs.len() > 1 {
                        (self.active_tab + 1) % self.tabs.len()
                    } else {
                        self.active_tab
                    };
                    self.split_tab = Some(right_idx);
                    self.split_right_focused = false;
                }
            }
            AppAction::ToggleMarkdownPreview => {
                self.md_mode = self.md_mode.cycle();
                self.status_message = Some(format!("Markdown view: {:?}", self.md_mode));
            }
            AppAction::GoToDefinition => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(path) = &tab.doc.path {
                        let (line, col) =
                            tab.doc.char_to_line_col(tab.doc.selections.primary().head);
                        self.lsp
                            .request_definition(tab.doc.language, path, line, col);
                    }
                }
            }
            AppAction::TriggerCompletion => {
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(path) = &tab.doc.path {
                        let (line, col) =
                            tab.doc.char_to_line_col(tab.doc.selections.primary().head);
                        self.lsp
                            .request_completion(tab.doc.language, path, line, col);
                    }
                }
            }
            AppAction::ZoomIn => {
                self.settings.font_size += 1.0;
                self.sync_settings_to_app(ctx);
                self.status_message = Some(format!("Font size: {:.1}px", self.settings.font_size));
            }
            AppAction::ZoomOut => {
                self.settings.font_size -= 1.0;
                self.sync_settings_to_app(ctx);
                self.status_message = Some(format!("Font size: {:.1}px", self.settings.font_size));
            }
            AppAction::ZoomReset => {
                self.settings.font_size = 14.5;
                self.sync_settings_to_app(ctx);
                self.status_message = Some("Font size reset (14.5px)".to_string());
            }
        }
    }
}

fn apply_egui_visuals(ctx: &egui::Context, theme: &Theme) {
    let mut visuals = match theme.kind {
        ThemeKind::Dark => egui::Visuals::dark(),
        ThemeKind::Light => egui::Visuals::light(),
    };
    visuals.panel_fill = theme.bg_panel;
    visuals.window_fill = theme.bg_popup;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, theme.border);
    visuals.window_corner_radius = egui::CornerRadius::same(6);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, theme.border);
    visuals.widgets.hovered.bg_fill = theme.bg_hover;
    visuals.widgets.active.bg_fill = theme.bg_popup_selected;
    visuals.selection.bg_fill = theme.bg_selection;
    ctx.set_visuals(visuals);
}

impl eframe::App for DracoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_lsp_events();
        self.handle_watch_events();
        self.process_auto_save(ctx);

        if let Some(action) = keymap::poll_global_action(ctx) {
            self.handle_global_action(ctx, action);
        }

        // 1. Bottom Status Bar
        egui::TopBottomPanel::bottom("draco_status")
            .frame(
                egui::Frame::NONE
                    .fill(self.theme.bg_status)
                    .stroke(egui::Stroke::new(1.0_f32, self.theme.border)),
            )
            .show(ctx, |ui| {
                let (line, col, cursor_count, lang, lsp_status, errors, warnings, dirty, has_path) =
                    if let Some(tab) = self.tabs.get(self.active_tab) {
                        let (l, c) = tab.doc.char_to_line_col(tab.doc.selections.primary().head);
                        let diags = tab
                            .doc
                            .path
                            .as_ref()
                            .and_then(|p| self.diagnostics.get(p));
                        let errs = diags
                            .map(|d| d.iter().filter(|x| x.severity == 1).count())
                            .unwrap_or(0);
                        let warns = diags
                            .map(|d| d.iter().filter(|x| x.severity == 2).count())
                            .unwrap_or(0);
                        (
                            l,
                            c,
                            tab.doc.selections.len(),
                            tab.doc.language.display_name(),
                            self.lsp.status_for(tab.doc.language),
                            errs,
                            warns,
                            tab.doc.dirty,
                            tab.doc.path.is_some(),
                        )
                    } else {
                        (0, 0, 1, "Plain Text", None, 0, 0, false, false)
                    };

                let info = StatusInfo {
                    line,
                    col,
                    cursor_count,
                    language: lang,
                    lsp_status,
                    errors,
                    warnings,
                    dirty,
                    has_path,
                    auto_save: self.settings.auto_save,
                    message: self.status_message.as_deref(),
                };
                if let Some(action) = status_bar::show_status_bar(ui, &info, &self.theme) {
                    match action {
                        StatusAction::ToggleTerminal => {
                            self.handle_global_action(ctx, AppAction::ToggleTerminal);
                        }
                        StatusAction::ToggleTheme => {
                            self.handle_global_action(ctx, AppAction::ToggleTheme);
                        }
                        StatusAction::ToggleAutoSave => {
                            self.settings.auto_save = !self.settings.auto_save;
                            self.sync_settings_to_app(ctx);
                            self.status_message = Some(if self.settings.auto_save {
                                "Auto-Save enabled".to_string()
                            } else {
                                "Auto-Save disabled".to_string()
                            });
                        }
                        StatusAction::OpenSettings => {
                            self.handle_global_action(ctx, AppAction::OpenSettings);
                        }
                    }
                }
            });

        // 2. Slim Left Activity Bar (44px)
        let mut activity_action = None;
        let mut toggle_find_from_bar = false;
        let mut toggle_quick_open_from_bar = false;
        egui::SidePanel::left("draco_activity_bar")
            .resizable(false)
            .exact_width(44.0)
            .frame(
                egui::Frame::NONE
                    .fill(self.theme.bg_tab_bar)
                    .stroke(egui::Stroke::new(1.0_f32, self.theme.border))
                    .inner_margin(egui::Margin::symmetric(5, 8)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if icons::activity_button(
                        ui,
                        ActivityIcon::Files,
                        self.show_sidebar,
                        "Explorer (Ctrl+B)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        activity_action = Some(AppAction::ToggleSidebar);
                    }
                    if icons::activity_button(
                        ui,
                        ActivityIcon::Search,
                        self.find_bar.open,
                        "Find & Replace (Ctrl+F)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        toggle_find_from_bar = true;
                    }
                    if icons::activity_button(
                        ui,
                        ActivityIcon::QuickOpen,
                        self.quick_open.open,
                        "Quick Open (Ctrl+P)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        toggle_quick_open_from_bar = true;
                    }
                    if icons::activity_button(
                        ui,
                        ActivityIcon::Terminal,
                        self.show_terminal,
                        "Terminal (Ctrl+`)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        activity_action = Some(AppAction::ToggleTerminal);
                    }
                });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    if icons::activity_button(
                        ui,
                        ActivityIcon::Settings,
                        self.settings_modal.open,
                        "Settings & Plugins (Ctrl+,)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        activity_action = Some(AppAction::OpenSettings);
                    }
                    if icons::activity_button(
                        ui,
                        ActivityIcon::Theme,
                        false,
                        "Toggle Dark / Light Theme (Ctrl+Shift+T)",
                        &self.theme,
                    )
                    .clicked()
                    {
                        activity_action = Some(AppAction::ToggleTheme);
                    }
                });
            });
        if toggle_find_from_bar {
            if self.find_bar.open {
                self.find_bar.open = false;
                self.find_bar.matches.clear();
            } else {
                self.handle_global_action(ctx, AppAction::OpenFind);
            }
        }
        if toggle_quick_open_from_bar {
            if self.quick_open.open {
                self.quick_open.open = false;
            } else {
                self.handle_global_action(ctx, AppAction::OpenQuickOpen);
            }
        }
        if let Some(act) = activity_action {
            self.handle_global_action(ctx, act);
        }

        // 3. Left File Explorer Sidebar
        if self.show_sidebar {
            let active_path = self
                .tabs
                .get(self.active_tab)
                .and_then(|t| t.doc.path.clone());
            let dirty_paths: Vec<PathBuf> = self
                .tabs
                .iter()
                .filter(|t| t.doc.dirty)
                .filter_map(|t| t.doc.path.clone())
                .collect();

            let mut explorer_action = None;
            let mut ws_search_action = None;
            let has_workspace = self.file_tree.is_some();
            egui::SidePanel::left("draco_explorer")
                .resizable(true)
                .default_width(250.0)
                .min_width(180.0)
                .frame(
                    egui::Frame::NONE
                        .fill(self.theme.bg_panel)
                        .stroke(egui::Stroke::new(1.0_f32, self.theme.border))
                        .inner_margin(egui::Margin::symmetric(8, 8)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let exp_active = self.sidebar_mode == SidebarMode::Explorer;
                        if ui
                            .selectable_label(
                                exp_active,
                                egui::RichText::new("Explorer").size(11.5).color(if exp_active {
                                    self.theme.accent
                                } else {
                                    self.theme.fg_muted
                                }),
                            )
                            .clicked()
                        {
                            self.sidebar_mode = SidebarMode::Explorer;
                        }
                        let search_active = self.sidebar_mode == SidebarMode::Search;
                        if ui
                            .selectable_label(
                                search_active,
                                egui::RichText::new("Search").size(11.5).color(if search_active {
                                    self.theme.accent
                                } else {
                                    self.theme.fg_muted
                                }),
                            )
                            .on_hover_text("Search in Workspace (Ctrl+Shift+F)")
                            .clicked()
                        {
                            self.sidebar_mode = SidebarMode::Search;
                            self.workspace_search_ui.request_focus = true;
                        }
                    });
                    ui.separator();

                    match self.sidebar_mode {
                        SidebarMode::Explorer => {
                            explorer_action = explorer::show_explorer(
                                ui,
                                &mut self.file_tree,
                                &mut self.explorer_ui,
                                active_path.as_ref(),
                                &dirty_paths,
                                &self.theme,
                            );
                        }
                        SidebarMode::Search => {
                            let snap = self.workspace_searcher.snapshot();
                            ws_search_action = workspace_search::show_workspace_search(
                                ui,
                                &mut self.workspace_search_ui,
                                &snap,
                                has_workspace,
                                &self.theme,
                            );
                        }
                    }
                });

            if let Some(ws_act) = ws_search_action {
                match ws_act {
                    WorkspaceSearchAction::RunSearch => {
                        if let Some(root) = self.file_tree.as_ref().map(|t| t.root.clone()) {
                            self.workspace_searcher.start_search(
                                root,
                                self.workspace_search_ui.query.clone(),
                                self.workspace_search_ui.options.clone(),
                                Arc::clone(&self.repaint),
                            );
                        }
                    }
                    WorkspaceSearchAction::ReplaceAll => {
                        if let Some(root) = self.file_tree.as_ref().map(|t| t.root.clone()) {
                            match replace_in_workspace(
                                &root,
                                &self.workspace_search_ui.query,
                                &self.workspace_search_ui.replacement,
                                &self.workspace_search_ui.options,
                            ) {
                                Ok(sum) => {
                                    for p in &sum.modified_paths {
                                        let canon = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                                        for tab in &mut self.tabs {
                                            if tab.doc.path.as_deref() == Some(canon.as_path())
                                                && !tab.doc.dirty
                                            {
                                                let _ = tab.doc.reload_from_disk();
                                                tab.syntax = SyntaxState::new(tab.doc.language);
                                            }
                                        }
                                    }
                                    self.status_message = Some(format!(
                                        "Replaced {} occurrences across {} files",
                                        sum.replacements, sum.files_changed
                                    ));
                                    self.workspace_searcher.start_search(
                                        root,
                                        self.workspace_search_ui.query.clone(),
                                        self.workspace_search_ui.options.clone(),
                                        Arc::clone(&self.repaint),
                                    );
                                }
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Workspace replace error: {e}"));
                                }
                            }
                        }
                    }
                    WorkspaceSearchAction::OpenMatch { path, line, col } => {
                        self.open_file_path(path);
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            tab.doc.set_cursor_at_line_col(line, col);
                            tab.view.scroll_to_line = Some(line);
                        }
                    }
                }
            }

            if let Some(act) = explorer_action {
                match act {
                    ExplorerAction::OpenFile(p) => self.open_file_path(p),
                    ExplorerAction::OpenFolderDialog => {
                        self.handle_global_action(ctx, AppAction::OpenFolder);
                    }
                    ExplorerAction::Refresh => self.refresh_workspace(),
                    ExplorerAction::CreateFile(p) => {
                        if let Some(parent) = p.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        match fs::File::create(&p) {
                            Ok(_) => {
                                self.refresh_workspace();
                                self.open_file_path(p);
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Create file failed: {e}"));
                            }
                        }
                    }
                    ExplorerAction::CreateFolder(p) => match fs::create_dir_all(&p) {
                        Ok(()) => {
                            self.refresh_workspace();
                            self.status_message = Some(format!("Created folder {}", p.display()));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Create folder failed: {e}"));
                        }
                    },
                    ExplorerAction::Rename { from, to } => match fs::rename(&from, &to) {
                        Ok(()) => {
                            for tab in &mut self.tabs {
                                if tab.doc.path.as_deref() == Some(from.as_path()) {
                                    tab.doc.path = Some(to.clone());
                                }
                            }
                            self.refresh_workspace();
                            self.status_message = Some(format!("Renamed to {}", to.display()));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Rename failed: {e}"));
                        }
                    },
                    ExplorerAction::Delete(p) => {
                        let res = if p.is_dir() {
                            fs::remove_dir_all(&p)
                        } else {
                            fs::remove_file(&p)
                        };
                        match res {
                            Ok(()) => {
                                self.refresh_workspace();
                                self.status_message = Some(format!("Deleted {}", p.display()));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Delete failed: {e}"));
                            }
                        }
                    }
                    ExplorerAction::CopyPath(s) => self.copy_string(ctx, s),
                }
            }
        }

        // 4. Docked Terminal (Bottom or Right)
        let mut pending_term_action = None;
        if self.show_terminal && self.terminal.layout == TerminalLayout::Bottom {
            egui::TopBottomPanel::bottom("draco_terminal_bottom")
                .resizable(true)
                .default_height(220.0)
                .min_height(110.0)
                .frame(
                    egui::Frame::NONE
                        .fill(self.theme.bg_panel)
                        .stroke(egui::Stroke::new(1.0_f32, self.theme.border))
                        .inner_margin(egui::Margin::symmetric(8, 6)),
                )
                .show(ctx, |ui| {
                    pending_term_action = terminal_panel::show_terminal_panel(
                        ui,
                        &mut self.terminal,
                        &mut self.terminal_focused,
                        &self.theme,
                    );
                });
        } else if self.show_terminal && self.terminal.layout == TerminalLayout::Right {
            egui::SidePanel::right("draco_terminal_right")
                .resizable(true)
                .default_width(420.0)
                .min_width(240.0)
                .frame(
                    egui::Frame::NONE
                        .fill(self.theme.bg_panel)
                        .stroke(egui::Stroke::new(1.0_f32, self.theme.border))
                        .inner_margin(egui::Margin::symmetric(8, 6)),
                )
                .show(ctx, |ui| {
                    pending_term_action = terminal_panel::show_terminal_panel(
                        ui,
                        &mut self.terminal,
                        &mut self.terminal_focused,
                        &self.theme,
                    );
                });
        }

        // 5. Top Tab Bar
        egui::TopBottomPanel::top("draco_tabs")
            .frame(
                egui::Frame::NONE
                    .fill(self.theme.bg_tab_bar)
                    .stroke(egui::Stroke::new(1.0_f32, self.theme.border)),
            )
            .show(ctx, |ui| {
                let tab_infos: Vec<TabInfo> = self
                    .tabs
                    .iter()
                    .map(|t| TabInfo {
                        title: t.doc.title(),
                        path: t.doc.path.clone(),
                        dirty: t.doc.dirty,
                    })
                    .collect();
                if let Some(action) =
                    tabs::show_tabs(ui, &tab_infos, self.active_tab, &self.theme)
                {
                    match action {
                        TabAction::Select(i) => {
                            self.active_tab = i;
                            self.terminal_focused = false;
                            self.recompute_search();
                        }
                        TabAction::Close(i) => self.close_tab(i),
                        TabAction::CloseOthers(i) => self.close_other_tabs(i),
                        TabAction::CloseAll => self.close_all_tabs(),
                        TabAction::Save(i) => self.save_tab(i, false),
                        TabAction::CopyPath(s) => self.copy_string(ctx, s),
                        TabAction::NewTab => self.new_empty_tab(),
                    }
                }
            });

        // 6. Central Panel (Maximized Terminal OR Code Editor)
        let mut cursor_anchor = egui::Pos2::ZERO;
        let editor_has_focus = !self.quick_open.open
            && !self.command_palette.open
            && !self.settings_modal.open
            && !self.terminal_focused
            && !self.completion.open
            && !ctx.wants_keyboard_input();

        let active_doc_path = self
            .tabs
            .get(self.active_tab)
            .and_then(|t| t.doc.path.clone());
        let active_doc_dirty = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.doc.dirty)
            .unwrap_or(false);
        let root_path_owned = self.file_tree.as_ref().map(|t| t.root.clone());

        let bottom_resize_pad =
            if self.show_terminal && self.terminal.layout == TerminalLayout::Bottom {
                6
            } else {
                0
            };
        let right_resize_pad =
            if self.show_terminal && self.terminal.layout == TerminalLayout::Right {
                6
            } else {
                0
            };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(self.theme.bg_editor)
                    .inner_margin(egui::Margin {
                        left: 0,
                        right: right_resize_pad,
                        top: 0,
                        bottom: bottom_resize_pad,
                    }),
            )
            .show(ctx, |ui| {
                if self.show_terminal && self.terminal.layout == TerminalLayout::Maximized {
                    egui::Frame::NONE
                        .fill(self.theme.bg_panel)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            pending_term_action = terminal_panel::show_terminal_panel(
                                ui,
                                &mut self.terminal,
                                &mut self.terminal_focused,
                                &self.theme,
                            );
                        });
                    return;
                }

                let is_markdown = self
                    .tabs
                    .get(self.active_tab)
                    .map(|t| t.doc.language == LanguageId::Markdown)
                    .unwrap_or(false);

                if is_markdown {
                    egui::Frame::NONE
                        .fill(self.theme.bg_editor)
                        .inner_margin(egui::Margin::symmetric(12, 3))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let label = active_doc_path
                                    .as_deref()
                                    .and_then(|p| {
                                        root_path_owned
                                            .as_deref()
                                            .and_then(|r| p.strip_prefix(r).ok())
                                            .or(Some(p))
                                    })
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| "Untitled".to_string());
                                ui.label(
                                    egui::RichText::new(label)
                                        .size(12.0)
                                        .color(self.theme.fg_muted),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        markdown_preview::show_markdown_mode_switcher(
                                            ui,
                                            &mut self.md_mode,
                                            &mut self.md_cache.show_toc,
                                            &mut self.md_cache.scroll_sync,
                                            &self.theme,
                                        );
                                    },
                                );
                            });
                        });
                } else {
                    tabs::show_breadcrumbs(
                        ui,
                        active_doc_path.as_deref(),
                        root_path_owned.as_deref(),
                        active_doc_dirty,
                        &self.theme,
                    );
                }

                if let Some(find_act) =
                    find_bar::show_find_bar(ui, &mut self.find_bar, &self.theme)
                {
                    match find_act {
                        FindAction::QueryChanged => self.recompute_search(),
                        FindAction::Next => self.jump_to_match(true),
                        FindAction::Prev => self.jump_to_match(false),
                        FindAction::ReplaceOne => {
                            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                                let sel = tab.doc.selections.primary();
                                if !sel.is_empty() {
                                    let r = sel.range();
                                    let repl = self.find_bar.replacement.clone();
                                    let new_pos = r.start + repl.chars().count();
                                    tab.doc.apply_edits(
                                        vec![(r, repl)],
                                        SelectionSet::single(Selection::caret(new_pos)),
                                        EditKind::Other,
                                    );
                                    tab.last_edit = Some(Instant::now());
                                    self.recompute_search();
                                    self.jump_to_match(true);
                                }
                            }
                        }
                        FindAction::ReplaceAll => {
                            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                                if let Ok(count) = search::replace_all(
                                    &mut tab.doc,
                                    &self.find_bar.query,
                                    &self.find_bar.replacement,
                                    &self.find_bar.options,
                                ) {
                                    tab.last_edit = Some(Instant::now());
                                    self.status_message =
                                        Some(format!("Replaced {count} occurrences"));
                                    self.recompute_search();
                                }
                            }
                        }
                        FindAction::Close => {
                            self.find_bar.matches.clear();
                        }
                    }
                }

                let mut need_recompute_search = false;
                let mut pending_ctx_action = None;
                let mut pending_preview_action = None;
                let split_active_idx = if !is_markdown || self.md_mode == MarkdownViewMode::Editor {
                    self.split_tab.filter(|&idx| idx < self.tabs.len())
                } else {
                    None
                };

                if let Some(right_idx) = split_active_idx {
                    let left_idx = self.active_tab.min(self.tabs.len().saturating_sub(1));
                    let avail = ui.available_size();
                    let half_w = (avail.x * 0.5 - 3.0).max(180.0);
                    let mut close_split = false;
                    let mut next_right_idx = right_idx;
                    let tab_count = self.tabs.len();
                    let right_title = self
                        .tabs
                        .get(right_idx)
                        .map(|t| t.doc.title())
                        .unwrap_or_else(|| "Untitled".to_string());
                    self.split_view.font_size = self.settings.font_size;

                    ui.horizontal(|ui| {
                        // Left Split Pane
                        ui.allocate_ui_with_layout(
                            egui::vec2(half_w, avail.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                let pane_rect = ui.available_rect_before_wrap();
                                if ui.input(|i| i.pointer.primary_clicked())
                                    && ui.rect_contains_pointer(pane_rect)
                                {
                                    self.split_right_focused = false;
                                    self.terminal_focused = false;
                                    self.completion.close();
                                }
                                if !self.split_right_focused {
                                    ui.painter().hline(
                                        pane_rect.x_range(),
                                        pane_rect.min.y + 1.0,
                                        egui::Stroke::new(2.0_f32, self.theme.accent),
                                    );
                                }
                                if let Some(tab) = self.tabs.get_mut(left_idx) {
                                    let diags: Vec<DiagnosticItem> = tab
                                        .doc
                                        .path
                                        .as_ref()
                                        .and_then(|p| self.diagnostics.get(p))
                                        .cloned()
                                        .unwrap_or_default();
                                    let out = code_view::show_code_view(
                                        ui,
                                        &mut tab.doc,
                                        &mut tab.syntax,
                                        &mut tab.view,
                                        &self.theme,
                                        &self.find_bar.matches,
                                        &diags,
                                        editor_has_focus && !self.split_right_focused,
                                        &mut self.clipboard,
                                    );
                                    if !self.split_right_focused {
                                        cursor_anchor = out.primary_cursor_screen_pos;
                                        pending_ctx_action = out.context_action;
                                    }
                                    if out.text_changed {
                                        tab.last_edit = Some(Instant::now());
                                        if let Some(path) = tab.doc.path.clone() {
                                            let text = tab.doc.rope.to_string();
                                            self.lsp.did_change(
                                                tab.doc.language,
                                                &path,
                                                tab.doc.version,
                                                &text,
                                            );
                                        }
                                        if self.find_bar.open {
                                            need_recompute_search = true;
                                        }
                                    }
                                }
                            },
                        );

                        ui.separator();

                        // Right Split Pane
                        ui.allocate_ui_with_layout(
                            egui::vec2(half_w, avail.y),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                let pane_rect = ui.available_rect_before_wrap();
                                if ui.input(|i| i.pointer.primary_clicked())
                                    && ui.rect_contains_pointer(pane_rect)
                                {
                                    self.split_right_focused = true;
                                    self.terminal_focused = false;
                                    self.completion.close();
                                }
                                if self.split_right_focused {
                                    ui.painter().hline(
                                        pane_rect.x_range(),
                                        pane_rect.min.y + 1.0,
                                        egui::Stroke::new(2.0_f32, self.theme.accent),
                                    );
                                }
                                egui::Frame::NONE
                                    .fill(self.theme.bg_tab_bar)
                                    .inner_margin(egui::Margin::symmetric(8, 2))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&right_title)
                                                    .size(11.5)
                                                    .strong()
                                                    .color(if self.split_right_focused {
                                                        self.theme.accent
                                                    } else {
                                                        self.theme.fg_muted
                                                    }),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add(
                                                            egui::Button::new(
                                                                egui::RichText::new("×")
                                                                    .size(13.0)
                                                                    .color(self.theme.fg_muted),
                                                            )
                                                            .frame(false),
                                                        )
                                                        .on_hover_text("Close Split (Ctrl+\\)")
                                                        .clicked()
                                                    {
                                                        close_split = true;
                                                    }
                                                    if tab_count > 1 {
                                                        if ui.small_button("›").clicked() {
                                                            next_right_idx =
                                                                (right_idx + 1) % tab_count;
                                                        }
                                                        if ui.small_button("‹").clicked() {
                                                            next_right_idx = if right_idx == 0 {
                                                                tab_count - 1
                                                            } else {
                                                                right_idx - 1
                                                            };
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                    });

                                if let Some(tab) = self.tabs.get_mut(right_idx) {
                                    let diags: Vec<DiagnosticItem> = tab
                                        .doc
                                        .path
                                        .as_ref()
                                        .and_then(|p| self.diagnostics.get(p))
                                        .cloned()
                                        .unwrap_or_default();
                                    let out = code_view::show_code_view(
                                        ui,
                                        &mut tab.doc,
                                        &mut tab.syntax,
                                        &mut self.split_view,
                                        &self.theme,
                                        &[],
                                        &diags,
                                        editor_has_focus && self.split_right_focused,
                                        &mut self.clipboard,
                                    );
                                    if self.split_right_focused {
                                        cursor_anchor = out.primary_cursor_screen_pos;
                                        pending_ctx_action = out.context_action;
                                    }
                                    if out.text_changed {
                                        tab.last_edit = Some(Instant::now());
                                        if let Some(path) = tab.doc.path.clone() {
                                            let text = tab.doc.rope.to_string();
                                            self.lsp.did_change(
                                                tab.doc.language,
                                                &path,
                                                tab.doc.version,
                                                &text,
                                            );
                                        }
                                    }
                                }
                            },
                        );
                    });

                    if close_split {
                        self.split_tab = None;
                        self.split_right_focused = false;
                    } else if next_right_idx != right_idx {
                        self.split_tab = Some(next_right_idx);
                    }
                } else if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let diags: Vec<DiagnosticItem> = tab
                        .doc
                        .path
                        .as_ref()
                        .and_then(|p| self.diagnostics.get(p))
                        .cloned()
                        .unwrap_or_default();

                    if is_markdown && self.md_mode == MarkdownViewMode::Preview {
                        pending_preview_action = markdown_preview::show_markdown_preview(
                            ui,
                            &tab.doc,
                            root_path_owned.as_deref(),
                            &mut self.md_cache,
                            &self.theme,
                        );
                    } else if is_markdown && self.md_mode == MarkdownViewMode::Split {
                        let avail = ui.available_size();
                        let half_w = (avail.x * 0.5 - 2.0).max(200.0);
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(half_w, avail.y),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    let editor_hovered =
                                        ui.rect_contains_pointer(ui.available_rect_before_wrap());
                                    let out = code_view::show_code_view(
                                        ui,
                                        &mut tab.doc,
                                        &mut tab.syntax,
                                        &mut tab.view,
                                        &self.theme,
                                        &self.find_bar.matches,
                                        &diags,
                                        editor_has_focus,
                                        &mut self.clipboard,
                                    );
                                    cursor_anchor = out.primary_cursor_screen_pos;
                                    pending_ctx_action = out.context_action;
                                    if editor_hovered {
                                        self.md_cache.notify_editor_scroll(out.top_visible_line);
                                    }
                                    if out.text_changed {
                                        tab.last_edit = Some(Instant::now());
                                        if self.find_bar.open {
                                            need_recompute_search = true;
                                        }
                                    }
                                },
                            );
                            ui.separator();
                            ui.allocate_ui_with_layout(
                                egui::vec2(half_w, avail.y),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    pending_preview_action =
                                        markdown_preview::show_markdown_preview(
                                            ui,
                                            &tab.doc,
                                            root_path_owned.as_deref(),
                                            &mut self.md_cache,
                                            &self.theme,
                                        );
                                },
                            );
                        });
                    } else {
                        let out = code_view::show_code_view(
                            ui,
                            &mut tab.doc,
                            &mut tab.syntax,
                            &mut tab.view,
                            &self.theme,
                            &self.find_bar.matches,
                            &diags,
                            editor_has_focus,
                            &mut self.clipboard,
                        );
                        cursor_anchor = out.primary_cursor_screen_pos;
                        pending_ctx_action = out.context_action;

                        if ui.input(|i| i.pointer.primary_clicked())
                            && ui.rect_contains_pointer(ui.max_rect())
                        {
                            self.terminal_focused = false;
                            self.completion.close();
                        }

                        if out.text_changed {
                            tab.last_edit = Some(Instant::now());
                            if let Some(path) = tab.doc.path.clone() {
                                let text = tab.doc.rope.to_string();
                                self.lsp
                                    .did_change(tab.doc.language, &path, tab.doc.version, &text);
                                if out.trigger_completion_char {
                                    let (l, c) =
                                        tab.doc.char_to_line_col(tab.doc.selections.primary().head);
                                    self.lsp.request_completion(tab.doc.language, &path, l, c);
                                }
                            }
                            if self.find_bar.open {
                                need_recompute_search = true;
                            }
                        }

                        if let Some((line, col)) = out.request_definition {
                            if let Some(path) = &tab.doc.path {
                                self.lsp
                                    .request_definition(tab.doc.language, path, line, col);
                            }
                        }
                    }
                }
                if need_recompute_search {
                    self.recompute_search();
                }
                if let Some(p_act) = pending_preview_action {
                    match p_act {
                        PreviewAction::OpenFile { path, anchor } => {
                            self.open_file_path(path);
                            if let Some(a) = anchor {
                                self.md_cache.highlighted_heading =
                                    Some((a.clone(), Instant::now()));
                                self.md_cache.scroll_to_anchor = Some(a);
                            }
                        }
                        PreviewAction::OpenExternalUrl(url) => {
                            ctx.open_url(egui::OpenUrl::new_tab(url));
                        }
                        PreviewAction::ScrollEditorToLine(line) => {
                            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                                tab.view.scroll_to_top_line = Some(line);
                            }
                        }
                    }
                }
                if let Some(ctx_act) = pending_ctx_action {
                    match ctx_act {
                        CodeViewContextAction::OpenFind => {
                            self.handle_global_action(ctx, AppAction::OpenFind);
                        }
                        CodeViewContextAction::OpenReplace => {
                            self.handle_global_action(ctx, AppAction::OpenReplace);
                        }
                        CodeViewContextAction::TriggerCompletion => {
                            self.handle_global_action(ctx, AppAction::TriggerCompletion);
                        }
                    }
                }
            });

        // 7. Floating Terminal Window
        if self.show_terminal && self.terminal.layout == TerminalLayout::Floating {
            egui::Window::new("Draco Terminal")
                .resizable(true)
                .default_size([720.0, 340.0])
                .frame(
                    egui::Frame::NONE
                        .fill(self.theme.bg_panel)
                        .stroke(egui::Stroke::new(1.0_f32, self.theme.border))
                        .corner_radius(6.0)
                        .inner_margin(8.0),
                )
                .show(ctx, |ui| {
                    pending_term_action = terminal_panel::show_terminal_panel(
                        ui,
                        &mut self.terminal,
                        &mut self.terminal_focused,
                        &self.theme,
                    );
                });
        }
        if let Some(t_act) = pending_term_action {
            self.handle_terminal_action(ctx, t_act);
        }

        // 8. Floating Completion Popup
        if let Some(inserted) =
            completion::show_completion_popup(ctx, &mut self.completion, cursor_anchor, &self.theme)
        {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                editing::insert_text(&mut tab.doc, &inserted);
                tab.last_edit = Some(Instant::now());
            }
        }

        // 9. Unified Quick Open Modal (Ctrl+P Files | Ctrl+E Recent | Ctrl+R Symbols)
        let root_path: Option<&Path> = self.file_tree.as_ref().map(|t| t.root.as_path());
        let active_symbols =
            if self.quick_open.open && self.quick_open.tab == QuickOpenTab::Symbols {
                self.tabs
                    .get_mut(self.active_tab)
                    .map(|tab| {
                        let deltas = tab.doc.drain_deltas();
                        tab.syntax
                            .sync(&tab.doc.rope, tab.doc.language, tab.doc.version, &deltas);
                        tab.syntax.extract_symbols(&tab.doc.rope)
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        if let Some(action) = quick_open::show_quick_open_unified(
            ctx,
            &mut self.quick_open,
            &self.indexer,
            root_path,
            &self.recent_files,
            &active_symbols,
            &self.theme,
        ) {
            match action {
                QuickOpenAction::OpenFile(chosen) => self.open_file_path(chosen),
                QuickOpenAction::JumpToLine(line) => {
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.doc.set_cursor_at_line_col(line, 0);
                        tab.view.scroll_to_line = Some(line);
                        self.terminal_focused = false;
                    }
                }
            }
        }

        // 10. Command Palette (Ctrl+Shift+P)
        if self.command_palette.open {
            let entries = self.build_palette_entries();
            if let Some(cmd) = command_palette::show_command_palette(
                ctx,
                &mut self.command_palette,
                &entries,
                &self.theme,
            ) {
                match cmd {
                    PaletteCommand::ToggleAutoSave => {
                        self.settings.auto_save = !self.settings.auto_save;
                        self.sync_settings_to_app(ctx);
                    }
                    PaletteCommand::OpenSettings => {
                        self.settings_modal.open = true;
                    }
                    PaletteCommand::NewTerminalTab => self.spawn_new_terminal_tab(),
                    PaletteCommand::TerminalBottom => {
                        self.show_terminal = true;
                        self.handle_terminal_action(
                            ctx,
                            TerminalUiAction::SetLayout(TerminalLayout::Bottom),
                        );
                    }
                    PaletteCommand::TerminalRight => {
                        self.show_terminal = true;
                        self.handle_terminal_action(
                            ctx,
                            TerminalUiAction::SetLayout(TerminalLayout::Right),
                        );
                    }
                    PaletteCommand::TerminalFloating => {
                        self.show_terminal = true;
                        self.handle_terminal_action(
                            ctx,
                            TerminalUiAction::SetLayout(TerminalLayout::Floating),
                        );
                    }
                    PaletteCommand::TerminalMaximized => {
                        self.show_terminal = true;
                        self.handle_terminal_action(
                            ctx,
                            TerminalUiAction::SetLayout(TerminalLayout::Maximized),
                        );
                    }
                    PaletteCommand::ToggleTheme => {
                        self.handle_global_action(ctx, AppAction::ToggleTheme);
                    }
                    PaletteCommand::PluginCommand {
                        plugin_id,
                        command_id,
                    } => {
                        self.execute_plugin_command(&plugin_id, &command_id);
                    }
                }
            }
        }

        // 11. Settings & Plugins Modal (Ctrl+,)
        if let Some(s_act) = settings_modal::show_settings_modal(
            ctx,
            &mut self.settings_modal,
            &mut self.settings,
            &self.plugins,
            &self.theme,
        ) {
            match s_act {
                SettingsModalAction::SettingsChanged => self.sync_settings_to_app(ctx),
                SettingsModalAction::InstallPluginDialog => {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        match self.plugins.install_from_dir(&dir, &self.settings) {
                            Ok(m) => {
                                self.status_message =
                                    Some(format!("Installed plugin '{}' v{}", m.name, m.version));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Plugin install error: {e}"));
                            }
                        }
                    }
                }
                SettingsModalAction::ReloadPlugins => {
                    let root = self.file_tree.as_ref().map(|t| t.root.as_path());
                    self.plugins.reload(root, &self.settings);
                    self.status_message =
                        Some(format!("Reloaded {} plugin(s)", self.plugins.plugins.len()));
                }
                SettingsModalAction::TogglePlugin { id, enabled } => {
                    self.settings.set_plugin_enabled(&id, enabled);
                    let root = self.file_tree.as_ref().map(|t| t.root.as_path());
                    self.plugins.reload(root, &self.settings);
                    let _ = self.settings.save_default();
                }
                SettingsModalAction::UninstallPlugin(id) => {
                    match self.plugins.uninstall(&id, &self.settings) {
                        Ok(()) => {
                            self.status_message = Some(format!("Uninstalled plugin '{id}'"));
                        }
                        Err(e) => {
                            self.status_message = Some(e);
                        }
                    }
                }
                SettingsModalAction::RunPluginCommand {
                    plugin_id,
                    command_id,
                } => {
                    self.execute_plugin_command(&plugin_id, &command_id);
                }
            }
        }
    }
}
