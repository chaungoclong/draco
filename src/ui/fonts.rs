use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query, Style, Weight};

#[derive(Debug, Default, Clone)]
pub struct FontLoadReport {
    pub mono_loaded: Option<String>,
    pub proportional_loaded: Option<String>,
    pub symbol_fallback_loaded: Option<String>,
}

const PREFERRED_MONO_FAMILIES: &[&str] = &[
    "Noto Sans Mono",
    "JetBrains Mono",
    "Cascadia Mono",
    "DejaVu Sans Mono",
    "Liberation Mono",
    "Ubuntu Mono",
    "SF Mono",
    "Menlo",
    "Consolas",
];

const PREFERRED_SANS_FAMILIES: &[&str] = &[
    "Noto Sans",
    "Inter",
    "DejaVu Sans",
    "Liberation Sans",
    "Ubuntu",
    "SF Pro Text",
    "Segoe UI",
    "Arial",
];

const PREFERRED_SYMBOL_FAMILIES: &[&str] = &["DejaVu Sans", "Noto Sans Symbols", "Noto Sans Symbols 2"];

fn query_font_bytes(
    db: &Database,
    preferred_names: &[&str],
    generic_fallback: Option<Family<'_>>,
    weight: Weight,
) -> Option<(String, Vec<u8>)> {
    let mut families: Vec<Family<'_>> = preferred_names.iter().map(|n| Family::Name(n)).collect();
    if let Some(fallback) = generic_fallback {
        families.push(fallback);
    }

    let query = Query {
        families: &families,
        weight,
        stretch: fontdb::Stretch::Normal,
        style: Style::Normal,
    };

    let face_id = db.query(&query)?;
    let label = db
        .face(face_id)
        .and_then(|f| f.families.first().map(|(name, _)| name.clone()))
        .unwrap_or_else(|| "system-font".to_string());
    let bytes = db.with_face_data(face_id, |data, _index| data.to_vec())?;
    Some((label, bytes))
}

/// Discovers and loads UTF-8 / Vietnamese-capable system fonts across Linux (fontconfig + user dirs),
/// macOS, and Windows using `fontdb`.
pub fn configure_utf8_fonts(ctx: &egui::Context) -> FontLoadReport {
    let mut db = Database::new();
    db.load_system_fonts();

    let mut fonts = FontDefinitions::default();
    let mut report = FontLoadReport::default();

    if let Some((label, bytes)) = query_font_bytes(
        &db,
        PREFERRED_MONO_FAMILIES,
        Some(Family::Monospace),
        Weight::NORMAL,
    ) {
        let name = "draco_utf8_mono".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, name);
        report.mono_loaded = Some(label);
    }

    if let Some((label, bytes)) = query_font_bytes(
        &db,
        PREFERRED_SANS_FAMILIES,
        Some(Family::SansSerif),
        Weight::NORMAL,
    ) {
        let name = "draco_utf8_sans".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, name.clone());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push(name);
        report.proportional_loaded = Some(label);
    }

    // Load real Bold sans font for crisp Markdown headings & **bold** text
    if let Some((_, bytes)) = query_font_bytes(
        &db,
        PREFERRED_SANS_FAMILIES,
        Some(Family::SansSerif),
        Weight::BOLD,
    ) {
        let name = "draco_utf8_bold".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        let bold_family = FontFamily::Name("draco_bold".into());
        let mut list = vec![name];
        if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
            list.extend(prop.iter().cloned());
        }
        fonts.families.insert(bold_family, list);
    }

    if let Some((label, bytes)) =
        query_font_bytes(&db, PREFERRED_SYMBOL_FAMILIES, None, Weight::NORMAL)
    {
        let name = "draco_utf8_symbols".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .push(name.clone());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push(name);
        report.symbol_fallback_loaded = Some(label);
    }

    ctx.set_fonts(fonts);
    report
}
