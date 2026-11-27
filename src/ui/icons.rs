use std::path::Path;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIconKind {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Json,
    Toml,
    C,
    Markdown,
    Php,
    Yaml,
    DotEnv,
    Conf,
    Shell,
    Docker,
    Git,
    Generic,
}

impl FileIconKind {
    pub fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == ".gitignore" || name == ".gitattributes" {
            return Self::Git;
        }
        if name == ".env" || name.starts_with(".env.") {
            return Self::DotEnv;
        }
        if name == "dockerfile" || name.starts_with("dockerfile.") {
            return Self::Docker;
        }
        if matches!(name.as_str(), ".editorconfig" | ".gitconfig" | ".npmrc") {
            return Self::Conf;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("rs") => Self::Rust,
            Some("py" | "pyi") => Self::Python,
            Some("js" | "mjs" | "cjs" | "jsx") => Self::JavaScript,
            Some("ts" | "mts" | "cts" | "tsx") => Self::TypeScript,
            Some("json" | "jsonc") => Self::Json,
            Some("toml") => Self::Toml,
            Some("yml" | "yaml") => Self::Yaml,
            Some("env") => Self::DotEnv,
            Some("conf" | "ini" | "cfg" | "cnf" | "properties") => Self::Conf,
            Some("sh" | "bash" | "zsh") => Self::Shell,
            Some("dockerfile") => Self::Docker,
            Some("c" | "h" | "cpp" | "cc" | "hpp") => Self::C,
            Some("md" | "markdown") => Self::Markdown,
            Some("php" | "phtml") => Self::Php,
            _ => Self::Generic,
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::Rust => Color32::from_rgb(240, 126, 82),
            Self::Python => Color32::from_rgb(92, 168, 240),
            Self::JavaScript => Color32::from_rgb(240, 212, 88),
            Self::TypeScript => Color32::from_rgb(76, 156, 242),
            Self::Json => Color32::from_rgb(224, 175, 104),
            Self::Toml => Color32::from_rgb(187, 154, 247),
            Self::Yaml => Color32::from_rgb(247, 118, 142),
            Self::DotEnv => Color32::from_rgb(236, 200, 82),
            Self::Conf => Color32::from_rgb(152, 172, 224),
            Self::Shell => Color32::from_rgb(158, 206, 106),
            Self::Docker => Color32::from_rgb(56, 176, 240),
            Self::C => Color32::from_rgb(112, 168, 244),
            Self::Markdown => Color32::from_rgb(115, 218, 202),
            Self::Php => Color32::from_rgb(142, 152, 224),
            Self::Git => Color32::from_rgb(240, 108, 92),
            Self::Generic => Color32::from_rgb(148, 158, 196),
        }
    }

    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::Rust => Some("rs"),
            Self::Python => Some("py"),
            Self::JavaScript => Some("js"),
            Self::TypeScript => Some("ts"),
            Self::Json => Some("{}"),
            Self::Toml => Some("⚙"),
            Self::Yaml => Some("yml"),
            Self::DotEnv => Some("env"),
            Self::Conf => Some("cfg"),
            Self::Shell => Some("sh"),
            Self::Docker => Some("dk"),
            Self::C => Some("C"),
            Self::Markdown => Some("M"),
            Self::Php => Some("php"),
            Self::Git => Some("G"),
            Self::Generic => None,
        }
    }
}

/// Paints a crisp 16x16 vector chevron pointing right (collapsed) or down (expanded).
pub fn paint_chevron(painter: &egui::Painter, rect: Rect, expanded: bool, color: Color32) {
    let c = rect.center();
    let s = 3.2_f32;
    let stroke = Stroke::new(1.4_f32, color);
    if expanded {
        painter.line_segment([Pos2::new(c.x - s, c.y - s * 0.45), Pos2::new(c.x, c.y + s * 0.6)], stroke);
        painter.line_segment([Pos2::new(c.x, c.y + s * 0.6), Pos2::new(c.x + s, c.y - s * 0.45)], stroke);
    } else {
        painter.line_segment([Pos2::new(c.x - s * 0.45, c.y - s), Pos2::new(c.x + s * 0.6, c.y)], stroke);
        painter.line_segment([Pos2::new(c.x + s * 0.6, c.y), Pos2::new(c.x - s * 0.45, c.y + s)], stroke);
    }
}

/// Paints a crisp vector folder icon (closed or open).
pub fn paint_folder_icon(painter: &egui::Painter, rect: Rect, expanded: bool, theme: &Theme) {
    let color = if expanded {
        Color32::from_rgb(224, 175, 104)
    } else {
        Color32::from_rgb(186, 152, 102)
    };
    let r = rect.shrink2(Vec2::new(1.5, 2.5));
    // Folder tab
    let tab = Rect::from_min_size(r.min, Vec2::new(r.width() * 0.44, 3.0));
    painter.rect_filled(tab, 1.0, color);
    // Folder body
    let body = Rect::from_min_max(Pos2::new(r.min.x, r.min.y + 2.2), r.max);
    if expanded {
        painter.rect_filled(body, 2.0, color.linear_multiply(0.85));
        painter.rect_stroke(body, 2.0, Stroke::new(1.0_f32, color), egui::StrokeKind::Inside);
    } else {
        painter.rect_filled(body, 2.0, color);
    }
    let _ = theme;
}

/// Paints a crisp color-coded file icon badge for the given path.
pub fn paint_file_icon(painter: &egui::Painter, rect: Rect, path: &Path, theme: &Theme) {
    let kind = FileIconKind::from_path(path);
    let color = kind.color();
    let r = rect.shrink2(Vec2::new(1.0, 1.5));

    if let Some(badge) = kind.badge() {
        let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 34);
        painter.rect_filled(r, 3.0, bg);
        painter.rect_stroke(
            r,
            3.0,
            Stroke::new(0.8_f32, color.linear_multiply(0.75)),
            egui::StrokeKind::Inside,
        );
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            badge,
            FontId::monospace(8.5),
            color,
        );
    } else {
        // Generic document outline with folded corner
        let doc = r.shrink2(Vec2::new(1.5, 0.5));
        let stroke = Stroke::new(1.1_f32, theme.fg_muted);
        painter.rect_stroke(doc, 1.5, stroke, egui::StrokeKind::Inside);
        let y1 = doc.min.y + doc.height() * 0.38;
        let y2 = doc.min.y + doc.height() * 0.64;
        painter.line_segment(
            [Pos2::new(doc.min.x + 2.5, y1), Pos2::new(doc.max.x - 2.5, y1)],
            Stroke::new(1.0_f32, theme.fg_muted),
        );
        painter.line_segment(
            [Pos2::new(doc.min.x + 2.5, y2), Pos2::new(doc.max.x - 3.5, y2)],
            Stroke::new(1.0_f32, theme.fg_muted),
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ActivityIcon {
    Files,
    Search,
    QuickOpen,
    Terminal,
    Settings,
    Theme,
}

/// Renders a crisp 32x32 Activity Bar icon button with active indicator bar.
pub fn activity_button(
    ui: &mut egui::Ui,
    icon: ActivityIcon,
    active: bool,
    tooltip: &str,
    theme: &Theme,
) -> egui::Response {
    let size = Vec2::new(34.0, 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();

    let painter = ui.painter();
    if active {
        painter.rect_filled(rect, 6.0, theme.bg_popup_selected.linear_multiply(0.65));
        // Left accent bar
        let bar = Rect::from_min_size(
            Pos2::new(rect.min.x - 3.0, rect.min.y + 6.0),
            Vec2::new(2.5, rect.height() - 12.0),
        );
        painter.rect_filled(bar, 1.5, theme.accent);
    } else if hovered {
        painter.rect_filled(rect, 6.0, theme.bg_active_line);
    }

    let color = if active {
        theme.accent
    } else if hovered {
        theme.fg_text
    } else {
        theme.fg_muted
    };
    let stroke = Stroke::new(1.5_f32, color);
    let c = rect.center();

    match icon {
        ActivityIcon::Files => {
            let back = Rect::from_center_size(c + Vec2::new(-2.0, -1.5), Vec2::new(11.0, 13.0));
            let front = Rect::from_center_size(c + Vec2::new(2.0, 1.5), Vec2::new(11.0, 13.0));
            painter.rect_stroke(back, 1.5, Stroke::new(1.2_f32, color.linear_multiply(0.65)), egui::StrokeKind::Inside);
            painter.rect_filled(front, 1.5, theme.bg_panel);
            painter.rect_stroke(front, 1.5, stroke, egui::StrokeKind::Inside);
        }
        ActivityIcon::Search => {
            let lens = c + Vec2::new(-2.0, -2.0);
            painter.circle_stroke(lens, 5.2, stroke);
            painter.line_segment(
                [lens + Vec2::new(3.8, 3.8), c + Vec2::new(6.8, 6.8)],
                Stroke::new(1.8_f32, color),
            );
        }
        ActivityIcon::QuickOpen => {
            // Lightning / command bolt
            let pts = [
                c + Vec2::new(1.5, -7.5),
                c + Vec2::new(-4.5, 0.5),
                c + Vec2::new(0.0, 0.5),
                c + Vec2::new(-1.5, 7.5),
                c + Vec2::new(4.5, -0.5),
                c + Vec2::new(0.0, -0.5),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            painter.line_segment([pts[5], pts[0]], stroke);
        }
        ActivityIcon::Terminal => {
            let box_r = Rect::from_center_size(c, Vec2::new(16.0, 13.0));
            painter.rect_stroke(box_r, 2.0, stroke, egui::StrokeKind::Inside);
            // `>_`
            painter.line_segment([c + Vec2::new(-4.5, -2.5), c + Vec2::new(-1.5, 0.0)], stroke);
            painter.line_segment([c + Vec2::new(-1.5, 0.0), c + Vec2::new(-4.5, 2.5)], stroke);
            painter.line_segment([c + Vec2::new(0.5, 2.5), c + Vec2::new(4.5, 2.5)], stroke);
        }
        ActivityIcon::Settings => {
            painter.circle_stroke(c, 4.8, stroke);
            painter.circle_stroke(c, 1.8, stroke);
            for k in 0..8 {
                let ang = k as f32 * std::f32::consts::FRAC_PI_4;
                let dir = Vec2::new(ang.cos(), ang.sin());
                painter.line_segment([c + dir * 4.8, c + dir * 7.2], stroke);
            }
        }
        ActivityIcon::Theme => {
            painter.circle_stroke(c, 5.5, stroke);
            let half = Rect::from_min_max(Pos2::new(c.x, c.y - 5.0), Pos2::new(c.x + 5.0, c.y + 5.0));
            painter.rect_filled(half, 2.0, color.linear_multiply(0.55));
        }
    }

    response.on_hover_text(tooltip)
}
