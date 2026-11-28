use std::collections::HashMap;
use std::ops::Range;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, Event, FontId, Key, Modifiers, Pos2, Rect, Sense, Stroke,
    TextFormat, Vec2,
};
use unicode_width::UnicodeWidthChar;

use crate::core::buffer::Document;
use crate::core::editing::{self, MoveDir};
use crate::core::selection::Selection;
use crate::lsp::client::{DiagnosticItem, InlayHintItem};
use crate::syntax::ast::AstSelectionHistory;
use crate::syntax::symbols;
use crate::syntax::{HighlightSpan, SyntaxState};
use crate::theme::Theme;
use crate::workspace::git_diff::GitLineChange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeViewContextAction {
    OpenFind,
    OpenReplace,
    TriggerCompletion,
}

pub struct CodeViewOutput {
    pub text_changed: bool,
    pub request_definition: Option<(usize, usize)>,
    pub primary_cursor_screen_pos: Pos2,
    pub trigger_completion_char: bool,
    pub trigger_signature_char: bool,
    pub context_action: Option<CodeViewContextAction>,
    pub top_visible_line: f32,
}

pub struct CodeViewState {
    pub font_size: f32,
    pub ensure_cursor_visible: bool,
    pub scroll_to_line: Option<usize>,
    pub scroll_to_top_line: Option<f32>,
    pub scroll_salt: u64,
    dragging: bool,
    pub ast_selection: AstSelectionHistory,
    last_top_visible_line: usize,
    pub git_changes: HashMap<usize, GitLineChange>,
    pub inlay_hints: Vec<InlayHintItem>,
}

impl Default for CodeViewState {
    fn default() -> Self {
        Self {
            font_size: 14.5,
            ensure_cursor_visible: false,
            scroll_to_line: None,
            scroll_to_top_line: None,
            scroll_salt: 0,
            dragging: false,
            ast_selection: AstSelectionHistory::default(),
            last_top_visible_line: 0,
            git_changes: HashMap::new(),
            inlay_hints: Vec::new(),
        }
    }
}

/// Maps a 0-indexed document line into an Overview Ruler Y coordinate.
pub fn overview_marker_y(line: usize, total_lines: usize, ruler_top: f32, ruler_height: f32) -> f32 {
    if total_lines <= 1 || ruler_height <= 0.0 {
        return ruler_top;
    }
    let ratio = (line.min(total_lines - 1) as f32) / ((total_lines - 1) as f32);
    ruler_top + ratio * (ruler_height - 3.0).max(0.0)
}

fn visual_col_width(line_str: &str, char_col: usize) -> usize {
    line_str
        .chars()
        .take(char_col)
        .map(|c| if c == '\t' { 4 } else { c.width().unwrap_or(1) })
        .sum()
}

fn visual_col_to_char_col(line_str: &str, target_visual: usize) -> usize {
    let mut vis = 0usize;
    for (idx, c) in line_str.chars().enumerate() {
        let w = if c == '\t' { 4 } else { c.width().unwrap_or(1) };
        if vis + w > target_visual {
            return idx;
        }
        vis += w;
    }
    line_str.chars().count()
}

pub fn show_code_view(
    ui: &mut egui::Ui,
    doc: &mut Document,
    syntax: &mut SyntaxState,
    state: &mut CodeViewState,
    theme: &Theme,
    search_matches: &[Range<usize>],
    diagnostics: &[DiagnosticItem],
    has_focus: bool,
    clipboard: &mut Option<arboard::Clipboard>,
) -> CodeViewOutput {
    let mut output = CodeViewOutput {
        text_changed: false,
        request_definition: None,
        primary_cursor_screen_pos: ui.min_rect().min,
        trigger_completion_char: false,
        trigger_signature_char: false,
        context_action: None,
        top_visible_line: 0.0,
    };

    // Sync Tree-sitter incrementally before rendering
    let deltas = doc.drain_deltas();
    syntax.sync(&doc.rope, doc.language, doc.version, &deltas);

    let font_id = FontId::monospace(state.font_size);
    let char_width = ui.fonts(|f| f.glyph_width(&font_id, 'M'));
    let line_height = (state.font_size * 1.48).round();
    let total_lines = doc.len_lines().max(1);
    let gutter_digits = total_lines.to_string().len().max(3);
    let gutter_width = (gutter_digits as f32 + 2.2) * char_width;
    let text_left_pad = 10.0;

    // Handle keyboard input when editor has focus
    let before_version = doc.version;
    if has_focus {
        let page_lines = ((ui.available_height() / line_height) as usize).saturating_sub(2).max(5);
        handle_keyboard(
            ui,
            doc,
            syntax,
            &mut state.ast_selection,
            page_lines,
            clipboard,
            &mut output,
        );
    }
    if doc.version != before_version {
        output.text_changed = true;
        state.ensure_cursor_visible = true;
        state.ast_selection.clear();
        let new_deltas = doc.drain_deltas();
        syntax.sync(&doc.rope, doc.language, doc.version, &new_deltas);
    }

    // Sticky Scroll header: O(tree depth) query for up to 3 enclosing AST scopes
    let sticky_scopes = syntax
        .tree()
        .map(|tree| {
            symbols::enclosing_scopes_at_line(
                tree,
                &doc.rope,
                doc.language,
                state.last_top_visible_line,
                3,
            )
        })
        .unwrap_or_default();
    if !sticky_scopes.is_empty() && ui.available_height() > 140.0 {
        let mut jump_line: Option<usize> = None;
        egui::Frame::NONE
            .fill(theme.bg_panel)
            .stroke(Stroke::new(1.0_f32, theme.border))
            .inner_margin(egui::Margin::symmetric(0, 2))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for scope in &sticky_scopes {
                    let row_h = (line_height * 0.92).max(17.0);
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_h),
                        Sense::click(),
                    );
                    if row_resp.hovered() {
                        ui.painter().rect_filled(row_rect, 0.0, theme.bg_hover);
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    let line_no = format!("{}", scope.start_line + 1);
                    ui.painter().text(
                        Pos2::new(row_rect.min.x + gutter_width - char_width * 0.9, row_rect.center().y),
                        Align2::RIGHT_CENTER,
                        line_no,
                        font_id.clone(),
                        theme.fg_gutter,
                    );
                    let snippet = doc.line_string_without_newline(scope.start_line);
                    let indent_px = (scope.depth as f32 * 10.0).min(30.0);
                    ui.painter().text(
                        Pos2::new(row_rect.min.x + gutter_width + text_left_pad + indent_px, row_rect.center().y),
                        Align2::LEFT_CENTER,
                        snippet.trim(),
                        font_id.clone(),
                        theme.fg_text,
                    );
                    if row_resp.clicked() {
                        jump_line = Some(scope.start_line);
                    }
                }
            });
        if let Some(target) = jump_line {
            state.scroll_to_top_line = Some(target as f32);
        }
    }

    // Compute virtual canvas size (reserve 10px on right for Overview Ruler)
    let ruler_width = if ui.available_width() > 140.0 { 10.0 } else { 0.0 };
    let editor_avail_w = (ui.available_width() - ruler_width).max(60.0);
    let editor_avail_h = ui.available_height().max(40.0);
    let virtual_height = (total_lines as f32 * line_height + editor_avail_h * 0.5)
        .max(editor_avail_h);
    let virtual_width = editor_avail_w.max(gutter_width + text_left_pad + 160.0 * char_width);

    let mut scroll_area = egui::ScrollArea::both()
        .id_salt(("draco_code_scroll", doc.id, state.scroll_salt))
        .auto_shrink([false, false]);

    if let Some(top_line) = state.scroll_to_top_line.take() {
        let target_y = (top_line * line_height).max(0.0);
        scroll_area = scroll_area.vertical_scroll_offset(target_y);
    } else if let Some(target_line) = state.scroll_to_line.take() {
        let target_y = (target_line as f32 * line_height - editor_avail_h * 0.3).max(0.0);
        scroll_area = scroll_area.vertical_scroll_offset(target_y);
    }

    let bg_frame = egui::Frame::NONE.fill(theme.bg_editor);
    bg_frame.show(ui, |ui| {
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle {
            bar_width: 8.0,
            ..egui::style::ScrollStyle::solid()
        };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.allocate_ui_with_layout(
                Vec2::new(editor_avail_w, editor_avail_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    scroll_area.show_viewport(ui, |ui, viewport| {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(virtual_width, virtual_height),
                Sense::click_and_drag(),
            );

            if response.hovered()
                && ui.ctx().output(|o| o.cursor_icon == CursorIcon::Default)
            {
                ui.ctx().set_cursor_icon(CursorIcon::Text);
            }

            let origin = rect.min;
            let text_origin_x = origin.x + gutter_width + text_left_pad;

            // Mouse interactions
            handle_mouse(
                ui,
                &response,
                doc,
                state,
                origin,
                text_origin_x,
                char_width,
                line_height,
                &mut output,
            );

            // Visible line range + exact fractional top line for smooth Markdown scroll sync
            output.top_visible_line = (viewport.min.y / line_height).max(0.0);
            state.last_top_visible_line = output.top_visible_line.floor().max(0.0) as usize;
            let first_line = ((viewport.min.y / line_height).floor().max(0.0) as usize)
                .saturating_sub(1);
            let last_line = (((viewport.max.y / line_height).ceil().max(0.0) as usize) + 2)
                .min(total_lines.saturating_sub(1));

            // Keep cursor in view if requested
            if state.ensure_cursor_visible {
                state.ensure_cursor_visible = false;
                let (cline, _) = doc.char_to_line_col(doc.selections.primary().head);
                let cy = origin.y + cline as f32 * line_height;
                let cursor_rect = Rect::from_min_size(
                    Pos2::new(origin.x, cy - line_height),
                    Vec2::new(10.0, line_height * 3.0),
                );
                ui.scroll_to_rect(cursor_rect, None);
            }

            // Query Tree-sitter highlights ONLY for the visible byte range!
            let vis_start_byte = doc.rope.line_to_byte(first_line);
            let vis_end_byte = if last_line + 1 < doc.rope.len_lines() {
                doc.rope.line_to_byte(last_line + 1)
            } else {
                doc.rope.len_bytes()
            };
            let highlights = syntax.highlights_in_range(&doc.rope, vis_start_byte..vis_end_byte);

            let matching_bracket =
                editing::find_matching_bracket(&doc.rope, doc.selections.primary().head);
            let (primary_line, primary_col) = doc.char_to_line_col(doc.selections.primary().head);
            output.primary_cursor_screen_pos = Pos2::new(
                text_origin_x + primary_col as f32 * char_width,
                origin.y + (primary_line + 1) as f32 * line_height,
            );

            let painter = ui.painter_at(Rect::from_min_size(
                Pos2::new(origin.x + viewport.min.x, origin.y + viewport.min.y),
                viewport.size(),
            ));

            // Gutter background (pinned to viewport.min.x)
            let gutter_rect = Rect::from_min_size(
                Pos2::new(origin.x + viewport.min.x, origin.y + viewport.min.y),
                Vec2::new(gutter_width, viewport.height()),
            );

            // 1. Active line highlight
            let active_y = origin.y + primary_line as f32 * line_height;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(origin.x + gutter_width, active_y),
                    Vec2::new(virtual_width, line_height),
                ),
                0.0,
                theme.bg_active_line,
            );

            // 2. Search match highlights (visible lines only)
            for m in search_matches {
                let m_start_line = doc.rope.char_to_line(m.start.min(doc.len_chars()));
                let m_end_line = doc.rope.char_to_line(m.end.min(doc.len_chars()));
                if m_end_line < first_line || m_start_line > last_line {
                    continue;
                }
                paint_char_range_bg(
                    &painter,
                    doc,
                    m.start,
                    m.end,
                    first_line,
                    last_line,
                    origin.y,
                    text_origin_x,
                    char_width,
                    line_height,
                    theme.bg_search_match,
                );
            }

            // 3. Multi-selection highlights
            for sel in doc.selections.selections() {
                if sel.is_empty() {
                    continue;
                }
                paint_char_range_bg(
                    &painter,
                    doc,
                    sel.start(),
                    sel.end(),
                    first_line,
                    last_line,
                    origin.y,
                    text_origin_x,
                    char_width,
                    line_height,
                    theme.bg_selection,
                );
            }

            // 4. Matching bracket highlight
            if let Some((b1, b2)) = matching_bracket {
                for b in [b1, b2] {
                    let (bl, bc) = doc.char_to_line_col(b);
                    if bl >= first_line && bl <= last_line {
                        let r = Rect::from_min_size(
                            Pos2::new(
                                text_origin_x + bc as f32 * char_width,
                                origin.y + bl as f32 * line_height,
                            ),
                            Vec2::new(char_width, line_height),
                        );
                        painter.rect_filled(r, 2.0, theme.bg_bracket_match);
                    }
                }
            }

            // 5. Visible lines text with Tree-sitter syntax spans + indent guides
            for line_idx in first_line..=last_line {
                let line_str = doc.line_string_without_newline(line_idx);
                if line_str.is_empty() {
                    continue;
                }
                // Paint subtle 4-space vertical indent guides
                let leading_spaces = line_str.chars().take_while(|&c| c == ' ').count();
                let levels = leading_spaces / 4;
                for lvl in 1..=levels {
                    let gx = text_origin_x + (lvl * 4) as f32 * char_width;
                    let gy = origin.y + line_idx as f32 * line_height;
                    painter.line_segment(
                        [Pos2::new(gx, gy), Pos2::new(gx, gy + line_height)],
                        Stroke::new(1.0_f32, theme.indent_guide),
                    );
                }

                let line_start_byte = doc.rope.line_to_byte(line_idx);
                let line_end_byte = line_start_byte + line_str.len();
                let job = build_line_layout_job(
                    &line_str,
                    line_start_byte,
                    line_end_byte,
                    &highlights,
                    &font_id,
                    theme,
                );
                let galley = ui.fonts(|f| f.layout_job(job));
                let pos = Pos2::new(text_origin_x, origin.y + line_idx as f32 * line_height + 2.0);
                let line_end_x = text_origin_x + galley.size().x + 8.0;
                painter.galley(pos, galley, theme.fg_text);

                // Subtle inline LSP Inlay Hints (non-intrusive trailing pills on visible lines)
                let mut hint_x = line_end_x;
                for hint in state.inlay_hints.iter().filter(|h| h.line == line_idx) {
                    let hint_font = FontId::monospace((state.font_size - 2.0).max(10.0));
                    let h_galley =
                        ui.fonts(|f| f.layout_no_wrap(hint.label.clone(), hint_font, theme.fg_muted));
                    let pill_rect = Rect::from_min_size(
                        Pos2::new(hint_x, origin.y + line_idx as f32 * line_height + 2.0),
                        Vec2::new(h_galley.size().x + 8.0, (line_height - 4.0).max(12.0)),
                    );
                    painter.rect_filled(pill_rect, 3.0, theme.bg_panel);
                    painter.galley(
                        Pos2::new(pill_rect.min.x + 4.0, pill_rect.min.y + 1.0),
                        h_galley,
                        theme.fg_muted,
                    );
                    hint_x = pill_rect.max.x + 4.0;
                }
            }

            // 6. LSP Diagnostic underlines
            for diag in diagnostics {
                if diag.start_line < first_line || diag.start_line > last_line {
                    continue;
                }
                let color = match diag.severity {
                    1 => theme.diag_error,
                    2 => theme.diag_warning,
                    _ => theme.diag_info,
                };
                let start_x = text_origin_x + diag.start_col as f32 * char_width;
                let end_col = if diag.end_line == diag.start_line && diag.end_col > diag.start_col {
                    diag.end_col
                } else {
                    diag.start_col + 1
                };
                let end_x = text_origin_x + end_col as f32 * char_width;
                let y = origin.y + (diag.start_line + 1) as f32 * line_height - 2.0;
                painter.line_segment(
                    [Pos2::new(start_x, y), Pos2::new(end_x, y)],
                    Stroke::new(1.5_f32, color),
                );
            }

            // 7. Carets (multi-cursor)
            if has_focus {
                for sel in doc.selections.selections() {
                    let (cl, cc) = doc.char_to_line_col(sel.head);
                    if cl >= first_line && cl <= last_line {
                        let line_s = doc.line_string_without_newline(cl);
                        let vis_c = visual_col_width(&line_s, cc);
                        let cx = text_origin_x + vis_c as f32 * char_width;
                        let cy = origin.y + cl as f32 * line_height;
                        painter.line_segment(
                            [Pos2::new(cx, cy + 1.0), Pos2::new(cx, cy + line_height - 1.0)],
                            Stroke::new(2.0_f32, theme.cursor),
                        );
                    }
                }
            }

            // 8. Gutter (drawn on top so horizontal scroll never overlaps line numbers)
            painter.rect_filled(gutter_rect, 0.0, theme.bg_editor);
            painter.line_segment(
                [gutter_rect.right_top(), gutter_rect.right_bottom()],
                Stroke::new(1.0_f32, theme.border),
            );
            for line_idx in first_line..=last_line {
                let y = origin.y + line_idx as f32 * line_height + line_height * 0.5;
                let col = if line_idx == primary_line {
                    theme.fg_gutter_active
                } else {
                    theme.fg_gutter
                };
                // Diagnostic severity dot in gutter
                if let Some(diag) = diagnostics.iter().find(|d| d.start_line == line_idx) {
                    let dot_color = match diag.severity {
                        1 => theme.diag_error,
                        2 => theme.diag_warning,
                        _ => theme.diag_info,
                    };
                    painter.circle_filled(
                        Pos2::new(gutter_rect.left() + 6.0, y),
                        3.0,
                        dot_color,
                    );
                }
                painter.text(
                    Pos2::new(gutter_rect.right() - char_width * 0.8, y),
                    Align2::RIGHT_CENTER,
                    (line_idx + 1).to_string(),
                    font_id.clone(),
                    col,
                );
                // Git Gutter Indicator bar (Added = green, Modified = accent blue, Deleted = red)
                if let Some(change) = state.git_changes.get(&line_idx) {
                    let line_top = origin.y + line_idx as f32 * line_height;
                    match change {
                        GitLineChange::Added => {
                            painter.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(gutter_rect.right() - 3.5, line_top + 1.0),
                                    Vec2::new(3.0, line_height - 2.0),
                                ),
                                1.0,
                                Color32::from_rgb(106, 190, 96),
                            );
                        }
                        GitLineChange::Modified => {
                            painter.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(gutter_rect.right() - 3.5, line_top + 1.0),
                                    Vec2::new(3.0, line_height - 2.0),
                                ),
                                1.0,
                                theme.accent,
                            );
                        }
                        GitLineChange::Deleted => {
                            painter.rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(gutter_rect.right() - 5.0, line_top + line_height - 3.0),
                                    Vec2::new(5.0, 3.0),
                                ),
                                1.0,
                                theme.diag_error,
                            );
                        }
                    }
                }
            }

            // Subtle welcome hint on completely empty untitled buffer
            if doc.path.is_none() && doc.len_chars() == 0 {
                let center = Pos2::new(
                    origin.x + gutter_width + (viewport.width() - gutter_width) * 0.5,
                    origin.y + viewport.height() * 0.36,
                );
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    "Draco",
                    FontId::proportional(28.0),
                    theme.fg_gutter.linear_multiply(0.55),
                );
                painter.text(
                    center + Vec2::new(0.0, 32.0),
                    Align2::CENTER_CENTER,
                    "Ctrl+P  Quick Open   •   Ctrl+O  Open File   •   Ctrl+`  Terminal",
                    FontId::proportional(12.5),
                    theme.fg_gutter.linear_multiply(0.75),
                );
            }

            // Right-click Context Menu on code canvas
            let mut ctx_changed = false;
            response.context_menu(|ui| {
                ui.set_min_width(195.0);
                if ui.button("Cut               Ctrl+X").clicked() {
                    let s = editing::selected_text(doc);
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(s);
                    } else {
                        ui.ctx().copy_text(s);
                    }
                    editing::backspace(doc);
                    ctx_changed = true;
                    ui.close_menu();
                }
                if ui.button("Copy              Ctrl+C").clicked() {
                    let s = editing::selected_text(doc);
                    if let Some(cb) = clipboard.as_mut() {
                        let _ = cb.set_text(s);
                    } else {
                        ui.ctx().copy_text(s);
                    }
                    ui.close_menu();
                }
                if ui.button("Paste             Ctrl+V").clicked() {
                    if let Some(cb) = clipboard.as_mut() {
                        if let Ok(text) = cb.get_text() {
                            editing::insert_text(doc, &text.replace("\r\n", "\n"));
                            ctx_changed = true;
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Select All        Ctrl+A").clicked() {
                    editing::select_all(doc);
                    ui.close_menu();
                }
                if ui.button("Add Next Match    Ctrl+D").clicked() {
                    editing::select_next_occurrence(doc);
                    ui.close_menu();
                }
                if ui.button("Toggle Comment    Ctrl+/").clicked() {
                    editing::toggle_comment(doc);
                    ctx_changed = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Go to Definition  F12").clicked() {
                    let (l, c) = doc.char_to_line_col(doc.selections.primary().head);
                    output.request_definition = Some((l, c));
                    ui.close_menu();
                }
                if ui.button("Autocomplete      Ctrl+Space").clicked() {
                    output.context_action = Some(CodeViewContextAction::TriggerCompletion);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Find              Ctrl+F").clicked() {
                    output.context_action = Some(CodeViewContextAction::OpenFind);
                    ui.close_menu();
                }
                if ui.button("Replace           Ctrl+H").clicked() {
                    output.context_action = Some(CodeViewContextAction::OpenReplace);
                    ui.close_menu();
                }
            });
            if ctx_changed {
                output.text_changed = true;
            }

            // 9. Hover diagnostic tooltip
            if let Some(hover_pos) = response.hover_pos() {
                if hover_pos.x > gutter_rect.right() {
                    let h_line = ((hover_pos.y - origin.y) / line_height).floor().max(0.0) as usize;
                    let h_col = ((hover_pos.x - text_origin_x) / char_width).floor().max(0.0) as usize;
                    if let Some(diag) = diagnostics.iter().find(|d| {
                        d.start_line == h_line
                            && h_col >= d.start_col
                            && h_col <= d.end_col.max(d.start_col + 2)
                    }) {
                        egui::show_tooltip_at_pointer(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new("draco_diag_tooltip"),
                            |ui| {
                                ui.label(&diag.message);
                            },
                        );
                    }
                }
            }
                    });
                },
            );

            // 10. Overview Ruler (diagnostics, find matches, primary cursor; click to jump)
            if ruler_width > 0.0 {
                let (ruler_rect, ruler_resp) = ui.allocate_exact_size(
                    Vec2::new(ruler_width, editor_avail_h),
                    Sense::click_and_drag(),
                );
                let rp = ui.painter_at(ruler_rect);
                rp.rect_filled(ruler_rect, 0.0, theme.bg_panel);
                rp.line_segment(
                    [ruler_rect.left_top(), ruler_rect.left_bottom()],
                    Stroke::new(1.0_f32, theme.border),
                );

                // Search match ticks (sampled to at most 400 for O(1) perf on huge files)
                if !search_matches.is_empty() {
                    let step = (search_matches.len() / 400).max(1);
                    let match_color = theme.accent.linear_multiply(0.85);
                    for m in search_matches.iter().step_by(step) {
                        let line = doc.rope.char_to_line(m.start.min(doc.len_chars()));
                        let y = overview_marker_y(line, total_lines, ruler_rect.min.y, ruler_rect.height());
                        rp.rect_filled(
                            Rect::from_min_size(Pos2::new(ruler_rect.min.x + 2.0, y), Vec2::new(ruler_width - 3.0, 2.0)),
                            0.5,
                            match_color,
                        );
                    }
                }

                // LSP diagnostic ticks (warnings then errors on top)
                for diag in diagnostics.iter().filter(|d| d.severity != 1) {
                    let y = overview_marker_y(diag.start_line, total_lines, ruler_rect.min.y, ruler_rect.height());
                    rp.rect_filled(
                        Rect::from_min_size(Pos2::new(ruler_rect.min.x + 1.5, y), Vec2::new(ruler_width - 2.5, 3.0)),
                        0.5,
                        theme.diag_warning,
                    );
                }
                for diag in diagnostics.iter().filter(|d| d.severity == 1) {
                    let y = overview_marker_y(diag.start_line, total_lines, ruler_rect.min.y, ruler_rect.height());
                    rp.rect_filled(
                        Rect::from_min_size(Pos2::new(ruler_rect.min.x + 1.5, y), Vec2::new(ruler_width - 2.5, 3.0)),
                        0.5,
                        theme.diag_error,
                    );
                }

                // Primary cursor tick
                let cur_line = doc.rope.char_to_line(doc.selections.primary().head.min(doc.len_chars()));
                let cur_y = overview_marker_y(cur_line, total_lines, ruler_rect.min.y, ruler_rect.height());
                rp.rect_filled(
                    Rect::from_min_size(Pos2::new(ruler_rect.min.x + 1.0, cur_y), Vec2::new(ruler_width - 2.0, 2.0)),
                    0.5,
                    theme.fg_text,
                );

                if ruler_resp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if (ruler_resp.clicked() || ruler_resp.dragged()) && ruler_rect.height() > 1.0 {
                    if let Some(pos) = ruler_resp.interact_pointer_pos() {
                        let ratio = ((pos.y - ruler_rect.min.y) / ruler_rect.height()).clamp(0.0, 1.0);
                        let target = (ratio * (total_lines.saturating_sub(1)) as f32).round() as usize;
                        state.scroll_to_line = Some(target);
                    }
                }
            }
        });
    });

    output
}

fn build_line_layout_job(
    line_str: &str,
    line_start_byte: usize,
    line_end_byte: usize,
    highlights: &[HighlightSpan],
    font_id: &FontId,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mut pos = line_start_byte;

    // Collect spans overlapping this line
    let mut line_spans: Vec<(usize, usize, Color32)> = Vec::new();
    for sp in highlights {
        if sp.byte_range.end <= line_start_byte || sp.byte_range.start >= line_end_byte {
            continue;
        }
        let s = sp.byte_range.start.max(line_start_byte);
        let e = sp.byte_range.end.min(line_end_byte);
        if s < e {
            line_spans.push((s, e, theme.token_color(sp.kind)));
        }
    }
    line_spans.sort_by_key(|(s, e, _)| (*s, *e));

    for (s, e, color) in line_spans {
        if s > pos {
            let rel_s = pos - line_start_byte;
            let rel_e = s - line_start_byte;
            if line_str.is_char_boundary(rel_s) && line_str.is_char_boundary(rel_e) {
                job.append(
                    &line_str[rel_s..rel_e],
                    0.0,
                    TextFormat::simple(font_id.clone(), theme.fg_text),
                );
            }
            pos = s;
        }
        if e > pos {
            let rel_s = pos - line_start_byte;
            let rel_e = e - line_start_byte;
            if line_str.is_char_boundary(rel_s) && line_str.is_char_boundary(rel_e) {
                job.append(
                    &line_str[rel_s..rel_e],
                    0.0,
                    TextFormat::simple(font_id.clone(), color),
                );
            }
            pos = e;
        }
    }

    if pos < line_end_byte {
        let rel_s = pos - line_start_byte;
        if line_str.is_char_boundary(rel_s) {
            job.append(
                &line_str[rel_s..],
                0.0,
                TextFormat::simple(font_id.clone(), theme.fg_text),
            );
        }
    }

    job
}

#[allow(clippy::too_many_arguments)]
fn paint_char_range_bg(
    painter: &egui::Painter,
    doc: &Document,
    start_char: usize,
    end_char: usize,
    first_line: usize,
    last_line: usize,
    origin_y: f32,
    text_origin_x: f32,
    char_width: f32,
    line_height: f32,
    color: Color32,
) {
    let (sl, sc) = doc.char_to_line_col(start_char);
    let (el, ec) = doc.char_to_line_col(end_char);
    if el < first_line || sl > last_line {
        return;
    }
    for line in sl.max(first_line)..=el.min(last_line) {
        let line_s = doc.line_string_without_newline(line);
        let c0 = if line == sl {
            visual_col_width(&line_s, sc)
        } else {
            0
        };
        let c1 = if line == el {
            visual_col_width(&line_s, ec)
        } else {
            visual_col_width(&line_s, line_s.chars().count()) + 1
        };
        if c1 > c0 {
            let rect = Rect::from_min_size(
                Pos2::new(
                    text_origin_x + c0 as f32 * char_width,
                    origin_y + line as f32 * line_height,
                ),
                Vec2::new((c1 - c0) as f32 * char_width, line_height),
            );
            painter.rect_filled(rect, 2.0, color);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_mouse(
    ui: &egui::Ui,
    response: &egui::Response,
    doc: &mut Document,
    state: &mut CodeViewState,
    origin: Pos2,
    text_origin_x: f32,
    char_width: f32,
    line_height: f32,
    output: &mut CodeViewOutput,
) {
    let pos_to_char = |pos: Pos2, doc: &Document| -> (usize, usize, usize) {
        let line = ((pos.y - origin.y) / line_height).floor().max(0.0) as usize;
        let line = line.min(doc.len_lines().saturating_sub(1));
        let vis_col = (((pos.x - text_origin_x) / char_width) + 0.5)
            .floor()
            .max(0.0) as usize;
        let line_s = doc.line_string_without_newline(line);
        let col = visual_col_to_char_col(&line_s, vis_col);
        let ch = doc.line_col_to_char(line, col);
        (line, col, ch)
    };

    if response.double_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (_, _, ch) = pos_to_char(pos, doc);
            let r = editing::word_range_at(&doc.rope, ch);
            doc.selections
                .set_single(Selection::new(r.start, r.end), doc.len_chars());
            return;
        }
    }

    if response.triple_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (line, _, _) = pos_to_char(pos, doc);
            let start = doc.rope.line_to_char(line);
            let end = start + doc.line_len_chars_without_newline(line);
            doc.selections
                .set_single(Selection::new(start, end), doc.len_chars());
            return;
        }
    }

    let mods = ui.input(|i| i.modifiers);
    if response.is_pointer_button_down_on() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (line, col, ch) = pos_to_char(pos, doc);
            if !state.dragging {
                state.dragging = true;
                if mods.command {
                    output.request_definition = Some((line, col));
                } else if mods.alt {
                    doc.selections.add(Selection::caret(ch), doc.len_chars());
                } else if mods.shift {
                    let anchor = doc.selections.primary().anchor;
                    doc.selections
                        .set_single(Selection::new(anchor, ch), doc.len_chars());
                } else {
                    doc.selections
                        .set_single(Selection::caret(ch), doc.len_chars());
                }
                doc.preferred_col = None;
            } else {
                let anchor = doc.selections.primary().anchor;
                doc.selections
                    .set_single(Selection::new(anchor, ch), doc.len_chars());
            }
        }
    } else {
        state.dragging = false;
    }
}

fn handle_keyboard(
    ui: &egui::Ui,
    doc: &mut Document,
    syntax: &SyntaxState,
    ast_selection: &mut AstSelectionHistory,
    page_lines: usize,
    clipboard: &mut Option<arboard::Clipboard>,
    output: &mut CodeViewOutput,
) {
    let events = ui.input(|i| i.events.clone());
    for ev in events {
        match ev {
            Event::Text(text) => {
                if text.chars().all(|c| !c.is_control()) {
                    if text == "." || text == ":" {
                        output.trigger_completion_char = true;
                    }
                    if text == "(" || text == "," {
                        output.trigger_signature_char = true;
                    }
                    editing::insert_text(doc, &text);
                }
            }
            Event::Paste(text) => {
                let cleaned = text.replace("\r\n", "\n");
                editing::insert_text(doc, &cleaned);
            }
            Event::Copy => {
                let s = editing::selected_text(doc);
                if let Some(cb) = clipboard.as_mut() {
                    let _ = cb.set_text(s);
                } else {
                    ui.ctx().copy_text(s);
                }
            }
            Event::Cut => {
                let s = editing::selected_text(doc);
                if let Some(cb) = clipboard.as_mut() {
                    let _ = cb.set_text(s);
                } else {
                    ui.ctx().copy_text(s);
                }
                editing::backspace(doc);
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                handle_key_press(key, modifiers, doc, syntax, ast_selection, page_lines, clipboard);
            }
            _ => {}
        }
    }
}

fn handle_key_press(
    key: Key,
    m: Modifiers,
    doc: &mut Document,
    syntax: &SyntaxState,
    ast_selection: &mut AstSelectionHistory,
    page_lines: usize,
    clipboard: &mut Option<arboard::Clipboard>,
) {
    // Ctrl/Cmd shortcuts
    if m.command && !m.alt {
        match key {
            Key::Z if m.shift => {
                doc.redo();
                return;
            }
            Key::Z => {
                doc.undo();
                return;
            }
            Key::Y => {
                doc.redo();
                return;
            }
            Key::A => {
                editing::select_all(doc);
                return;
            }
            Key::D => {
                editing::select_next_occurrence(doc);
                return;
            }
            Key::Slash => {
                editing::toggle_comment(doc);
                return;
            }
            Key::V => {
                if let Some(cb) = clipboard.as_mut() {
                    if let Ok(text) = cb.get_text() {
                        editing::insert_text(doc, &text.replace("\r\n", "\n"));
                    }
                }
                return;
            }
            Key::ArrowLeft => {
                editing::move_cursors(doc, MoveDir::WordLeft, m.shift);
                return;
            }
            Key::ArrowRight => {
                editing::move_cursors(doc, MoveDir::WordRight, m.shift);
                return;
            }
            Key::Home => {
                editing::move_cursors(doc, MoveDir::DocStart, m.shift);
                return;
            }
            Key::End => {
                editing::move_cursors(doc, MoveDir::DocEnd, m.shift);
                return;
            }
            _ => {}
        }
    }

    // Ctrl+Alt+Up/Down -> multi-cursor vertical
    if m.command && m.alt {
        match key {
            Key::ArrowUp => {
                editing::add_cursor_vertical(doc, false);
                return;
            }
            Key::ArrowDown => {
                editing::add_cursor_vertical(doc, true);
                return;
            }
            _ => {}
        }
    }

    // Alt+Up/Down -> move line; Shift+Alt+Down -> duplicate line; Shift+Alt+Right/Left -> AST Expand/Shrink Selection
    if m.alt && !m.command {
        match key {
            Key::ArrowRight if m.shift => {
                if let Some(tree) = syntax.tree() {
                    let cur = doc.selections.primary();
                    if let Some(expanded) = ast_selection.expand(tree, &doc.rope, cur) {
                        doc.selections.set_single(expanded, doc.len_chars());
                    }
                }
                return;
            }
            Key::ArrowLeft if m.shift => {
                let cur = doc.selections.primary();
                if let Some(shrunk) = ast_selection.shrink(cur) {
                    doc.selections.set_single(shrunk, doc.len_chars());
                }
                return;
            }
            Key::ArrowDown if m.shift => {
                editing::duplicate_lines(doc);
                return;
            }
            Key::ArrowUp => {
                editing::move_lines(doc, true);
                return;
            }
            Key::ArrowDown => {
                editing::move_lines(doc, false);
                return;
            }
            _ => {}
        }
    }

    if m.command {
        return;
    }

    match key {
        Key::Enter => editing::insert_newline(doc),
        Key::Backspace => editing::backspace(doc),
        Key::Delete => editing::delete_forward(doc),
        Key::Tab if m.shift => editing::outdent(doc),
        Key::Tab => editing::indent(doc),
        Key::Escape => doc.selections.collapse_to_primary(),
        Key::ArrowLeft => editing::move_cursors(doc, MoveDir::Left, m.shift),
        Key::ArrowRight => editing::move_cursors(doc, MoveDir::Right, m.shift),
        Key::ArrowUp => editing::move_cursors(doc, MoveDir::Up, m.shift),
        Key::ArrowDown => editing::move_cursors(doc, MoveDir::Down, m.shift),
        Key::Home => editing::move_cursors(doc, MoveDir::LineStart, m.shift),
        Key::End => editing::move_cursors(doc, MoveDir::LineEnd, m.shift),
        Key::PageUp => editing::move_cursors(doc, MoveDir::PageUp(page_lines), m.shift),
        Key::PageDown => editing::move_cursors(doc, MoveDir::PageDown(page_lines), m.shift),
        _ => {}
    }
}
