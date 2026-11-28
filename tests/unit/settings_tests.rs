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

#[test]
fn recent_workspaces_mru_dedup_cap_and_valid_last_workspace() {
    let tmp = TempDir::new("unit_settings_ws");
    let existing_dir = tmp.path().to_path_buf();
    let missing_dir = tmp.path().join("does_not_exist_dir");

    let mut s = Settings::default();
    assert_eq!(s.valid_last_workspace(), None);

    // Record 12 folders -> capped at 10 in MRU order
    for i in 0..12 {
        s.record_recent_workspace(std::path::PathBuf::from(format!("/tmp/proj_{i}")));
    }
    assert_eq!(s.recent_workspaces.len(), 10);
    assert_eq!(
        s.last_workspace,
        Some(std::path::PathBuf::from("/tmp/proj_11"))
    );

    // Re-recording an existing entry moves it to index 0 without duplicating
    s.record_recent_workspace(existing_dir.clone());
    assert_eq!(s.recent_workspaces.len(), 10);
    assert_eq!(s.recent_workspaces[0], existing_dir);
    assert_eq!(s.valid_last_workspace(), Some(existing_dir.clone()));

    // Missing directory is not returned by valid_last_workspace()
    s.record_recent_workspace(missing_dir.clone());
    assert_eq!(s.valid_last_workspace(), None);

    // Removing missing_dir falls back last_workspace to existing_dir
    s.remove_recent_workspace(&missing_dir);
    assert_eq!(s.last_workspace, Some(existing_dir.clone()));
    assert_eq!(s.valid_last_workspace(), Some(existing_dir));

    // Roundtrip JSON persistence
    let path = tmp.path().join("settings.json");
    s.save_to(&path).unwrap();
    let loaded = Settings::load_from(&path);
    assert_eq!(loaded, s);
}
