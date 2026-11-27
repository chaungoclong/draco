use eframe::egui::{self, Key, Pos2, RichText, Stroke};

use crate::lsp::client::CompletionCandidate;
use crate::theme::Theme;

#[derive(Default)]
pub struct CompletionState {
    pub open: bool,
    pub items: Vec<CompletionCandidate>,
    pub selected: usize,
}

impl CompletionState {
    pub fn show_items(&mut self, items: Vec<CompletionCandidate>) {
        if items.is_empty() {
            self.open = false;
            self.items.clear();
        } else {
            self.items = items;
            self.selected = 0;
            self.open = true;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.items.clear();
    }
}

pub fn show_completion_popup(
    ctx: &egui::Context,
    state: &mut CompletionState,
    anchor: Pos2,
    theme: &Theme,
) -> Option<String> {
    if !state.open || state.items.is_empty() {
        return None;
    }

    state.selected = state.selected.min(state.items.len() - 1);

    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
        state.close();
        return None;
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown)) {
        state.selected = (state.selected + 1) % state.items.len();
    }
    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)) {
        state.selected = if state.selected == 0 {
            state.items.len() - 1
        } else {
            state.selected - 1
        };
    }
    if ctx.input_mut(|i| {
        i.consume_key(egui::Modifiers::NONE, Key::Enter)
            || i.consume_key(egui::Modifiers::NONE, Key::Tab)
    }) {
        let text = state.items[state.selected].insert_text.clone();
        state.close();
        return Some(text);
    }

    let mut chosen = None;
    egui::Area::new(egui::Id::new("draco_completion_popup"))
        .fixed_pos(anchor)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(5.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_min_width(240.0);
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (i, item) in state.items.iter().enumerate() {
                                let is_sel = i == state.selected;
                                let bg = if is_sel {
                                    theme.bg_popup_selected
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                egui::Frame::NONE
                                    .fill(bg)
                                    .corner_radius(3.0)
                                    .inner_margin(egui::Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        let label = if let Some(d) = &item.detail {
                                            format!("{}  {}", item.label, d)
                                        } else {
                                            item.label.clone()
                                        };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(label)
                                                        .color(if is_sel {
                                                            theme.accent
                                                        } else {
                                                            theme.fg_text
                                                        })
                                                        .monospace()
                                                        .size(12.5),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            chosen = Some(item.insert_text.clone());
                                        }
                                    });
                            }
                        });
                });
        });

    if chosen.is_some() {
        state.close();
    }
    chosen
}
