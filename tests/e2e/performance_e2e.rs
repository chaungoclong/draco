use std::time::Instant;

use draco::core::buffer::Document;
use draco::core::LanguageId;
use draco::lsp::client::DiagnosticItem;
use draco::syntax::SyntaxState;
use draco::theme::Theme;
use draco::ui::code_view::{self, CodeViewState};
use eframe::egui::{self, Event};

use crate::common::raw_input;

#[test]
fn virtualized_100k_line_file_renders_and_edits_quickly() {
    let mut big = String::with_capacity(4_500_000);
    for i in 0..100_000 {
        big.push_str(&format!("fn line_{i}() -> usize {{ {i} }}\n"));
    }

    let ctx = egui::Context::default();
    let mut doc = Document::from_str(1, &big, LanguageId::Rust);
    let mut syntax = SyntaxState::new(LanguageId::Rust);
    let mut view = CodeViewState::default();
    let theme = Theme::dark();
    let mut clipboard = None;

    syntax.sync(&doc.rope, doc.language, doc.version, &[]);

    doc.set_cursor_at_line_col(50_000, 0);
    view.scroll_to_line = Some(50_000);
    let diags = vec![DiagnosticItem {
        start_line: 50_000,
        start_col: 0,
        end_line: 50_000,
        end_col: 2,
        severity: 2,
        message: "unused fn".into(),
    }];

    let t0 = Instant::now();
    let _ = ctx.run(raw_input(vec![Event::Text("// ".into())]), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            code_view::show_code_view(
                ui,
                &mut doc,
                &mut syntax,
                &mut view,
                &theme,
                &[],
                &diags,
                true,
                &mut clipboard,
            );
        });
    });
    let elapsed = t0.elapsed();

    assert!(doc
        .line_string_without_newline(50_000)
        .starts_with("// fn line_50000"));
    assert!(
        elapsed.as_millis() < 250,
        "Virtualized 100k-line frame edit took too long: {elapsed:?}"
    );
}
