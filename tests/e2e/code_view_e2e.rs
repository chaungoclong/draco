use draco::core::buffer::Document;
use draco::core::LanguageId;
use draco::syntax::SyntaxState;
use draco::theme::Theme;
use draco::ui::code_view::{self, CodeViewState};
use eframe::egui::{self, Event, Key, Modifiers};

use crate::common::{key_press, raw_input};

#[test]
fn code_view_typing_auto_indent_comment_undo_redo_and_ctrl_d() {
    let ctx = egui::Context::default();
    let mut doc = Document::from_str(1, "", LanguageId::Rust);
    let mut syntax = SyntaxState::new(LanguageId::Rust);
    let mut view = CodeViewState::default();
    let theme = Theme::dark();
    let mut clipboard = None;

    // Frame 1: Type function + auto-close brace + auto-indent Enter + body
    let mut changed = false;
    let _ = ctx.run(
        raw_input(vec![
            Event::Text("fn run() ".into()),
            Event::Text("{".into()),
            key_press(Key::Enter, Modifiers::NONE),
            Event::Text("let x = 1;".into()),
        ]),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let out = code_view::show_code_view(
                    ui,
                    &mut doc,
                    &mut syntax,
                    &mut view,
                    &theme,
                    &[],
                    &[],
                    true,
                    &mut clipboard,
                );
                changed = out.text_changed;
            });
        },
    );
    assert!(changed);
    assert_eq!(doc.rope.to_string(), "fn run() {\n    let x = 1;\n}");

    // Frame 2: Ctrl+/ toggles comment
    let _ = ctx.run(raw_input(vec![key_press(Key::Slash, Modifiers::COMMAND)]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &[],
                true,
                &mut clipboard,
            );
        });
    });
    assert_eq!(doc.rope.to_string(), "fn run() {\n    // let x = 1;\n}");

    // Frame 3: Ctrl+Z undo, then Ctrl+Y redo
    let _ = ctx.run(raw_input(vec![key_press(Key::Z, Modifiers::COMMAND)]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &[],
                true,
                &mut clipboard,
            );
        });
    });
    assert_eq!(doc.rope.to_string(), "fn run() {\n    let x = 1;\n}");

    let _ = ctx.run(raw_input(vec![key_press(Key::Y, Modifiers::COMMAND)]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &[],
                true,
                &mut clipboard,
            );
        });
    });
    assert_eq!(doc.rope.to_string(), "fn run() {\n    // let x = 1;\n}");
}

#[test]
fn code_view_php_editing_and_trigger_completion_char() {
    let ctx = egui::Context::default();
    let mut doc = Document::from_str(1, "<?php\n$obj", LanguageId::Php);
    doc.set_cursor_at_line_col(1, 4);
    let mut syntax = SyntaxState::new(LanguageId::Php);
    let mut view = CodeViewState::default();
    let theme = Theme::dark();
    let mut clipboard = None;

    let mut triggered = false;
    let _ = ctx.run(raw_input(vec![Event::Text(":".into())]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let out = code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &[],
                true,
                &mut clipboard,
            );
            triggered = out.trigger_completion_char;
        });
    });
    assert!(triggered);
    assert_eq!(doc.rope.to_string(), "<?php\n$obj:");
}

#[test]
fn utf8_vietnamese_fonts_load_and_render_in_code_view() {
    use draco::ui::fonts;

    let ctx = egui::Context::default();
    let report = fonts::configure_utf8_fonts(&ctx);
    assert!(
        report.mono_loaded.is_some(),
        "Expected system UTF-8 monospace font to load"
    );
    assert!(
        report.proportional_loaded.is_some(),
        "Expected system UTF-8 proportional font to load"
    );

    let vietnamese = "// Tiếng Việt: ă â đ ê ô ơ ư ắ ằ ặ ệ ộ ứ ự ể ở\nlet lời_chào = \"Xin chào Draco!\";\n";
    let mut doc = Document::from_str(1, vietnamese, LanguageId::Rust);
    let mut syntax = SyntaxState::new(LanguageId::Rust);
    let mut view = CodeViewState::default();
    let theme = Theme::dark();
    let mut clipboard = None;

    let out = ctx.run(raw_input(vec![]), |ctx| {
        // Verify Vietnamese glyph widths are measured properly in Monospace
        let font = egui::FontId::monospace(14.5);
        for ch in ['ă', 'đ', 'ệ', 'ộ', 'ứ', 'ự', 'ở'] {
            let w = ctx.fonts(|f| f.glyph_width(&font, ch));
            assert!(w > 0.0, "Missing glyph width for Vietnamese char '{ch}'");
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &[],
                true,
                &mut clipboard,
            );
        });
    });
    assert!(!out.shapes.is_empty());
}
