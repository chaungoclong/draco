use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use eframe::egui::{self, ColorImage, RichText, Stroke, TextureHandle, TextureOptions, Vec2};

use crate::theme::{Theme, ThemeKind};

#[derive(Debug, Clone)]
pub struct RenderedMermaid {
    pub width: u32,
    pub height: u32,
    pub logical_width: f32,
    pub logical_height: f32,
    pub rgba_premultiplied: Vec<u8>,
}

fn shared_usvg_options() -> &'static resvg::usvg::Options<'static> {
    static OPTS: OnceLock<resvg::usvg::Options<'static>> = OnceLock::new();
    OPTS.get_or_init(|| {
        let mut opts = resvg::usvg::Options::default();
        opts.fontdb_mut().load_system_fonts();

        for candidate in [
            "Noto Sans",
            "DejaVu Sans",
            "Liberation Sans",
            "Ubuntu",
            "Segoe UI",
            "Arial",
        ] {
            if opts
                .fontdb
                .query(&resvg::usvg::fontdb::Query {
                    families: &[resvg::usvg::fontdb::Family::Name(candidate)],
                    ..Default::default()
                })
                .is_some()
            {
                opts.font_family = candidate.to_string();
                opts.fontdb_mut().set_sans_serif_family(candidate);
                opts.fontdb_mut().set_serif_family(candidate);
                break;
            }
        }

        for mono in [
            "Noto Sans Mono",
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Ubuntu Mono",
            "Consolas",
        ] {
            if opts
                .fontdb
                .query(&resvg::usvg::fontdb::Query {
                    families: &[resvg::usvg::fontdb::Family::Name(mono)],
                    ..Default::default()
                })
                .is_some()
            {
                opts.fontdb_mut().set_monospace_family(mono);
                break;
            }
        }

        opts
    })
}

/// Renders any of the 23 Mermaid diagram types to a crisp HiDPI RGBA bitmap via `mermaid-svg` + `resvg`.
pub fn render_mermaid_to_rgba(source: &str, dark: bool) -> Result<RenderedMermaid, String> {
    let opts = shared_usvg_options();
    let mut m_theme = mermaid_svg::Theme::by_name(if dark { "dark" } else { "default" })
        .unwrap_or_default();
    m_theme.font_family = format!("{}, sans-serif", opts.font_family).into();
    let svg = mermaid_svg::render_with(source, &m_theme).map_err(|e| e.to_string())?;
    rasterize_svg(&svg)
}

/// Rasterizes an SVG string at 2x HiDPI scale into premultiplied RGBA pixels using `resvg`.
pub fn rasterize_svg(svg: &str) -> Result<RenderedMermaid, String> {
    let tree = resvg::usvg::Tree::from_str(svg, shared_usvg_options())
        .map_err(|e| format!("SVG parse error: {e}"))?;

    let size = tree.size();
    let logical_w = size.width().max(1.0);
    let logical_h = size.height().max(1.0);

    // 2x HiDPI scale, capped at 2400px to keep texture memory lightweight
    let max_dim = 2400.0_f32;
    let scale = (2.0_f32)
        .min(max_dim / logical_w)
        .min(max_dim / logical_h)
        .max(0.5);

    let px_w = ((logical_w * scale).ceil() as u32).max(1);
    let px_h = ((logical_h * scale).ceil() as u32).max(1);

    let mut pixmap = resvg::tiny_skia::Pixmap::new(px_w, px_h)
        .ok_or_else(|| "Failed to allocate SVG pixmap".to_string())?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(RenderedMermaid {
        width: px_w,
        height: px_h,
        logical_width: logical_w,
        logical_height: logical_h,
        rgba_premultiplied: pixmap.take(),
    })
}

#[derive(Clone)]
enum CachedMermaidEntry {
    Ready {
        texture: TextureHandle,
        logical_size: Vec2,
    },
    Error(String),
}

fn source_hash(source: &str, dark: bool) -> u64 {
    let mut h = DefaultHasher::new();
    source.hash(&mut h);
    dark.hash(&mut h);
    h.finish()
}

/// Renders a Mermaid block inside Markdown Preview with GPU texture caching (0ms CPU after first frame).
pub fn show_mermaid_diagram(
    ui: &mut egui::Ui,
    source: &str,
    content_w: f32,
    theme: &Theme,
) {
    let dark = theme.kind == ThemeKind::Dark;
    let hash = source_hash(source, dark);
    let src_toggle_id = egui::Id::new(("draco_mermaid_show_src", hash));
    let cache_id = egui::Id::new(("draco_mermaid_tex", hash));

    let mut show_src = ui
        .ctx()
        .data_mut(|d| d.get_temp::<bool>(src_toggle_id).unwrap_or(false));

    egui::Frame::NONE
        .fill(theme.bg_panel)
        .stroke(Stroke::new(1.0_f32, theme.border))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width((content_w - 24.0).max(200.0));
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("MERMAID DIAGRAM")
                        .size(10.5)
                        .strong()
                        .color(theme.accent),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Copy").size(11.0).color(theme.fg_muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        ui.ctx().copy_text(source.to_string());
                    }
                    let toggle_lbl = if show_src { "Diagram" } else { "Source" };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(toggle_lbl).size(11.0).color(theme.fg_muted),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        show_src = !show_src;
                        ui.ctx().data_mut(|d| d.insert_temp(src_toggle_id, show_src));
                    }
                });
            });
            ui.separator();

            if show_src {
                ui.add(
                    egui::Label::new(
                        RichText::new(source)
                            .monospace()
                            .size(12.5)
                            .color(theme.fg_text),
                    )
                    .wrap(),
                );
                return;
            }

            let entry = ui
                .ctx()
                .data_mut(|d| d.get_temp::<CachedMermaidEntry>(cache_id))
                .unwrap_or_else(|| {
                    let built = match render_mermaid_to_rgba(source, dark) {
                        Ok(rendered) => {
                            let img = ColorImage::from_rgba_premultiplied(
                                [rendered.width as usize, rendered.height as usize],
                                &rendered.rgba_premultiplied,
                            );
                            let tex = ui.ctx().load_texture(
                                format!("draco_mermaid_{hash}"),
                                img,
                                TextureOptions::LINEAR,
                            );
                            CachedMermaidEntry::Ready {
                                texture: tex,
                                logical_size: Vec2::new(
                                    rendered.logical_width,
                                    rendered.logical_height,
                                ),
                            }
                        }
                        Err(err) => CachedMermaidEntry::Error(err),
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(cache_id, built.clone()));
                    built
                });

            match entry {
                CachedMermaidEntry::Ready {
                    texture,
                    logical_size,
                } => {
                    let max_w = (content_w - 32.0).max(200.0);
                    let display_size = if logical_size.x > max_w {
                        let ratio = max_w / logical_size.x;
                        Vec2::new(max_w, logical_size.y * ratio)
                    } else {
                        logical_size
                    };
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Image::new((texture.id(), display_size)));
                    });
                }
                CachedMermaidEntry::Error(err) => {
                    ui.label(
                        RichText::new(format!("Mermaid syntax error: {err}"))
                            .size(12.0)
                            .color(theme.diag_warning),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(source)
                                .monospace()
                                .size(12.0)
                                .color(theme.fg_muted),
                        )
                        .wrap(),
                    );
                }
            }
        });
}
