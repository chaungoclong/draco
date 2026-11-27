use eframe::egui::{self, Color32, Event, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::settings::TerminalLayout;
use crate::terminal::grid::TermColor;
use crate::terminal::manager::TerminalManager;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalUiAction {
    NewTab,
    SelectTab(usize),
    CloseTab(usize),
    SetLayout(TerminalLayout),
    Hide,
}

fn ansi_to_color(c: TermColor, default: Color32) -> Color32 {
    match c {
        TermColor::Default => default,
        TermColor::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        TermColor::Indexed(idx) => match idx {
            0 => Color32::from_rgb(24, 25, 36),
            1 => Color32::from_rgb(247, 118, 142),
            2 => Color32::from_rgb(158, 206, 106),
            3 => Color32::from_rgb(224, 175, 104),
            4 => Color32::from_rgb(122, 162, 247),
            5 => Color32::from_rgb(187, 154, 247),
            6 => Color32::from_rgb(125, 207, 255),
            7 => Color32::from_rgb(192, 202, 245),
            8 => Color32::from_rgb(92, 103, 140),
            9 => Color32::from_rgb(255, 138, 160),
            10 => Color32::from_rgb(185, 224, 130),
            11 => Color32::from_rgb(240, 195, 128),
            12 => Color32::from_rgb(148, 184, 255),
            13 => Color32::from_rgb(208, 178, 255),
            14 => Color32::from_rgb(155, 224, 255),
            15 => Color32::from_rgb(240, 244, 255),
            _ => default,
        },
    }
}

pub fn show_terminal_panel(
    ui: &mut egui::Ui,
    manager: &mut TerminalManager,
    terminal_focused: &mut bool,
    theme: &Theme,
) -> Option<TerminalUiAction> {
    let mut action = None;

    // Header: Terminal tabs + "+" + Display Mode Switcher + Hide
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (idx, tab) in manager.tabs.iter().enumerate() {
            let is_active = idx == manager.active;
            let bg = if is_active {
                theme.bg_popup_selected
            } else {
                theme.bg_tab_inactive
            };
            let fg = if is_active {
                theme.accent
            } else {
                theme.fg_muted
            };
            egui::Frame::NONE
                .fill(bg)
                .stroke(Stroke::new(
                    1.0_f32,
                    if is_active { theme.accent } else { theme.border },
                ))
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(7, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let resp = ui.add(
                            egui::Button::new(RichText::new(&tab.title).color(fg).size(11.5))
                                .frame(false),
                        );
                        if resp.middle_clicked() {
                            action = Some(TerminalUiAction::CloseTab(idx));
                        } else if resp.clicked() {
                            action = Some(TerminalUiAction::SelectTab(idx));
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("×").color(theme.fg_muted).size(11.5),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            action = Some(TerminalUiAction::CloseTab(idx));
                        }
                    });
                });
        }

        if ui
            .add(
                egui::Button::new(RichText::new("+").color(theme.fg_text).size(13.0)).frame(false),
            )
            .on_hover_text("New Terminal Tab (Ctrl+Shift+`)")
            .clicked()
        {
            action = Some(TerminalUiAction::NewTab);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("×").color(theme.fg_muted).size(13.0)).frame(false))
                .on_hover_text("Hide Terminal (Ctrl+`)")
                .clicked()
            {
                action = Some(TerminalUiAction::Hide);
            }
            ui.separator();
            for (mode, label) in [
                (TerminalLayout::Maximized, "Max"),
                (TerminalLayout::Floating, "Float"),
                (TerminalLayout::Right, "Right"),
                (TerminalLayout::Bottom, "Bottom"),
            ] {
                let active = manager.layout == mode;
                let text = RichText::new(label)
                    .size(11.0)
                    .color(if active { theme.accent } else { theme.fg_muted });
                if ui.selectable_label(active, text).clicked() {
                    action = Some(TerminalUiAction::SetLayout(mode));
                }
            }
        });
    });
    ui.separator();

    let Some(pty) = manager.active_session_mut() else {
        ui.label(RichText::new("No active terminal. Click '+' to create one.").color(theme.fg_muted));
        return action;
    };

    let font_id = FontId::monospace(13.0);
    let char_w = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let line_h = 18.0;

    let avail = ui.available_size();
    let cols = ((avail.x / char_w).floor().max(10.0)) as u16;
    let rows = ((avail.y / line_h).floor().max(4.0)) as u16;
    pty.resize(cols, rows);

    let (rect, response) = ui.allocate_exact_size(avail, Sense::click());
    if response.clicked() {
        *terminal_focused = true;
    }

    if *terminal_focused {
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            match ev {
                Event::Text(t) => pty.write_bytes(t.as_bytes()),
                Event::Paste(t) => pty.write_bytes(t.as_bytes()),
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if modifiers.ctrl {
                        match key {
                            Key::C => pty.write_bytes(&[3]),
                            Key::D => pty.write_bytes(&[4]),
                            Key::L => pty.write_bytes(&[12]),
                            Key::Z => pty.write_bytes(&[26]),
                            _ => {}
                        }
                    } else {
                        match key {
                            Key::Enter => pty.write_bytes(b"\r"),
                            Key::Backspace => pty.write_bytes(&[0x7f]),
                            Key::Tab => pty.write_bytes(b"\t"),
                            Key::Escape => pty.write_bytes(&[0x1b]),
                            Key::ArrowUp => pty.write_bytes(b"\x1b[A"),
                            Key::ArrowDown => pty.write_bytes(b"\x1b[B"),
                            Key::ArrowRight => pty.write_bytes(b"\x1b[C"),
                            Key::ArrowLeft => pty.write_bytes(b"\x1b[D"),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme.bg_panel);

    if let Ok(grid) = pty.grid.lock() {
        for (r, row) in grid.cells.iter().enumerate() {
            let y = rect.min.y + r as f32 * line_h;
            for (c, cell) in row.iter().enumerate() {
                let x = rect.min.x + c as f32 * char_w;
                if cell.bg != TermColor::Default {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(x, y), Vec2::new(char_w, line_h)),
                        0.0,
                        ansi_to_color(cell.bg, theme.bg_panel),
                    );
                }
                if cell.ch != ' ' {
                    painter.text(
                        Pos2::new(x, y + 1.0),
                        egui::Align2::LEFT_TOP,
                        cell.ch,
                        font_id.clone(),
                        ansi_to_color(cell.fg, theme.fg_text),
                    );
                }
            }
        }

        let cx = rect.min.x + grid.cursor_col as f32 * char_w;
        let cy = rect.min.y + grid.cursor_row as f32 * line_h;
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(char_w, line_h)),
            0.0,
            if *terminal_focused {
                theme.accent.linear_multiply(0.6)
            } else {
                theme.fg_muted.linear_multiply(0.35)
            },
        );
    }

    action
}
