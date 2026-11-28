use std::path::PathBuf;

use eframe::egui::{self, Key, RichText, Stroke, Vec2};

use crate::theme::Theme;
use crate::workspace::search::{WorkspaceMatch, WorkspaceSearchOptions, WorkspaceSearchSnapshot};

#[derive(Default)]
pub struct WorkspaceSearchUiState {
    pub query: String,
    pub replacement: String,
    pub show_replace: bool,
    pub show_filters: bool,
    pub options: WorkspaceSearchOptions,
    pub request_focus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSearchAction {
    RunSearch,
    ReplaceAll,
    OpenMatch {
        path: PathBuf,
        line: usize,
        col: usize,
    },
}

pub fn show_workspace_search(
    ui: &mut egui::Ui,
    state: &mut WorkspaceSearchUiState,
    snapshot: &WorkspaceSearchSnapshot,
    has_workspace: bool,
    theme: &Theme,
) -> Option<WorkspaceSearchAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("SEARCH IN FILES")
                .size(11.0)
                .strong()
                .color(theme.fg_muted),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(state.show_filters, RichText::new("⋯").size(12.0))
                .on_hover_text("Toggle Include / Exclude filters")
                .clicked()
            {
                state.show_filters = !state.show_filters;
            }
            if ui
                .selectable_label(state.show_replace, RichText::new("⇄").size(12.0))
                .on_hover_text("Toggle Replace in Files")
                .clicked()
            {
                state.show_replace = !state.show_replace;
            }
        });
    });
    ui.add_space(4.0);

    if !has_workspace {
        ui.label(
            RichText::new("Open a folder (Ctrl+Shift+O) to search across files.")
                .size(12.0)
                .color(theme.fg_muted),
        );
        return None;
    }

    // Search input
    let resp = ui.add_sized(
        Vec2::new(ui.available_width(), 24.0),
        egui::TextEdit::singleline(&mut state.query).hint_text("Search across workspace…"),
    );
    if state.request_focus {
        resp.request_focus();
        state.request_focus = false;
    }
    if resp.changed() || (resp.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
        action = Some(WorkspaceSearchAction::RunSearch);
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if ui
            .selectable_label(state.options.case_sensitive, RichText::new("Aa").size(11.0))
            .on_hover_text("Match Case")
            .clicked()
        {
            state.options.case_sensitive = !state.options.case_sensitive;
            action = Some(WorkspaceSearchAction::RunSearch);
        }
        if ui
            .selectable_label(state.options.whole_word, RichText::new("W").size(11.0))
            .on_hover_text("Whole Word")
            .clicked()
        {
            state.options.whole_word = !state.options.whole_word;
            action = Some(WorkspaceSearchAction::RunSearch);
        }
        if ui
            .selectable_label(state.options.use_regex, RichText::new(".*").size(11.0))
            .on_hover_text("Use Regular Expression")
            .clicked()
        {
            state.options.use_regex = !state.options.use_regex;
            action = Some(WorkspaceSearchAction::RunSearch);
        }
    });

    if state.show_replace {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 68.0).max(80.0);
            ui.add_sized(
                Vec2::new(w, 22.0),
                egui::TextEdit::singleline(&mut state.replacement).hint_text("Replace with…"),
            );
            if ui
                .add_enabled(
                    !state.query.is_empty() && !snapshot.matches.is_empty(),
                    egui::Button::new(RichText::new("Replace").size(11.0)),
                )
                .on_hover_text("Replace all matches in workspace files")
                .clicked()
            {
                action = Some(WorkspaceSearchAction::ReplaceAll);
            }
        });
    }

    if state.show_filters {
        ui.add_space(4.0);
        let inc = ui.add_sized(
            Vec2::new(ui.available_width(), 22.0),
            egui::TextEdit::singleline(&mut state.options.include_filter)
                .hint_text("Include (e.g. *.rs, src/)"),
        );
        ui.add_space(2.0);
        let exc = ui.add_sized(
            Vec2::new(ui.available_width(), 22.0),
            egui::TextEdit::singleline(&mut state.options.exclude_filter)
                .hint_text("Exclude (e.g. tests/, *.lock)"),
        );
        if inc.changed() || exc.changed() {
            action = Some(WorkspaceSearchAction::RunSearch);
        }
    }

    ui.add_space(6.0);
    if let Some(err) = &snapshot.error {
        ui.label(
            RichText::new(format!("Regex error: {err}"))
                .size(11.5)
                .color(theme.diag_error),
        );
        return action;
    }

    if snapshot.searching {
        ui.label(RichText::new("Searching…").size(11.5).color(theme.fg_muted));
    } else if !state.query.is_empty() {
        let mut file_count = 0usize;
        let mut prev: Option<&str> = None;
        for m in &snapshot.matches {
            if prev != Some(m.rel_path.as_str()) {
                file_count += 1;
                prev = Some(m.rel_path.as_str());
            }
        }
        let suffix = if snapshot.truncated { "+" } else { "" };
        ui.label(
            RichText::new(format!(
                "{}{suffix} results in {file_count} files",
                snapshot.matches.len()
            ))
            .size(11.5)
            .color(theme.fg_muted),
        );
    }

    ui.separator();

    egui::ScrollArea::vertical()
        .id_salt("draco_workspace_search_results")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut last_file: Option<&str> = None;
            for m in &snapshot.matches {
                if last_file != Some(m.rel_path.as_str()) {
                    if last_file.is_some() {
                        ui.add_space(4.0);
                    }
                    last_file = Some(m.rel_path.as_str());
                    ui.label(
                        RichText::new(&m.rel_path)
                            .size(11.5)
                            .strong()
                            .color(theme.accent),
                    );
                }
                if render_match_row(ui, m, theme) {
                    action = Some(WorkspaceSearchAction::OpenMatch {
                        path: m.path.clone(),
                        line: m.line,
                        col: m.col,
                    });
                }
            }
        });

    action
}

fn render_match_row(ui: &mut egui::Ui, m: &WorkspaceMatch, theme: &Theme) -> bool {
    let trimmed = m.line_text.trim();
    let preview: String = trimmed.chars().take(72).collect();
    let label = format!("{}: {}", m.line + 1, preview);
    let resp = egui::Frame::NONE
        .fill(egui::Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .inner_margin(egui::Margin::symmetric(4, 2))
        .show(ui, |ui| {
            ui.add(
                egui::Button::new(
                    RichText::new(label)
                        .monospace()
                        .size(11.5)
                        .color(theme.fg_text),
                )
                .frame(false)
                .truncate(),
            )
        })
        .inner;
    resp.clicked()
}
