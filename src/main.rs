#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use draco::app;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let initial_paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Draco")
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([640.0, 400.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/draco.png"))
    {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Draco",
        options,
        Box::new(move |cc| Ok(Box::new(app::DracoApp::new(cc, initial_paths)))),
    )
}
