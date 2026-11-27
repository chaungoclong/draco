use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::theme::Theme;
use crate::ui::icons;

pub struct TabInfo {
    pub title: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

pub enum TabAction {
    Select(usize),
    Close(usize),
    CloseOthers(usize),
    CloseAll,
    Save(usize),
    CopyPath(String),
    NewTab,
}

pub fn show_tabs(
    ui: &mut egui::Ui,
    tabs: &[TabInfo],
    active: usize,
    theme: &Theme,
) -> Option<TabAction> {
    let mut action = None;

    egui::Frame::NONE
        .fill(theme.bg_tab_bar)
        .inner_margin(egui::Margin::symmetric(4, 0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                for (idx, tab) in tabs.iter().enumerate() {
                    let is_active = idx == active;
                    let measured_w = ui
                        .fonts(|f| {
                            f.layout_no_wrap(
                                tab.title.clone(),
                                FontId::proportional(12.5),
                                theme.fg_text,
                            )
                            .size()
                            .x
                        })
                        .clamp(48.0, 240.0);
                    let tab_w = 28.0 + measured_w + 26.0;
                    let tab_h = 31.0;

                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(tab_w, tab_h), Sense::click());
                    let hovered = response.hovered();
                    let painter = ui.painter();

                    let bg = if is_active {
                        theme.bg_tab_active
                    } else if hovered {
                        theme.bg_hover
                    } else {
                        theme.bg_tab_inactive
                    };

                    painter.rect_filled(rect, 0.0, bg);
                    // Right separator
                    painter.line_segment(
                        [rect.right_top(), rect.right_bottom()],
                        Stroke::new(1.0_f32, theme.border),
                    );
                    // Top accent line on active tab
                    if is_active {
                        painter.rect_filled(
                            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 2.0)),
                            0.0,
                            theme.accent,
                        );
                    }

                    // File icon
                    let icon_rect = Rect::from_center_size(
                        Pos2::new(rect.min.x + 14.0, rect.center().y),
                        Vec2::new(15.0, 14.0),
                    );
                    let dummy = PathBuf::from(&tab.title);
                    let icon_path = tab.path.as_deref().unwrap_or(dummy.as_path());
                    icons::paint_file_icon(painter, icon_rect, icon_path, theme);

                    // Title
                    let fg = if is_active {
                        theme.fg_text
                    } else {
                        theme.fg_muted
                    };
                    painter.text(
                        Pos2::new(rect.min.x + 26.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        &tab.title,
                        FontId::proportional(12.5),
                        fg,
                    );

                    // Close / Dirty indicator on right
                    let close_rect = Rect::from_center_size(
                        Pos2::new(rect.max.x - 12.0, rect.center().y),
                        Vec2::new(16.0, 16.0),
                    );
                    let close_hovered = response
                        .hover_pos()
                        .map(|p| close_rect.contains(p))
                        .unwrap_or(false);

                    if tab.dirty && !close_hovered {
                        painter.circle_filled(close_rect.center(), 3.2, theme.accent);
                    } else if hovered || is_active {
                        if close_hovered {
                            painter.rect_filled(close_rect, 3.0, theme.bg_popup_selected);
                        }
                        painter.text(
                            close_rect.center(),
                            Align2::CENTER_CENTER,
                            "×",
                            FontId::proportional(13.5),
                            if close_hovered {
                                theme.fg_text
                            } else {
                                theme.fg_muted
                            },
                        );
                    }

                    // Click handling
                    if response.middle_clicked() {
                        action = Some(TabAction::Close(idx));
                    } else if response.clicked() {
                        if close_hovered {
                            action = Some(TabAction::Close(idx));
                        } else {
                            action = Some(TabAction::Select(idx));
                        }
                    }

                    // Tab right-click context menu
                    let path_str = tab.path.as_ref().map(|p| p.display().to_string());
                    response.context_menu(|ui| {
                        ui.set_min_width(160.0);
                        if ui.button("Save (Ctrl+S)").clicked() {
                            action = Some(TabAction::Save(idx));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Close (Ctrl+W)").clicked() {
                            action = Some(TabAction::Close(idx));
                            ui.close_menu();
                        }
                        if ui.button("Close Others").clicked() {
                            action = Some(TabAction::CloseOthers(idx));
                            ui.close_menu();
                        }
                        if ui.button("Close All").clicked() {
                            action = Some(TabAction::CloseAll);
                            ui.close_menu();
                        }
                        if let Some(p) = &path_str {
                            ui.separator();
                            if ui.button("Copy Path").clicked() {
                                action = Some(TabAction::CopyPath(p.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                }

                ui.add_space(4.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("+").size(15.0).color(theme.fg_muted))
                            .frame(false),
                    )
                    .on_hover_text("New File (Ctrl+N)")
                    .clicked()
                {
                    action = Some(TabAction::NewTab);
                }
            });
        });

    action
}

/// Compact breadcrumb bar below the tab strip showing relative path segments and file save state.
pub fn show_breadcrumbs(
    ui: &mut egui::Ui,
    active_path: Option<&Path>,
    workspace_root: Option<&Path>,
    dirty: bool,
    theme: &Theme,
) {
    egui::Frame::NONE
        .fill(theme.bg_editor)
        .inner_margin(egui::Margin::symmetric(12, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                if let Some(path) = active_path {
                    let rel = workspace_root
                        .and_then(|r| path.strip_prefix(r).ok())
                        .unwrap_or(path);
                    let parts: Vec<String> = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect();
                    for (i, seg) in parts.iter().enumerate() {
                        let is_last = i + 1 == parts.len();
                        if is_last {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(15.0, 14.0), Sense::hover());
                            icons::paint_file_icon(ui.painter(), r, path, theme);
                            ui.label(RichText::new(seg).size(12.0).color(theme.fg_text));
                        } else {
                            ui.label(RichText::new(seg).size(12.0).color(theme.fg_muted));
                            ui.label(RichText::new("›").size(12.0).color(theme.fg_gutter));
                        }
                    }
                } else {
                    ui.label(RichText::new("Untitled").size(12.0).color(theme.fg_muted));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (badge, color) = if active_path.is_none() {
                        if dirty {
                            ("● Unsaved", theme.diag_warning)
                        } else {
                            ("New File", theme.fg_muted)
                        }
                    } else if dirty {
                        ("● Modified", theme.diag_warning)
                    } else {
                        ("✓ Saved", theme.syn_string)
                    };
                    ui.label(RichText::new(badge).size(11.5).color(color));
                });
            });
        });
}
