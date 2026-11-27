use draco::settings::{Settings, TerminalLayout};

use crate::common::TempDir;

#[test]
fn default_settings_enable_auto_save_and_sensible_defaults() {
    let s = Settings::default();
    assert!(s.auto_save);
    assert_eq!(s.auto_save_delay_ms, 800);
    assert_eq!(s.theme, "dark");
    assert_eq!(s.font_size, 14.5);
    assert_eq!(s.tab_size, 4);
    assert!(s.show_indent_guides);
    assert_eq!(s.terminal_layout, TerminalLayout::Bottom);
}

#[test]
fn corrupt_or_missing_settings_file_falls_back_to_defaults() {
    let tmp = TempDir::new("unit_settings_bad");
    let missing = tmp.path().join("does_not_exist.json");
    assert_eq!(Settings::load_from(&missing), Settings::default());

    let corrupt = tmp.write_file("bad.json", "{ not valid json !!!");
    assert_eq!(Settings::load_from(&corrupt), Settings::default());
}

#[test]
fn settings_partial_json_preserves_unspecified_defaults() {
    let tmp = TempDir::new("unit_settings_partial");
    let partial = tmp.write_file("partial.json", r#"{"theme":"light","auto_save":false}"#);
    let loaded = Settings::load_from(&partial);
    assert_eq!(loaded.theme, "light");
    assert!(!loaded.auto_save);
    assert_eq!(loaded.font_size, 14.5); // defaulted
    assert_eq!(loaded.terminal_layout, TerminalLayout::Bottom);
}

#[test]
fn plugin_enable_disable_is_idempotent() {
    let mut s = Settings::default();
    assert!(s.is_plugin_enabled("fmt"));
    s.set_plugin_enabled("fmt", false);
    s.set_plugin_enabled("fmt", false);
    assert_eq!(s.disabled_plugins.len(), 1);
    assert!(!s.is_plugin_enabled("fmt"));
    s.set_plugin_enabled("fmt", true);
    assert!(s.is_plugin_enabled("fmt"));
    assert!(s.disabled_plugins.is_empty());
    assert!(!Settings::config_dir().as_os_str().is_empty());
}
