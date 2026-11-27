use draco::core::buffer::Document;
use draco::core::LanguageId;
use draco::syntax::SyntaxState;
use draco::theme::Theme;
use draco::ui::code_view::{self, CodeViewState};
use draco::ui::markdown_preview::{self, MarkdownPreviewCache, MarkdownViewMode};
use eframe::egui::{self, Event, Key, Modifiers};

use crate::common::{key_press, raw_input};

#[test]
fn markdown_split_preview_toc_and_live_edit_e2e() {
    let ctx = egui::Context::default();
    let mut doc = Document::from_str(
        1,
        "# Draco Docs\n\nXem [Changelog](./CHANGELOG.md) và [Mục dưới](#live-section).\n\n- [x] Fast\n\n```mermaid\nflowchart LR\n    A[Editor] <--> B[Preview]\n```\n\n```yaml\nport: 8080\n```\n",
        LanguageId::Markdown,
    );
    let mut syntax = SyntaxState::new(LanguageId::Markdown);
    let mut view = CodeViewState::default();
    let mut cache = MarkdownPreviewCache::default();
    let mut mode = MarkdownViewMode::Split;
    let mut clipboard = None;

    for theme in [Theme::dark(), Theme::light()] {
        let out = ctx.run(raw_input(vec![]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                markdown_preview::show_markdown_mode_switcher(
                    ui,
                    &mut mode,
                    &mut cache.show_toc,
                    &mut cache.scroll_sync,
                    &theme,
                );
                ui.horizontal(|ui| {
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
                    cache.notify_editor_scroll(out.top_visible_line);
                    markdown_preview::show_markdown_preview(ui, &doc, None, &mut cache, &theme);
                });
            });
        });
        assert!(!out.shapes.is_empty());
    }

    let before_blocks = cache.blocks.len();
    doc.set_cursor_at_line_col(doc.len_lines().saturating_sub(1), 0);
    let _ = ctx.run(
        raw_input(vec![
            key_press(Key::Enter, Modifiers::NONE),
            Event::Text("## Live Section".into()),
        ]),
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                code_view::show_code_view(
                    ui,
                    &mut doc,
                    &mut syntax,
                    &mut view,
                    &Theme::dark(),
                    &[],
                    &[],
                    true,
                    &mut clipboard,
                );
                markdown_preview::show_markdown_preview(ui, &doc, None, &mut cache, &Theme::dark());
            });
        },
    );
    assert!(cache.blocks.len() > before_blocks);
    assert_eq!(cache.headings.len(), 2);
}
