use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, Key, RichText, Sense, Stroke, Vec2};

use crate::theme::Theme;
use crate::ui::icons;
use crate::workspace::fuzzy::WorkspaceIndexer;

#[derive(Default)]
pub struct QuickOpenState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub request_focus: bool,
    pub opened_this_frame: bool,
}

impl QuickOpenState {
    pub fn show_modal(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.request_focus = true;
        self.opened_this_frame = true;
    }
}

pub fn show_quick_open(
    ctx: &egui::Context,
    state: &mut QuickOpenState,
    indexer: &WorkspaceIndexer,
    root: Option<&Path>,
    theme: &Theme,
) -> Option<PathBuf> {
    if !state.open {
        return None;
    }

    let mut chosen = None;
    let results = root
        .map(|r| indexer.query(r, &state.query, 25))
        .unwrap_or_default();

    if !results.is_empty() {
        state.selected = state.selected.min(results.len() - 1);
    } else {
        state.selected = 0;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !results.is_empty() {
        state.selected = (state.selected + 1) % results.len();
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !results.is_empty() {
        state.selected = if state.selected == 0 {
            results.len() - 1
        } else {
            state.selected - 1
        };
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        if let Some((path, _, _)) = results.get(state.selected) {
            chosen = Some(path.clone());
            state.open = false;
            return chosen;
        }
    }

    let mut close_clicked = false;
    let inner_resp = egui::Window::new("Quick Open")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 56.0))
        .default_width(540.0)
        .max_width(540.0)
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
            ui.horizontal(|ui| {
                let resp = ui.add_sized(
                    Vec2::new(492.0, 28.0),
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("Search files by name (Ctrl+P)…"),
                );
                if state.request_focus {
                    resp.request_focus();
                    state.request_focus = false;
                }
                if resp.changed() {
                    state.selected = 0;
                }
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
            });

            ui.add_space(6.0);
            if root.is_none() {
                ui.label(
                    RichText::new("Open a folder (Ctrl+Shift+O) to search workspace files.")
                        .color(theme.fg_muted),
                );
            } else if results.is_empty() {
                ui.label(RichText::new("No matching files.").color(theme.fg_muted));
            } else {
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        for (i, (path, rel, _)) in results.iter().enumerate() {
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
                                        let (icon_r, _) = ui.allocate_exact_size(
                                            Vec2::new(16.0, 15.0),
                                            Sense::hover(),
                                        );
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
                                            chosen = Some(path.clone());
                                            state.open = false;
                                        }
                                    });
                                });
                        }
                    });
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
