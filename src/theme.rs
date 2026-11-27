use eframe::egui::Color32;

use crate::syntax::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub kind: ThemeKind,
    pub bg_editor: Color32,
    pub bg_panel: Color32,
    pub bg_tab_bar: Color32,
    pub bg_tab_active: Color32,
    pub bg_tab_inactive: Color32,
    pub bg_status: Color32,
    pub bg_active_line: Color32,
    pub bg_selection: Color32,
    pub bg_search_match: Color32,
    pub bg_bracket_match: Color32,
    pub bg_popup: Color32,
    pub bg_popup_selected: Color32,
    pub bg_hover: Color32,
    pub indent_guide: Color32,
    pub border: Color32,
    pub fg_text: Color32,
    pub fg_muted: Color32,
    pub fg_gutter: Color32,
    pub fg_gutter_active: Color32,
    pub cursor: Color32,
    pub accent: Color32,
    pub diag_error: Color32,
    pub diag_warning: Color32,
    pub diag_info: Color32,

    // Syntax token colors
    pub syn_comment: Color32,
    pub syn_keyword: Color32,
    pub syn_string: Color32,
    pub syn_number: Color32,
    pub syn_constant: Color32,
    pub syn_type: Color32,
    pub syn_function: Color32,
    pub syn_macro: Color32,
    pub syn_variable: Color32,
    pub syn_property: Color32,
    pub syn_operator: Color32,
    pub syn_punctuation: Color32,
    pub syn_attribute: Color32,
    pub syn_heading: Color32,
}

impl Theme {
    /// Tokyo Night inspired dark theme.
    pub fn dark() -> Self {
        Self {
            kind: ThemeKind::Dark,
            bg_editor: Color32::from_rgb(26, 27, 38),
            bg_panel: Color32::from_rgb(22, 22, 30),
            bg_tab_bar: Color32::from_rgb(20, 20, 28),
            bg_tab_active: Color32::from_rgb(26, 27, 38),
            bg_tab_inactive: Color32::from_rgb(20, 20, 28),
            bg_status: Color32::from_rgb(18, 19, 26),
            bg_active_line: Color32::from_rgb(35, 37, 52),
            bg_selection: Color32::from_rgba_unmultiplied(61, 89, 161, 120),
            bg_search_match: Color32::from_rgba_unmultiplied(224, 175, 104, 80),
            bg_bracket_match: Color32::from_rgba_unmultiplied(122, 162, 247, 75),
            bg_popup: Color32::from_rgb(30, 32, 46),
            bg_popup_selected: Color32::from_rgb(47, 53, 78),
            bg_hover: Color32::from_rgb(36, 39, 56),
            indent_guide: Color32::from_rgba_unmultiplied(86, 95, 137, 45),
            border: Color32::from_rgb(45, 48, 66),
            fg_text: Color32::from_rgb(192, 202, 245),
            fg_muted: Color32::from_rgb(120, 127, 160),
            fg_gutter: Color32::from_rgb(86, 95, 137),
            fg_gutter_active: Color32::from_rgb(192, 202, 245),
            cursor: Color32::from_rgb(122, 162, 247),
            accent: Color32::from_rgb(122, 162, 247),
            diag_error: Color32::from_rgb(247, 118, 142),
            diag_warning: Color32::from_rgb(224, 175, 104),
            diag_info: Color32::from_rgb(125, 207, 255),

            syn_comment: Color32::from_rgb(92, 103, 140),
            syn_keyword: Color32::from_rgb(187, 154, 247),
            syn_string: Color32::from_rgb(158, 206, 106),
            syn_number: Color32::from_rgb(255, 158, 100),
            syn_constant: Color32::from_rgb(255, 158, 100),
            syn_type: Color32::from_rgb(42, 195, 222),
            syn_function: Color32::from_rgb(122, 162, 247),
            syn_macro: Color32::from_rgb(125, 207, 255),
            syn_variable: Color32::from_rgb(192, 202, 245),
            syn_property: Color32::from_rgb(115, 218, 202),
            syn_operator: Color32::from_rgb(137, 221, 255),
            syn_punctuation: Color32::from_rgb(148, 158, 196),
            syn_attribute: Color32::from_rgb(224, 175, 104),
            syn_heading: Color32::from_rgb(122, 162, 247),
        }
    }

    pub fn light() -> Self {
        Self {
            kind: ThemeKind::Light,
            bg_editor: Color32::from_rgb(248, 249, 252),
            bg_panel: Color32::from_rgb(238, 240, 245),
            bg_tab_bar: Color32::from_rgb(230, 232, 240),
            bg_tab_active: Color32::from_rgb(248, 249, 252),
            bg_tab_inactive: Color32::from_rgb(230, 232, 240),
            bg_status: Color32::from_rgb(226, 229, 238),
            bg_active_line: Color32::from_rgb(236, 240, 249),
            bg_selection: Color32::from_rgba_unmultiplied(64, 112, 214, 70),
            bg_search_match: Color32::from_rgba_unmultiplied(212, 146, 32, 70),
            bg_bracket_match: Color32::from_rgba_unmultiplied(64, 112, 214, 60),
            bg_popup: Color32::from_rgb(252, 252, 255),
            bg_popup_selected: Color32::from_rgb(218, 228, 248),
            bg_hover: Color32::from_rgb(230, 234, 246),
            indent_guide: Color32::from_rgba_unmultiplied(150, 156, 180, 60),
            border: Color32::from_rgb(205, 210, 224),
            fg_text: Color32::from_rgb(44, 48, 66),
            fg_muted: Color32::from_rgb(115, 122, 148),
            fg_gutter: Color32::from_rgb(150, 156, 180),
            fg_gutter_active: Color32::from_rgb(44, 48, 66),
            cursor: Color32::from_rgb(46, 98, 214),
            accent: Color32::from_rgb(46, 98, 214),
            diag_error: Color32::from_rgb(210, 48, 72),
            diag_warning: Color32::from_rgb(184, 122, 16),
            diag_info: Color32::from_rgb(26, 124, 198),

            syn_comment: Color32::from_rgb(132, 140, 166),
            syn_keyword: Color32::from_rgb(138, 64, 200),
            syn_string: Color32::from_rgb(66, 134, 44),
            syn_number: Color32::from_rgb(188, 92, 22),
            syn_constant: Color32::from_rgb(188, 92, 22),
            syn_type: Color32::from_rgb(16, 122, 148),
            syn_function: Color32::from_rgb(42, 96, 204),
            syn_macro: Color32::from_rgb(26, 124, 198),
            syn_variable: Color32::from_rgb(44, 48, 66),
            syn_property: Color32::from_rgb(24, 128, 114),
            syn_operator: Color32::from_rgb(42, 118, 158),
            syn_punctuation: Color32::from_rgb(98, 106, 132),
            syn_attribute: Color32::from_rgb(166, 114, 24),
            syn_heading: Color32::from_rgb(42, 96, 204),
        }
    }

    pub fn token_color(&self, kind: TokenKind) -> Color32 {
        match kind {
            TokenKind::Plain => self.fg_text,
            TokenKind::Comment => self.syn_comment,
            TokenKind::Keyword => self.syn_keyword,
            TokenKind::String => self.syn_string,
            TokenKind::Number => self.syn_number,
            TokenKind::Constant => self.syn_constant,
            TokenKind::Type => self.syn_type,
            TokenKind::Function => self.syn_function,
            TokenKind::Macro => self.syn_macro,
            TokenKind::Variable => self.syn_variable,
            TokenKind::Property => self.syn_property,
            TokenKind::Operator => self.syn_operator,
            TokenKind::Punctuation => self.syn_punctuation,
            TokenKind::Attribute => self.syn_attribute,
            TokenKind::Heading => self.syn_heading,
        }
    }
}
