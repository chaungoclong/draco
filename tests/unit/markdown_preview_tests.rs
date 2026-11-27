use draco::core::buffer::Document;
use draco::core::editing;
use draco::core::LanguageId;
use draco::ui::markdown_preview::{
    anchor_matches, compute_editor_line_from_preview_y, compute_preview_scroll_y,
    heading_ascii_slug, heading_slug, parse_inline, parse_markdown, parse_markdown_mapped,
    resolve_markdown_link, BlockLayoutPos, InlineSpan, MarkdownPreviewCache, MarkdownViewMode,
    MdBlock, PreviewAction,
};

use crate::common::TempDir;

#[test]
fn parses_all_markdown_blocks_and_code_block_highlighting() {
    let md = r#"# Draco Editor

Intro with **bold**, *italic*, `code`, ~~old~~, and [docs](https://example.com).

> Fast and lightweight.

- [x] Done task
- [ ] Pending task
- Bullet item
1. First step

| Feature | Status |
|---------|--------|
| YAML    | Ready  |
| Preview | Fast   |

---

```yaml
server:
  port: 8080
```

```php
<?php
function hi() { return 1; }
```
"#;

    let blocks = parse_markdown(md);
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::Heading { level: 1, .. })));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::Paragraph(_))));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::Blockquote(_))));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::TaskItem { checked: true, .. })));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::TaskItem { checked: false, .. })));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::BulletItem { .. })));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::OrderedItem { .. })));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::Table { headers, rows } if headers.len() == 2 && rows.len() == 2)));
    assert!(blocks.iter().any(|b| matches!(b, MdBlock::HorizontalRule)));

    // Verify fenced code blocks received real syntax highlight spans
    let code_blocks: Vec<_> = blocks
        .iter()
        .filter_map(|b| match b {
            MdBlock::CodeBlock { lang, spans, .. } => Some((lang.as_str(), spans.len())),
            _ => None,
        })
        .collect();
    assert_eq!(code_blocks.len(), 2);
    assert_eq!(code_blocks[0].0, "yaml");
    assert!(code_blocks[0].1 > 0);
    assert_eq!(code_blocks[1].0, "php");
    assert!(code_blocks[1].1 > 0);
}

#[test]
fn inline_parser_handles_mixed_styles() {
    let spans = parse_inline("Hi **Draco** *fast* `fn()` ~~bad~~ [link](url)");
    assert!(spans.contains(&InlineSpan::Bold("Draco".into())));
    assert!(spans.contains(&InlineSpan::Italic("fast".into())));
    assert!(spans.contains(&InlineSpan::Code("fn()".into())));
    assert!(spans.contains(&InlineSpan::Strike("bad".into())));
    assert!(spans.contains(&InlineSpan::Link {
        label: "link".into(),
        url: "url".into()
    }));
}

#[test]
fn preview_cache_tracks_document_version_and_mode_cycles() {
    assert_eq!(MarkdownViewMode::Editor.cycle(), MarkdownViewMode::Split);
    assert_eq!(MarkdownViewMode::Split.cycle(), MarkdownViewMode::Preview);
    assert_eq!(MarkdownViewMode::Preview.cycle(), MarkdownViewMode::Editor);

    let mut doc = Document::from_str(1, "# Title\n", LanguageId::Markdown);
    let mut cache = MarkdownPreviewCache::default();
    cache.sync(&doc);
    assert_eq!(cache.cached_version(), doc.version);
    assert_eq!(cache.blocks.len(), 1);

    // Edit doc -> version bumps -> cache updates automatically
    doc.set_cursor_at_line_col(1, 0);
    editing::insert_text(&mut doc, "\nNew paragraph");
    cache.sync(&doc);
    assert_eq!(cache.cached_version(), doc.version);
    assert_eq!(cache.blocks.len(), 2);
}

#[test]
fn vietnamese_heading_slugs_toc_and_relative_link_resolution() {
    assert_eq!(heading_slug("Cấu trúc tài liệu"), "cấu-trúc-tài-liệu");
    assert_eq!(heading_ascii_slug("Cấu trúc tài liệu"), "cau-truc-tai-lieu");
    assert!(anchor_matches(
        "cau-truc-tai-lieu",
        "cấu-trúc-tài-liệu",
        "cau-truc-tai-lieu"
    ));
    assert!(anchor_matches(
        "cấu-trúc-tài-liệu",
        "cấu-trúc-tài-liệu",
        "cau-truc-tai-lieu"
    ));

    // TOC extraction
    let doc = Document::from_str(
        1,
        "# Giới thiệu\n\n## Cấu trúc tài liệu\n\n### Chi tiết\n",
        LanguageId::Markdown,
    );
    let mut cache = MarkdownPreviewCache::default();
    cache.sync(&doc);
    assert_eq!(cache.headings.len(), 3);
    assert_eq!(cache.headings[1].title, "Cấu trúc tài liệu");
    assert_eq!(cache.headings[1].slug, "cấu-trúc-tài-liệu");
    assert_eq!(cache.headings[1].ascii_slug, "cau-truc-tai-lieu");

    // Relative file link resolution + anchor + external URL
    let tmp = TempDir::new("md_links");
    let readme = tmp.write_file("docs/README.md", "# Hub");
    let changelog = tmp.write_file("docs/CHANGELOG.md", "# Changelog");
    let canon_changelog = std::fs::canonicalize(&changelog).unwrap();

    let resolved =
        resolve_markdown_link("./CHANGELOG.md#unreleased", Some(&readme), Some(tmp.path()));
    assert_eq!(
        resolved,
        Some(PreviewAction::OpenFile {
            path: canon_changelog,
            anchor: Some("unreleased".into()),
        })
    );

    assert_eq!(
        resolve_markdown_link("https://rust-lang.org", Some(&readme), None),
        Some(PreviewAction::OpenExternalUrl("https://rust-lang.org".into()))
    );
    assert_eq!(
        resolve_markdown_link("#cau-truc-tai-lieu", Some(&readme), None),
        None // in-page anchor handled directly via scroll_to_anchor
    );
}

#[test]
fn block_mapped_scroll_sync_interpolates_accurately() {
    let md = "# Top\n\nParagraph\n\n```rust\nfn a() {}\nfn b() {}\nfn c() {}\n```\n\n## Bottom\n";
    let rope = ropey::Rope::from_str(md);
    let mapped = parse_markdown_mapped(md, &rope);
    assert_eq!(mapped[0].start_line, 0);
    assert!(mapped.last().unwrap().start_line >= 10);

    // Simulated layout positions: block 0 (lines 0..=0 -> y 0..40), block 1 (lines 4..=8 -> y 100..300)
    let positions = vec![
        BlockLayoutPos {
            start_line: 0,
            end_line: 0,
            top_y: 0.0,
            bottom_y: 40.0,
        },
        BlockLayoutPos {
            start_line: 4,
            end_line: 8,
            top_y: 100.0,
            bottom_y: 300.0,
        },
    ];

    assert_eq!(compute_preview_scroll_y(0.0, &positions), Some(0.0));
    // Halfway through the gap (lines 1..4 -> line 2.5): halfway between 40.0 and 100.0 = 70.0
    assert!((compute_preview_scroll_y(2.5, &positions).unwrap() - 70.0).abs() < 0.01);
    // Halfway through the 5-line code block (4..9 -> line 6.5): halfway between 100.0 and 300.0 = 200.0
    assert!((compute_preview_scroll_y(6.5, &positions).unwrap() - 200.0).abs() < 0.01);
    // Past end clamps to last bottom_y
    assert_eq!(compute_preview_scroll_y(50.0, &positions), Some(300.0));

    // Bidirectional inverse: preview Y -> exact editor fractional line
    assert_eq!(compute_editor_line_from_preview_y(0.0, &positions), Some(0.0));
    assert!((compute_editor_line_from_preview_y(70.0, &positions).unwrap() - 2.5).abs() < 0.01);
    assert!((compute_editor_line_from_preview_y(200.0, &positions).unwrap() - 6.5).abs() < 0.01);
    assert_eq!(compute_editor_line_from_preview_y(999.0, &positions), Some(8.0));
}
