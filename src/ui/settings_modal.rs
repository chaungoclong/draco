use eframe::egui::{self, Align2, Key, RichText, Stroke, Vec2};

use crate::plugins::PluginManager;
use crate::settings::{Settings, TerminalLayout};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Editor,
    Terminal,
    Plugins,
}

#[derive(Default)]
pub struct SettingsModalState {
    pub open: bool,
    pub tab: SettingsTab,
}

#[derive(Debug, Clone)]
pub enum SettingsModalAction {
    SettingsChanged,
    InstallPluginDialog,
    ReloadPlugins,
    TogglePlugin { id: String, enabled: bool },
    UninstallPlugin(String),
    RunPluginCommand { plugin_id: String, command_id: String },
}

pub fn show_settings_modal(
    ctx: &egui::Context,
    state: &mut SettingsModalState,
    settings: &mut Settings,
    plugins: &PluginManager,
    theme: &Theme,
) -> Option<SettingsModalAction> {
    if !state.open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }

    let mut action = None;

    egui::Window::new("Settings")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .default_width(600.0)
        .max_width(600.0)
        .frame(
            egui::Frame::NONE
                .fill(theme.bg_popup)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(8.0)
                .shadow(egui::Shadow {
                    offset: [0, 10],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(100),
                })
                .inner_margin(14.0),
        )
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.label(RichText::new("Settings").strong().size(16.0).color(theme.fg_text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("×").clicked() {
                        state.open = false;
                    }
                });
            });
            ui.add_space(6.0);

            // Tabs
            ui.horizontal(|ui| {
                for (t, label) in [
                    (SettingsTab::Editor, "Editor & Auto-Save"),
                    (SettingsTab::Terminal, "Terminal"),
                    (SettingsTab::Plugins, "Plugins"),
                ] {
                    let active = state.tab == t;
                    if ui
                        .selectable_label(
                            active,
                            RichText::new(label)
                                .size(13.0)
                                .color(if active { theme.accent } else { theme.fg_muted }),
                        )
                        .clicked()
                    {
                        state.tab = t;
                    }
                }
            });
            ui.separator();
            ui.add_space(8.0);

            match state.tab {
                SettingsTab::Editor => {
                    if ui
                        .checkbox(&mut settings.auto_save, "Enable Auto-Save (saves modified files automatically)")
                        .changed()
                    {
                        action = Some(SettingsModalAction::SettingsChanged);
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Auto-Save Delay (ms):");
                        let mut delay = settings.auto_save_delay_ms as f32;
                        if ui
                            .add(egui::Slider::new(&mut delay, 200.0..=5000.0).step_by(100.0))
                            .changed()
                        {
                            settings.auto_save_delay_ms = delay as u64;
                            action = Some(SettingsModalAction::SettingsChanged);
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Theme:");
                        let is_dark = settings.theme == "dark";
                        if ui.selectable_label(is_dark, "Dark (Tokyo Night)").clicked() && !is_dark {
                            settings.theme = "dark".to_string();
                            action = Some(SettingsModalAction::SettingsChanged);
                        }
                        if ui.selectable_label(!is_dark, "Light").clicked() && is_dark {
                            settings.theme = "light".to_string();
                            action = Some(SettingsModalAction::SettingsChanged);
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        if ui
                            .add(egui::Slider::new(&mut settings.font_size, 10.0..=28.0).step_by(0.5))
                            .changed()
                        {
                            action = Some(SettingsModalAction::SettingsChanged);
                        }
                    });
                    ui.add_space(6.0);
                    if ui
                        .checkbox(&mut settings.show_indent_guides, "Show vertical indent guides")
                        .changed()
                    {
                        action = Some(SettingsModalAction::SettingsChanged);
                    }
                }
                SettingsTab::Terminal => {
                    ui.label(RichText::new("Terminal Display Mode").strong().color(theme.fg_text));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        for mode in [
                            TerminalLayout::Bottom,
                            TerminalLayout::Right,
                            TerminalLayout::Floating,
                            TerminalLayout::Maximized,
                        ] {
                            let active = settings.terminal_layout == mode;
                            if ui.selectable_label(active, mode.label()).clicked() {
                                settings.terminal_layout = mode;
                                action = Some(SettingsModalAction::SettingsChanged);
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Tip: Use '+' in the terminal header or Ctrl+Shift+` to open multiple terminal tabs.")
                            .size(12.0)
                            .color(theme.fg_muted),
                    );
                }
                SettingsTab::Plugins => {
                    ui.horizontal(|ui| {
                        if ui.button("Install Plugin Folder…").clicked() {
                            action = Some(SettingsModalAction::InstallPluginDialog);
                        }
                        if ui.button("Reload Plugins").clicked() {
                            action = Some(SettingsModalAction::ReloadPlugins);
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Plugins directory: {}", plugins.plugins_dir().display()))
                            .size(11.5)
                            .color(theme.fg_muted),
                    );
                    ui.add_space(8.0);

                    if plugins.plugins.is_empty() {
                        ui.label(
                            RichText::new(
                                "No plugins installed yet. Select a folder containing plugin.json to install.",
                            )
                            .color(theme.fg_muted),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(260.0)
                            .show(ui, |ui| {
                                for p in &plugins.plugins {
                                    egui::Frame::NONE
                                        .fill(theme.bg_panel)
                                        .stroke(Stroke::new(1.0_f32, theme.border))
                                        .corner_radius(5.0)
                                        .inner_margin(8.0)
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let mut enabled = p.enabled;
                                                if ui.checkbox(&mut enabled, "").changed() {
                                                    action = Some(SettingsModalAction::TogglePlugin {
                                                        id: p.manifest.id.clone(),
                                                        enabled,
                                                    });
                                                }
                                                ui.label(
                                                    RichText::new(&p.manifest.name)
                                                        .strong()
                                                        .color(theme.fg_text),
                                                );
                                                ui.label(
                                                    RichText::new(format!("v{}", p.manifest.version))
                                                        .size(11.5)
                                                        .color(theme.fg_muted),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.small_button("Uninstall").clicked() {
                                                            action = Some(
                                                                SettingsModalAction::UninstallPlugin(
                                                                    p.manifest.id.clone(),
                                                                ),
                                                            );
                                                        }
                                                    },
                                                );
                                            });
                                            if !p.manifest.description.is_empty() {
                                                ui.label(
                                                    RichText::new(&p.manifest.description)
                                                        .size(12.0)
                                                        .color(theme.fg_muted),
                                                );
                                            }
                                            if p.enabled && !p.manifest.commands.is_empty() {
                                                ui.horizontal_wrapped(|ui| {
                                                    for cmd in &p.manifest.commands {
                                                        if ui
                                                            .small_button(format!("▶ {}", cmd.title))
                                                            .clicked()
                                                        {
                                                            action = Some(
                                                                SettingsModalAction::RunPluginCommand {
                                                                    plugin_id: p.manifest.id.clone(),
                                                                    command_id: cmd.id.clone(),
                                                                },
                                                            );
                                                            state.open = false;
                                                        }
                                                    }
                                                });
                                            }
                                        });
                                    ui.add_space(4.0);
                                }
                            });
                    }
                }
            }
        });

    action
}
