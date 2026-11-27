use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, Key, RichText, Sense, Stroke, Vec2};

use crate::syntax::symbols::DocumentSymbol;
use crate::theme::Theme;
use crate::ui::icons;
use crate::workspace::fuzzy::{fuzzy_score, WorkspaceIndexer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuickOpenTab {
    #[default]
    Files,
    Recent,
    Symbols,
}

impl QuickOpenTab {
    pub fn next(self) -> Self {
        match self {
            Self::Files => Self::Recent,
            Self::Recent => Self::Symbols,
            Self::Symbols => Self::Files,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickOpenAction {
    OpenFile(PathBuf),
    JumpToLine(usize),
}

#[derive(Default)]
pub struct QuickOpenState {
    pub open: bool,
    pub tab: QuickOpenTab,
    pub query: String,
    pub selected: usize,
    pub request_focus: bool,
    pub opened_this_frame: bool,
}

impl QuickOpenState {
    pub fn show_modal(&mut self) {
        self.show_tab(QuickOpenTab::Files);
    }

    pub fn show_tab(&mut self, tab: QuickOpenTab) {
        self.open = true;
        self.tab = tab;
        self.query.clear();
        self.selected = 0;
        self.request_focus = true;
        self.opened_this_frame = true;
    }
}

/// Backward-compatible wrapper returning only chosen file path (Files tab).
pub fn show_quick_open(
    ctx: &egui::Context,
    state: &mut QuickOpenState,
    indexer: &WorkspaceIndexer,
    root: Option<&Path>,
    theme: &Theme,
) -> Option<PathBuf> {
    match show_quick_open_unified(ctx, state, indexer, root, &[], &[], theme) {
        Some(QuickOpenAction::OpenFile(p)) => Some(p),
        _ => None,
    }
}

/// Unified Search Everywhere modal supporting Files (`Ctrl+P`), Recent Files (`Ctrl+E`),
/// and Document Symbols (`Ctrl+R`) with `Tab` switching.
pub fn show_quick_open_unified(
    ctx: &egui::Context,
    state: &mut QuickOpenState,
    indexer: &WorkspaceIndexer,
    root: Option<&Path>,
    recent_files: &[PathBuf],
    symbols: &[DocumentSymbol],
    theme: &Theme,
) -> Option<QuickOpenAction> {
    if !state.open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Tab)) {
        state.tab = state.tab.next();
        state.selected = 0;
        state.request_focus = true;
    }

    // Prepare filtered list for active tab
    let file_results: Vec<(PathBuf, String)> = if state.tab == QuickOpenTab::Files {
        root.map(|r| {
            indexer
                .query(r, &state.query, 25)
                .into_iter()
                .map(|(p, rel, _)| (p, rel))
                .collect()
        })
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let recent_results: Vec<(PathBuf, String)> = if state.tab == QuickOpenTab::Recent {
        let mut items: Vec<(PathBuf, String, i32)> = recent_files
            .iter()
            .filter_map(|p| {
                let rel = root
                    .and_then(|r| p.strip_prefix(r).ok())
                    .unwrap_or(p.as_path())
                    .to_string_lossy()
                    .to_string();
                if state.query.is_empty() {
                    Some((p.clone(), rel, 0))
                } else {
                    fuzzy_score(&state.query, &rel).map(|s| (p.clone(), rel, s))
                }
            })
            .collect();
        if !state.query.is_empty() {
            items.sort_by(|a, b| b.2.cmp(&a.2));
        }
        items.into_iter().take(25).map(|(p, r, _)| (p, r)).collect()
    } else {
        Vec::new()
    };

    let symbol_results: Vec<&DocumentSymbol> = if state.tab == QuickOpenTab::Symbols {
        let mut items: Vec<(&DocumentSymbol, i32)> = symbols
            .iter()
            .filter_map(|s| {
                if state.query.is_empty() {
                    Some((s, 0))
                } else {
                    fuzzy_score(&state.query, &s.name).map(|sc| (s, sc))
                }
            })
            .collect();
        if !state.query.is_empty() {
            items.sort_by(|a, b| b.1.cmp(&a.1));
        }
        items.into_iter().take(40).map(|(s, _)| s).collect()
    } else {
        Vec::new()
    };

    let item_count = match state.tab {
        QuickOpenTab::Files => file_results.len(),
        QuickOpenTab::Recent => recent_results.len(),
        QuickOpenTab::Symbols => symbol_results.len(),
    };

    if item_count > 0 {
        state.selected = state.selected.min(item_count - 1);
    } else {
        state.selected = 0;
    }

    if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && item_count > 0 {
        state.selected = (state.selected + 1) % item_count;
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && item_count > 0 {
        state.selected = if state.selected == 0 {
            item_count - 1
        } else {
            state.selected - 1
        };
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) && item_count > 0 {
        state.open = false;
        return match state.tab {
            QuickOpenTab::Files => file_results
                .get(state.selected)
                .map(|(p, _)| QuickOpenAction::OpenFile(p.clone())),
            QuickOpenTab::Recent => recent_results
                .get(state.selected)
                .map(|(p, _)| QuickOpenAction::OpenFile(p.clone())),
            QuickOpenTab::Symbols => symbol_results
                .get(state.selected)
                .map(|s| QuickOpenAction::JumpToLine(s.start_line)),
        };
    }

    let mut chosen: Option<QuickOpenAction> = None;
    let mut close_clicked = false;
    let inner_resp = egui::Window::new("Quick Open")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 56.0))
        .default_width(560.0)
        .max_width(560.0)
        .frame(
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(8.0)
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 20,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(90),
                })
                .inner_margin(10.0),
        )
        .show(ctx, |ui| {
            // JetBrains-style Search Everywhere tab strip
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (tab, label) in [
                    (QuickOpenTab::Files, "Files (Ctrl+P)"),
                    (QuickOpenTab::Recent, "Recent (Ctrl+E)"),
                    (QuickOpenTab::Symbols, "Symbols (Ctrl+R)"),
                ] {
                    let active = state.tab == tab;
                    let text = RichText::new(label)
                        .size(11.5)
                        .color(if active { theme.accent } else { theme.fg_muted });
                    if ui.selectable_label(active, text).clicked() {
                        state.tab = tab;
                        state.selected = 0;
                        state.request_focus = true;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("×").size(15.0).color(theme.fg_muted))
                                .frame(false),
                        )
                        .on_hover_text("Close (Esc)")
                        .clicked()
                    {
                        close_clicked = true;
                    }
                    ui.label(RichText::new("Tab ↹").size(10.5).color(theme.fg_muted));
                });
            });
            ui.add_space(5.0);

            let hint = match state.tab {
                QuickOpenTab::Files => "Search workspace files by name…",
                QuickOpenTab::Recent => "Filter recently opened files…",
                QuickOpenTab::Symbols => "Jump to function, struct, class, or heading…",
            };
            let resp = ui.add_sized(
                Vec2::new(540.0, 28.0),
                egui::TextEdit::singleline(&mut state.query).hint_text(hint),
            );
            if state.request_focus {
                resp.request_focus();
                state.request_focus = false;
            }
            if resp.changed() {
                state.selected = 0;
            }

            ui.add_space(6.0);
            match state.tab {
                QuickOpenTab::Files => {
                    if root.is_none() {
                        ui.label(
                            RichText::new("Open a folder (Ctrl+Shift+O) to search workspace files.")
                                .color(theme.fg_muted),
                        );
                    } else if file_results.is_empty() {
                        ui.label(RichText::new("No matching files.").color(theme.fg_muted));
                    } else {
                        render_file_rows(ui, &file_results, state.selected, theme, &mut chosen, &mut state.open);
                    }
                }
                QuickOpenTab::Recent => {
                    if recent_results.is_empty() {
                        ui.label(
                            RichText::new("No recent files yet. Open a file to populate history.")
                                .color(theme.fg_muted),
                        );
                    } else {
                        render_file_rows(ui, &recent_results, state.selected, theme, &mut chosen, &mut state.open);
                    }
                }
                QuickOpenTab::Symbols => {
                    if symbol_results.is_empty() {
                        ui.label(
                            RichText::new("No symbols found in active document.")
                                .color(theme.fg_muted),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(340.0)
                            .show(ui, |ui| {
                                for (i, sym) in symbol_results.iter().enumerate() {
                                    let is_sel = i == state.selected;
                                    let bg = if is_sel {
                                        theme.bg_popup_selected
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    egui::Frame::NONE
                                        .fill(bg)
                                        .corner_radius(4.0)
                                        .inner_margin(egui::Margin::symmetric(8, 4))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let indent = (sym.depth as f32 * 10.0).min(30.0);
                                                if indent > 0.0 {
                                                    ui.add_space(indent);
                                                }
                                                egui::Frame::NONE
                                                    .fill(theme.bg_tab_inactive)
                                                    .corner_radius(3.0)
                                                    .inner_margin(egui::Margin::symmetric(4, 1))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new(sym.kind.badge())
                                                                .monospace()
                                                                .size(10.5)
                                                                .color(theme.accent),
                                                        );
                                                    });
                                                let resp = ui.add(
                                                    egui::Button::new(
                                                        RichText::new(&sym.name)
                                                            .strong()
                                                            .color(if is_sel {
                                                                theme.accent
                                                            } else {
                                                                theme.fg_text
                                                            })
                                                            .size(13.0),
                                                    )
                                                    .frame(false),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(format!(
                                                                ":{}",
                                                                sym.start_line + 1
                                                            ))
                                                            .monospace()
                                                            .size(11.5)
                                                            .color(theme.fg_muted),
                                                        );
                                                    },
                                                );
                                                if resp.clicked() {
                                                    chosen = Some(QuickOpenAction::JumpToLine(
                                                        sym.start_line,
                                                    ));
                                                    state.open = false;
                                                }
                                            });
                                        });
                                }
                            });
                    }
                }
            }
        });

    if close_clicked {
        state.open = false;
    } else if state.opened_this_frame {
        state.opened_this_frame = false;
    } else if let Some(resp) = inner_resp {
        let clicked_outside = ctx.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !resp.response.rect.contains(pos))
        });
        if clicked_outside {
            state.open = false;
        }
    }

    chosen
}

fn render_file_rows(
    ui: &mut egui::Ui,
    rows: &[(PathBuf, String)],
    selected: usize,
    theme: &Theme,
    chosen: &mut Option<QuickOpenAction>,
    open_flag: &mut bool,
) {
    egui::ScrollArea::vertical()
        .max_height(340.0)
        .show(ui, |ui| {
            for (i, (path, rel)) in rows.iter().enumerate() {
                let is_sel = i == selected;
                let bg = if is_sel {
                    theme.bg_popup_selected
                } else {
                    egui::Color32::TRANSPARENT
                };
                egui::Frame::NONE
                    .fill(bg)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon_r, _) =
                                ui.allocate_exact_size(Vec2::new(16.0, 15.0), Sense::hover());
                            icons::paint_file_icon(ui.painter(), icon_r, path, theme);

                            let filename = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(rel);
                            let resp = ui.add(
                                egui::Button::new(
                                    RichText::new(filename)
                                        .strong()
                                        .color(if is_sel {
                                            theme.accent
                                        } else {
                                            theme.fg_text
                                        })
                                        .size(13.0),
                                )
                                .frame(false),
                            );
                            if rel != filename {
                                ui.label(
                                    RichText::new(rel)
                                        .color(theme.fg_muted)
                                        .size(11.5),
                                );
                            }
                            if resp.clicked() {
                                *chosen = Some(QuickOpenAction::OpenFile(path.clone()));
                                *open_flag = false;
                            }
                        });
                    });
            }
        });
}
