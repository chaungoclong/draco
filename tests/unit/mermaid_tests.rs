use draco::ui::mermaid::render_mermaid_to_rgba;

#[test]
fn renders_multiple_mermaid_diagram_types_to_rgba_in_dark_and_light() {
    let diagrams = [
        (
            "flowchart",
            "flowchart TD\n    A[Start] --> B{Decision}\n    B -->|Yes| C[Done]\n",
        ),
        (
            "sequence",
            "sequenceDiagram\n    participant E as Editor\n    participant L as LSP\n    E->>L: completion\n    L-->>E: items\n",
        ),
        ("pie", "pie\n    \"Rust\" : 70\n    \"PHP\" : 30\n"),
        (
            "state",
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> Editing\n    Editing --> [*]\n",
        ),
        (
            "class",
            "classDiagram\n    class Document {\n        +Rope rope\n        +save()\n    }\n",
        ),
        (
            "er",
            "erDiagram\n    USER ||--o{ ORDER : places\n",
        ),
    ];

    for (name, src) in diagrams {
        for dark in [true, false] {
            let rendered = render_mermaid_to_rgba(src, dark)
                .unwrap_or_else(|e| panic!("Failed to render {name} (dark={dark}): {e}"));
            assert!(rendered.width > 0 && rendered.height > 0);
            assert_eq!(
                rendered.rgba_premultiplied.len(),
                (rendered.width * rendered.height * 4) as usize
            );
        }
    }
}

#[test]
fn invalid_mermaid_returns_graceful_error() {
    let res = render_mermaid_to_rgba("not_a_valid_diagram_type\n???", true);
    assert!(res.is_err());
}

#[test]
fn mermaid_renders_visible_text_pixels_with_system_sans_font() {
    // Compare rendered bitmap of a node with long text vs empty text — if text renders,
    // pixel data MUST differ significantly!
    let with_text = render_mermaid_to_rgba("flowchart LR\n    A[Kiến trúc hệ thống Draco]\n", false).unwrap();
    let without_text = render_mermaid_to_rgba("flowchart LR\n    A[ ]\n", false).unwrap();
    assert_ne!(
        with_text.rgba_premultiplied, without_text.rgba_premultiplied,
        "Expected Mermaid SVG text glyphs to be rasterized by resvg"
    );
    // Also verify significant dark text pixels exist in Light theme
    let dark_pixels = with_text
        .rgba_premultiplied
        .chunks_exact(4)
        .filter(|px| px[3] > 200 && px[0] < 90 && px[1] < 90 && px[2] < 90)
        .count();
    assert!(
        dark_pixels > 150,
        "Expected visible dark text pixels in rendered Mermaid diagram, found only {dark_pixels}"
    );
}
