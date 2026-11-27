use draco::plugins::{PluginContext, PluginManager};
use draco::settings::Settings;

use crate::common::TempDir;

#[test]
fn rejects_invalid_plugin_id_and_missing_manifest() {
    let tmp = TempDir::new("unit_plug_invalid");
    let mut mgr = PluginManager::new(tmp.path().join("installed"));
    let settings = Settings::default();

    // Missing plugin.json
    let empty_src = tmp.path().join("empty");
    std::fs::create_dir_all(&empty_src).unwrap();
    assert!(mgr.install_from_dir(&empty_src, &settings).is_err());

    // Path traversal id rejected
    let bad_src = tmp.path().join("bad");
    std::fs::create_dir_all(&bad_src).unwrap();
    std::fs::write(
        bad_src.join("plugin.json"),
        r#"{"id":"../escape","name":"Bad","version":"1.0.0"}"#,
    )
    .unwrap();
    assert!(mgr.install_from_dir(&bad_src, &settings).is_err());
}

#[test]
fn disabled_plugin_commands_are_hidden_and_blocked() {
    let tmp = TempDir::new("unit_plug_disable");
    let src = tmp.path().join("src_p");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("plugin.json"),
        r#"{
            "id": "echo-plug",
            "name": "Echo Plugin",
            "version": "1.0.0",
            "commands": [{
                "id": "say_hi",
                "title": "Say Hi",
                "executable": "echo",
                "args": ["hello"],
                "input_mode": "none",
                "output_mode": "status"
            }]
        }"#,
    )
    .unwrap();

    let mut settings = Settings::default();
    let mut mgr = PluginManager::new(tmp.path().join("installed"));
    mgr.install_from_dir(&src, &settings).unwrap();
    assert_eq!(mgr.all_enabled_commands().len(), 1);

    // Disable plugin
    settings.set_plugin_enabled("echo-plug", false);
    mgr.reload(None, &settings);
    assert!(mgr.all_enabled_commands().is_empty());

    let err = mgr.execute_command(
        "echo-plug",
        "say_hi",
        &PluginContext {
            file_path: None,
            workspace_root: None,
            language: "PHP",
            line: 0,
            col: 0,
            stdin_text: String::new(),
        },
    );
    assert!(err.is_err());
}
