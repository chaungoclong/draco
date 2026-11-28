use eframe::egui::{self, Align2, Key, Pos2, RichText, Stroke, Vec2};

use crate::lsp::client::{CodeActionItem, SignatureHelpInfo};
use crate::theme::Theme;

#[derive(Default)]
pub struct CodeActionPopupState {
    pub open: bool,
    pub items: Vec<CodeActionItem>,
    pub selected: usize,
}

impl CodeActionPopupState {
    pub fn show(&mut self, items: Vec<CodeActionItem>) {
        self.items = items;
        self.selected = 0;
        self.open = !self.items.is_empty();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.items.clear();
    }
}

pub fn show_code_action_popup(
    ctx: &egui::Context,
    state: &mut CodeActionPopupState,
    anchor: Pos2,
    theme: &Theme,
) -> Option<CodeActionItem> {
    if !state.open || state.items.is_empty() {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.close();
        return None;
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
        state.selected = (state.selected + 1) % state.items.len();
    }
    if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
        state.selected = if state.selected == 0 {
            state.items.len() - 1
        } else {
            state.selected - 1
        };
    }
    if ctx.input(|i| i.key_pressed(Key::Enter)) {
        let item = state.items.get(state.selected).cloned();
        state.close();
        return item;
    }

    let mut chosen = None;
    egui::Area::new(egui::Id::new("draco_code_actions_popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new(anchor.x, anchor.y + 22.0))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(6.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_min_width(280.0);
                    ui.label(
                        RichText::new("Quick Fix / Code Actions (Alt+Enter)")
                            .size(11.0)
                            .color(theme.fg_muted),
                    );
                    ui.add_space(4.0);
                    for (i, item) in state.items.iter().enumerate() {
                        let is_sel = i == state.selected;
                        let bg = if is_sel {
                            theme.bg_popup_selected
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        egui::Frame::NONE
                            .fill(bg)
                            .corner_radius(4.0)
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                let resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(&item.title)
                                            .size(12.5)
                                            .color(if is_sel {
                                                theme.accent
                                            } else {
                                                theme.fg_text
                                            }),
                                    )
                                    .frame(false),
                                );
                                if resp.clicked() {
                                    chosen = Some(item.clone());
                                }
                            });
                    }
                });
        });

    if chosen.is_some() {
        state.close();
    }
    chosen
}

#[derive(Default)]
pub struct RenameModalState {
    pub open: bool,
    pub new_name: String,
    pub request_focus: bool,
}

impl RenameModalState {
    pub fn show(&mut self, current_symbol: String) {
        self.open = true;
        self.new_name = current_symbol;
        self.request_focus = true;
    }
}

pub fn show_rename_modal(
    ctx: &egui::Context,
    state: &mut RenameModalState,
    theme: &Theme,
) -> Option<String> {
    if !state.open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }

    let mut confirmed = None;
    egui::Window::new("Rename Symbol")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 96.0))
        .default_width(360.0)
        .frame(
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(8.0)
                .inner_margin(10.0),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Rename Symbol (F2)")
                    .size(12.0)
                    .strong()
                    .color(theme.accent),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let resp = ui.add_sized(
                    Vec2::new(260.0, 26.0),
                    egui::TextEdit::singleline(&mut state.new_name).hint_text("New identifier…"),
                );
                if state.request_focus {
                    resp.request_focus();
                    state.request_focus = false;
                }
                if (resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                    || ui.button("Rename").clicked()
                {
                    let trimmed = state.new_name.trim().to_string();
                    if !trimmed.is_empty() {
                        confirmed = Some(trimmed);
                    }
                    state.open = false;
                }
            });
        });

    confirmed
}

pub fn show_signature_help_tooltip(
    ctx: &egui::Context,
    info: Option<&SignatureHelpInfo>,
    anchor: Pos2,
    theme: &Theme,
) {
    let Some(sig) = info else {
        return;
    };
    let pos = Pos2::new(anchor.x, (anchor.y - 34.0).max(8.0));
    egui::Area::new(egui::Id::new("draco_signature_help"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(5.0)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(&sig.label)
                            .monospace()
                            .size(12.0)
                            .color(theme.fg_text),
                    );
                });
        });
}
