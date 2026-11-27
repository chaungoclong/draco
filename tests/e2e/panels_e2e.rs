use std::path::{Path, PathBuf};

use draco::settings::TerminalLayout;
use draco::terminal::manager::TerminalManager;
use draco::theme::Theme;
use draco::ui::explorer::{self, ExplorerUiState};
use draco::ui::icons::{self, ActivityIcon, FileIconKind};
use draco::ui::status_bar::{self, StatusInfo};
use draco::ui::tabs::{self, TabInfo};
use draco::ui::terminal_panel;
use draco::workspace::file_tree::FileTree;
use eframe::egui;

use crate::common::{raw_input, TempDir};

#[test]
fn all_panels_icons_dirty_indicators_and_themes_render_cleanly() {
    let tmp = TempDir::new("e2e_panels");
    let rust_file = tmp.write_file("src/main.rs", "fn main() {}");
    let php_file = tmp.write_file("public/index.php", "<?php echo 1;");

    assert_eq!(FileIconKind::from_path(Path::new("main.rs")), FileIconKind::Rust);
    assert_eq!(FileIconKind::from_path(Path::new("index.php")), FileIconKind::Php);
    assert_eq!(FileIconKind::from_path(Path::new("app.tsx")), FileIconKind::TypeScript);
    assert_eq!(FileIconKind::from_path(Path::new("data.json")), FileIconKind::Json);
    assert_eq!(FileIconKind::from_path(Path::new(".gitignore")), FileIconKind::Git);

    let ctx = egui::Context::default();
    let mut tree = Some(FileTree::open(tmp.path().to_path_buf()));
    let mut explorer_ui = ExplorerUiState::default();
    let dirty_paths = vec![rust_file.clone()];
    let mut term_mgr = TerminalManager::new(TerminalLayout::Bottom);
    let mut term_focus = false;

    for theme in [Theme::dark(), Theme::light()] {
        for layout in [
            TerminalLayout::Bottom,
            TerminalLayout::Right,
            TerminalLayout::Floating,
            TerminalLayout::Maximized,
        ] {
            term_mgr.layout = layout;
            let out = ctx.run(raw_input(vec![]), |ctx| {
                egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
                    let infos = vec![
                        TabInfo {
                            title: "main.rs".into(),
                            path: Some(rust_file.clone()),
                            dirty: true,
                        },
                        TabInfo {
                            title: "index.php".into(),
                            path: Some(php_file.clone()),
                            dirty: false,
                        },
                    ];
                    tabs::show_tabs(ui, &infos, 0, &theme);
                });
                egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
                    let info = StatusInfo {
                        line: 12,
                        col: 3,
                        cursor_count: 2,
                        language: "PHP",
                        lsp_status: Some("LSP: Ready"),
                        errors: 1,
                        warnings: 2,
                        dirty: true,
                        has_path: true,
                        auto_save: true,
                        message: Some("Ready"),
                    };
                    status_bar::show_status_bar(ui, &info, &theme);
                });
                egui::SidePanel::left("activity").exact_width(44.0).show(ctx, |ui| {
                    icons::activity_button(ui, ActivityIcon::Files, true, "Explorer", &theme);
                    icons::activity_button(ui, ActivityIcon::Search, false, "Search", &theme);
                    icons::activity_button(ui, ActivityIcon::QuickOpen, false, "Quick Open", &theme);
                    icons::activity_button(ui, ActivityIcon::Terminal, true, "Terminal", &theme);
                    icons::activity_button(ui, ActivityIcon::Settings, false, "Settings", &theme);
                    icons::activity_button(ui, ActivityIcon::Theme, false, "Theme", &theme);
                });
                egui::SidePanel::left("explorer").show(ctx, |ui| {
                    explorer::show_explorer(
                        ui,
                        &mut tree,
                        &mut explorer_ui,
                        Some(&rust_file),
                        &dirty_paths,
                        &theme,
                    );
                });
                egui::CentralPanel::default().show(ctx, |ui| {
                    tabs::show_breadcrumbs(
                        ui,
                        Some(&rust_file),
                        Some(tmp.path()),
                        true,
                        &theme,
                    );
                    tabs::show_breadcrumbs(ui, None, Some(tmp.path()), false, &theme);
                    terminal_panel::show_terminal_panel(ui, &mut term_mgr, &mut term_focus, &theme);
                });
            });
            assert!(!out.shapes.is_empty());
        }
    }
    let _ = PathBuf::from(&php_file);
}
