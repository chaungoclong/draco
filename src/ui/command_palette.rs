use eframe::egui::{self, Align2, Key, RichText, Stroke, Vec2};

use crate::theme::Theme;
use crate::workspace::fuzzy::fuzzy_score;

#[derive(Debug, Clone)]
pub enum PaletteCommand {
    ToggleAutoSave,
    OpenSettings,
    NewTerminalTab,
    TerminalBottom,
    TerminalRight,
    TerminalFloating,
    TerminalMaximized,
    ToggleTheme,
    PluginCommand {
        plugin_id: String,
        command_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub title: String,
    pub category: String,
    pub command: PaletteCommand,
}

#[derive(Default)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    pub request_focus: bool,
    pub opened_this_frame: bool,
}

impl CommandPaletteState {
    pub fn show_modal(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.request_focus = true;
        self.opened_this_frame = true;
    }
}

pub fn show_command_palette(
    ctx: &egui::Context,
    state: &mut CommandPaletteState,
    entries: &[PaletteEntry],
    theme: &Theme,
) -> Option<PaletteCommand> {
    if !state.open {
        return None;
    }

    let mut filtered: Vec<(&PaletteEntry, i32)> = entries
        .iter()
        .filter_map(|e| {
            let label = format!("{}: {}", e.category, e.title);
            fuzzy_score(&state.query, &label).map(|s| (e, s))
        })
        .collect();
    filtered.sort_by(|a, b| b.1.cmp(&a.1));

    if !filtered.is_empty() {
        state.selected = state.selected.min(filtered.len() - 1);
    } else {
        state.selected = 0;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !filtered.is_empty() {
        state.selected = (state.selected + 1) % filtered.len();
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !filtered.is_empty() {
        state.selected = if state.selected == 0 {
            filtered.len() - 1
        } else {
            state.selected - 1
        };
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        if let Some((entry, _)) = filtered.get(state.selected) {
            state.open = false;
            return Some(entry.command.clone());
        }
    }

    let mut chosen = None;
    let mut close_clicked = false;
    let inner_resp = egui::Window::new("Command Palette")
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
                    color: egui::Color32::from_black_alpha(95),
                })
                .inner_margin(10.0),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let resp = ui.add_sized(
                    Vec2::new(492.0, 28.0),
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("> Run command or plugin (Ctrl+Shift+P)…"),
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

            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (i, (entry, _)) in filtered.iter().enumerate() {
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
                                    ui.label(
                                        RichText::new(&entry.category)
                                            .size(11.5)
                                            .color(theme.fg_muted),
                                    );
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(&entry.title)
                                                    .color(if is_sel {
                                                        theme.accent
                                                    } else {
                                                        theme.fg_text
                                                    })
                                                    .size(13.0),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        chosen = Some(entry.command.clone());
                                        state.open = false;
                                    }
                                });
                            });
                    }
                });
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
