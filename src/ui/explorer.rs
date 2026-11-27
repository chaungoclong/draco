use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::theme::Theme;
use crate::ui::icons;
use crate::workspace::file_tree::{FileNode, FileTree};

#[derive(Debug, Clone)]
pub enum ExplorerAction {
    OpenFile(PathBuf),
    OpenFolderDialog,
    Refresh,
    CreateFile(PathBuf),
    CreateFolder(PathBuf),
    Rename { from: PathBuf, to: PathBuf },
    Delete(PathBuf),
    CopyPath(String),
}

#[derive(Debug, Clone)]
enum PromptKind {
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    ConfirmDelete(PathBuf),
}

#[derive(Default)]
pub struct ExplorerUiState {
    prompt: Option<PromptKind>,
    input: String,
    request_focus: bool,
}

impl ExplorerUiState {
    fn open_prompt(&mut self, kind: PromptKind, initial: String) {
        self.prompt = Some(kind);
        self.input = initial;
        self.request_focus = true;
    }
}

pub fn show_explorer(
    ui: &mut egui::Ui,
    tree: &mut Option<FileTree>,
    state: &mut ExplorerUiState,
    active_path: Option<&PathBuf>,
    dirty_paths: &[PathBuf],
    theme: &Theme,
) -> Option<ExplorerAction> {
    let mut action = None;

    // Header row
    ui.horizontal(|ui| {
        let title = tree
            .as_ref()
            .map(|t| t.root_node.name.to_uppercase())
            .unwrap_or_else(|| "EXPLORER".to_string());
        ui.label(RichText::new(title).strong().size(11.5).color(theme.fg_muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("↻").size(13.0).color(theme.fg_muted)).frame(false))
                .on_hover_text("Refresh Explorer")
                .clicked()
            {
                action = Some(ExplorerAction::Refresh);
            }
            if let Some(t) = tree.as_ref() {
                if ui
                    .add(egui::Button::new(RichText::new("+D").size(11.0).color(theme.fg_muted)).frame(false))
                    .on_hover_text("New Folder")
                    .clicked()
                {
                    state.open_prompt(PromptKind::NewFolder(t.root.clone()), String::new());
                }
                if ui
                    .add(egui::Button::new(RichText::new("+F").size(11.0).color(theme.fg_muted)).frame(false))
                    .on_hover_text("New File")
                    .clicked()
                {
                    state.open_prompt(PromptKind::NewFile(t.root.clone()), String::new());
                }
            }
            if ui
                .add(egui::Button::new(RichText::new("Open").size(11.0).color(theme.fg_muted)).frame(false))
                .on_hover_text("Open Folder (Ctrl+Shift+O)")
                .clicked()
            {
                action = Some(ExplorerAction::OpenFolderDialog);
            }
        });
    });
    ui.add_space(2.0);
    ui.separator();

    // Inline prompt for New File / New Folder / Rename / Delete
    if let Some(prompt_act) = show_inline_prompt(ui, state, theme) {
        action = Some(prompt_act);
    }

    if let Some(file_tree) = tree.as_mut() {
        let root_path = file_tree.root.clone();
        egui::ScrollArea::vertical()
            .id_salt("draco_explorer_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                if let Some(children) = file_tree.root_node.children.as_mut() {
                    let mut collapse_all = false;
                    for child in children {
                        render_node(
                            ui,
                            child,
                            0,
                            &root_path,
                            active_path,
                            dirty_paths,
                            theme,
                            state,
                            &mut action,
                            &mut collapse_all,
                        );
                    }
                    if collapse_all {
                        collapse_recursive(&mut file_tree.root_node, true);
                    }
                }
            });
    } else {
        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No folder opened").color(theme.fg_muted));
            ui.add_space(6.0);
            if ui.button("Open Folder…").clicked() {
                action = Some(ExplorerAction::OpenFolderDialog);
            }
        });
    }

    action
}

fn collapse_recursive(node: &mut FileNode, is_root: bool) {
    if !is_root {
        node.expanded = false;
    }
    if let Some(children) = node.children.as_mut() {
        for c in children {
            collapse_recursive(c, false);
        }
    }
}

fn show_inline_prompt(
    ui: &mut egui::Ui,
    state: &mut ExplorerUiState,
    theme: &Theme,
) -> Option<ExplorerAction> {
    let prompt = state.prompt.clone()?;
    let mut result = None;
    let mut close = false;

    egui::Frame::NONE
        .fill(theme.bg_popup)
        .stroke(Stroke::new(1.0_f32, theme.accent))
        .corner_radius(4.0)
        .inner_margin(6.0)
        .show(ui, |ui| match &prompt {
            PromptKind::ConfirmDelete(path) => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("item");
                ui.label(
                    RichText::new(format!("Delete \"{name}\"?"))
                        .color(theme.diag_error)
                        .size(12.0),
                );
                ui.horizontal(|ui| {
                    if ui.small_button("Delete").clicked() {
                        result = Some(ExplorerAction::Delete(path.clone()));
                        close = true;
                    }
                    if ui.small_button("Cancel").clicked() {
                        close = true;
                    }
                });
            }
            PromptKind::NewFile(dir) | PromptKind::NewFolder(dir) | PromptKind::Rename(dir) => {
                let title = match &prompt {
                    PromptKind::NewFile(_) => "New file name:",
                    PromptKind::NewFolder(_) => "New folder name:",
                    PromptKind::Rename(_) => "Rename to:",
                    _ => "",
                };
                ui.label(RichText::new(title).size(11.5).color(theme.fg_muted));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut state.input)
                        .desired_width(ui.available_width())
                        .font(FontId::monospace(12.5)),
                );
                if state.request_focus {
                    resp.request_focus();
                    state.request_focus = false;
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    close = true;
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    let trimmed = state.input.trim();
                    if !trimmed.is_empty() {
                        result = match &prompt {
                            PromptKind::NewFile(_) => Some(ExplorerAction::CreateFile(dir.join(trimmed))),
                            PromptKind::NewFolder(_) => Some(ExplorerAction::CreateFolder(dir.join(trimmed))),
                            PromptKind::Rename(old) => {
                                let new_path = old
                                    .parent()
                                    .map(|p| p.join(trimmed))
                                    .unwrap_or_else(|| PathBuf::from(trimmed));
                                Some(ExplorerAction::Rename {
                                    from: old.clone(),
                                    to: new_path,
                                })
                            }
                            _ => None,
                        };
                    }
                    close = true;
                }
            }
        });

    if close {
        state.prompt = None;
        state.input.clear();
    }
    ui.add_space(4.0);
    result
}

#[allow(clippy::too_many_arguments)]
fn render_node(
    ui: &mut egui::Ui,
    node: &mut FileNode,
    depth: usize,
    root: &Path,
    active_path: Option<&PathBuf>,
    dirty_paths: &[PathBuf],
    theme: &Theme,
    state: &mut ExplorerUiState,
    action: &mut Option<ExplorerAction>,
    collapse_all: &mut bool,
) {
    let row_height = 23.0;
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, row_height), Sense::click());

    let is_active = !node.is_dir && active_path.map(|p| p == &node.path).unwrap_or(false);
    let hovered = response.hovered();
    let painter = ui.painter();

    // Row highlight
    if is_active {
        painter.rect_filled(rect, 4.0, theme.bg_popup_selected.linear_multiply(0.8));
    } else if hovered {
        painter.rect_filled(rect, 4.0, theme.bg_hover);
    }

    // Indent guides
    let step = 14.0;
    for d in 0..depth {
        let gx = rect.min.x + 10.0 + d as f32 * step;
        painter.line_segment(
            [Pos2::new(gx, rect.min.y), Pos2::new(gx, rect.max.y)],
            Stroke::new(1.0_f32, theme.indent_guide),
        );
    }

    let base_x = rect.min.x + 6.0 + depth as f32 * step;
    let cy = rect.center().y;

    if node.is_dir {
        let chev_rect = Rect::from_center_size(Pos2::new(base_x + 5.0, cy), Vec2::new(12.0, 12.0));
        icons::paint_chevron(painter, chev_rect, node.expanded, theme.fg_muted);

        let folder_rect = Rect::from_center_size(Pos2::new(base_x + 21.0, cy), Vec2::new(15.0, 14.0));
        icons::paint_folder_icon(painter, folder_rect, node.expanded, theme);

        painter.text(
            Pos2::new(base_x + 33.0, cy),
            Align2::LEFT_CENTER,
            &node.name,
            FontId::proportional(13.0),
            theme.fg_text,
        );
    } else {
        let is_dirty = dirty_paths.iter().any(|p| p == &node.path);
        let icon_rect = Rect::from_center_size(Pos2::new(base_x + 15.0, cy), Vec2::new(16.0, 15.0));
        icons::paint_file_icon(painter, icon_rect, &node.path, theme);

        painter.text(
            Pos2::new(base_x + 28.0, cy),
            Align2::LEFT_CENTER,
            &node.name,
            FontId::proportional(13.0),
            if is_active {
                theme.accent
            } else if is_dirty {
                theme.diag_warning
            } else {
                theme.fg_text
            },
        );
        if is_dirty {
            painter.circle_filled(Pos2::new(rect.max.x - 10.0, cy), 3.0, theme.diag_warning);
        }
    }

    // Left click
    if response.clicked() {
        if node.is_dir {
            node.toggle();
        } else {
            *action = Some(ExplorerAction::OpenFile(node.path.clone()));
        }
    }

    // Right-click Context Menu
    let node_path = node.path.clone();
    let node_name = node.name.clone();
    let is_dir = node.is_dir;
    response.context_menu(|ui| {
        ui.set_min_width(170.0);
        if is_dir {
            if ui.button("New File…").clicked() {
                state.open_prompt(PromptKind::NewFile(node_path.clone()), String::new());
                ui.close_menu();
            }
            if ui.button("New Folder…").clicked() {
                state.open_prompt(PromptKind::NewFolder(node_path.clone()), String::new());
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Rename…").clicked() {
                state.open_prompt(PromptKind::Rename(node_path.clone()), node_name.clone());
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                state.open_prompt(PromptKind::ConfirmDelete(node_path.clone()), String::new());
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Copy Path").clicked() {
                *action = Some(ExplorerAction::CopyPath(node_path.display().to_string()));
                ui.close_menu();
            }
            if ui.button("Collapse All").clicked() {
                *collapse_all = true;
                ui.close_menu();
            }
            if ui.button("Refresh").clicked() {
                *action = Some(ExplorerAction::Refresh);
                ui.close_menu();
            }
        } else {
            if ui.button("Open").clicked() {
                *action = Some(ExplorerAction::OpenFile(node_path.clone()));
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Rename…").clicked() {
                state.open_prompt(PromptKind::Rename(node_path.clone()), node_name.clone());
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                state.open_prompt(PromptKind::ConfirmDelete(node_path.clone()), String::new());
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Copy Path").clicked() {
                *action = Some(ExplorerAction::CopyPath(node_path.display().to_string()));
                ui.close_menu();
            }
            if ui.button("Copy Relative Path").clicked() {
                let rel = node_path
                    .strip_prefix(root)
                    .unwrap_or(&node_path)
                    .display()
                    .to_string();
                *action = Some(ExplorerAction::CopyPath(rel));
                ui.close_menu();
            }
        }
    });

    if node.is_dir && node.expanded {
        if let Some(children) = node.children.as_mut() {
            for child in children {
                render_node(
                    ui,
                    child,
                    depth + 1,
                    root,
                    active_path,
                    dirty_paths,
                    theme,
                    state,
                    action,
                    collapse_all,
                );
            }
        }
    }
}
