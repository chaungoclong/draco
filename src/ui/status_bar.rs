use eframe::egui::{self, RichText};

use crate::theme::{Theme, ThemeKind};

pub struct StatusInfo<'a> {
    pub line: usize,
    pub col: usize,
    pub cursor_count: usize,
    pub language: &'a str,
    pub lsp_status: Option<&'a str>,
    pub errors: usize,
    pub warnings: usize,
    pub dirty: bool,
    pub has_path: bool,
    pub auto_save: bool,
    pub message: Option<&'a str>,
}

pub enum StatusAction {
    ToggleTerminal,
    ToggleTheme,
    ToggleAutoSave,
    OpenSettings,
}

pub fn show_status_bar(
    ui: &mut egui::Ui,
    info: &StatusInfo<'_>,
    theme: &Theme,
) -> Option<StatusAction> {
    let mut action = None;

    egui::Frame::NONE
        .fill(theme.bg_status)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Accent pill for Draco
                egui::Frame::NONE
                    .fill(theme.accent.linear_multiply(0.18))
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(6, 1))
                    .show(ui, |ui| {
                        ui.label(RichText::new("DRACO").strong().color(theme.accent).size(11.0));
                    });

                ui.add_space(4.0);
                let diag_color = if info.errors > 0 {
                    theme.diag_error
                } else if info.warnings > 0 {
                    theme.diag_warning
                } else {
                    theme.fg_muted
                };
                ui.label(
                    RichText::new(format!("⊗ {}  △ {}", info.errors, info.warnings))
                        .color(diag_color)
                        .size(11.5),
                );

                if let Some(lsp) = info.lsp_status {
                    ui.separator();
                    ui.label(RichText::new(lsp).color(theme.fg_muted).size(11.5));
                }

                if let Some(msg) = info.message {
                    ui.separator();
                    ui.label(RichText::new(msg).color(theme.fg_muted).size(11.5));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Settings").color(theme.fg_muted).size(11.5),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Open Settings & Plugins (Ctrl+,)")
                        .clicked()
                    {
                        action = Some(StatusAction::OpenSettings);
                    }
                    ui.separator();
                    let theme_label = match theme.kind {
                        ThemeKind::Dark => "Light",
                        ThemeKind::Light => "Dark",
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(theme_label).color(theme.fg_muted).size(11.5),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Toggle Theme (Ctrl+Shift+T)")
                        .clicked()
                    {
                        action = Some(StatusAction::ToggleTheme);
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Terminal").color(theme.fg_muted).size(11.5),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Toggle Terminal (Ctrl+`)")
                        .clicked()
                    {
                        action = Some(StatusAction::ToggleTerminal);
                    }
                    ui.separator();
                    let auto_label = if info.auto_save {
                        "Auto Save: ON"
                    } else {
                        "Auto Save: OFF"
                    };
                    let auto_color = if info.auto_save {
                        theme.syn_string
                    } else {
                        theme.fg_muted
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(auto_label).color(auto_color).size(11.5))
                                .frame(false),
                        )
                        .on_hover_text("Click to toggle Auto Save")
                        .clicked()
                    {
                        action = Some(StatusAction::ToggleAutoSave);
                    }
                    ui.separator();
                    let (save_label, save_color) = if !info.has_path {
                        if info.dirty {
                            ("● Unsaved", theme.diag_warning)
                        } else {
                            ("New", theme.fg_muted)
                        }
                    } else if info.dirty {
                        ("● Modified", theme.diag_warning)
                    } else {
                        ("✓ Saved", theme.syn_string)
                    };
                    ui.label(RichText::new(save_label).color(save_color).size(11.5));
                    ui.separator();
                    ui.label(RichText::new("UTF-8").color(theme.fg_muted).size(11.5));
                    ui.separator();
                    ui.label(RichText::new(info.language).color(theme.fg_text).size(11.5));
                    ui.separator();
                    let pos_text = if info.cursor_count > 1 {
                        format!(
                            "Ln {}, Col {} ({} cursors)",
                            info.line + 1,
                            info.col + 1,
                            info.cursor_count
                        )
                    } else {
                        format!("Ln {}, Col {}", info.line + 1, info.col + 1)
                    };
                    ui.label(RichText::new(pos_text).color(theme.fg_text).size(11.5));
                });
            });
        });

    action
}
