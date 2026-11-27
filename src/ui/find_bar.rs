use std::ops::Range;

use eframe::egui::{self, Key, RichText, Stroke};

use crate::core::search::SearchOptions;
use crate::theme::Theme;

#[derive(Default)]
pub struct FindBarState {
    pub open: bool,
    pub show_replace: bool,
    pub query: String,
    pub replacement: String,
    pub options: SearchOptions,
    pub matches: Vec<Range<usize>>,
    pub error: Option<String>,
    pub request_focus: bool,
}

pub enum FindAction {
    QueryChanged,
    Next,
    Prev,
    ReplaceOne,
    ReplaceAll,
    Close,
}

pub fn show_find_bar(
    ui: &mut egui::Ui,
    state: &mut FindBarState,
    theme: &Theme,
) -> Option<FindAction> {
    if !state.open {
        return None;
    }

    let mut action = None;

    egui::Frame::NONE
        .fill(theme.bg_popup)
        .stroke(Stroke::new(1.0_f32, theme.border))
        .inner_margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .desired_width(220.0)
                        .hint_text("Find…"),
                );
                if state.request_focus {
                    resp.request_focus();
                    state.request_focus = false;
                }
                if resp.changed() {
                    action = Some(FindAction::QueryChanged);
                }
                if resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    action = Some(if ui.input(|i| i.modifiers.shift) {
                        FindAction::Prev
                    } else {
                        FindAction::Next
                    });
                }

                if ui.selectable_label(state.options.case_sensitive, "Aa").on_hover_text("Case sensitive").clicked() {
                    state.options.case_sensitive = !state.options.case_sensitive;
                    action = Some(FindAction::QueryChanged);
                }
                if ui.selectable_label(state.options.whole_word, "W").on_hover_text("Whole word").clicked() {
                    state.options.whole_word = !state.options.whole_word;
                    action = Some(FindAction::QueryChanged);
                }
                if ui.selectable_label(state.options.use_regex, ".*").on_hover_text("Regex").clicked() {
                    state.options.use_regex = !state.options.use_regex;
                    action = Some(FindAction::QueryChanged);
                }

                if ui.small_button("↑").clicked() {
                    action = Some(FindAction::Prev);
                }
                if ui.small_button("↓").clicked() {
                    action = Some(FindAction::Next);
                }

                let count_label = if let Some(err) = &state.error {
                    RichText::new(format!("Err: {err}")).color(theme.diag_error).size(11.5)
                } else {
                    RichText::new(format!("{} matches", state.matches.len()))
                        .color(theme.fg_muted)
                        .size(11.5)
                };
                ui.label(count_label);

                if ui.small_button("×").clicked() {
                    state.open = false;
                    action = Some(FindAction::Close);
                }
            });

            if state.show_replace {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.replacement)
                            .desired_width(220.0)
                            .hint_text("Replace with…"),
                    );
                    if ui.small_button("Replace").clicked() {
                        action = Some(FindAction::ReplaceOne);
                    }
                    if ui.small_button("Replace All").clicked() {
                        action = Some(FindAction::ReplaceAll);
                    }
                });
            }
        });

    if ui.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return Some(FindAction::Close);
    }

    action
}
