use draco::plugins::{PluginContext, PluginManager, PluginOutputMode};
use draco::settings::Settings;

use crate::common::TempDir;

#[test]
fn plugin_install_workspace_discovery_env_vars_and_uninstall() {
    let tmp = TempDir::new("feat_plugins");
    let global_plugins = tmp.path().join("global_plugins");
    let workspace = tmp.path().join("project");
    std::fs::create_dir_all(&workspace).unwrap();

    // 1. Install a global plugin that transforms stdin to uppercase
    let src_global = tmp.path().join("src_upper");
    tmp.write_file(
        "src_upper/plugin.json",
        r#"{
            "id": "upper-tool",
            "name": "Upper Tool",
            "version": "1.2.0",
            "description": "Uppercases text",
            "author": "Draco",
            "commands": [{
                "id": "upper",
                "title": "Uppercase Selection",
                "executable": "tr",
                "args": ["a-z", "A-Z"],
                "input_mode": "selection_or_document",
                "output_mode": "replace_selection"
            }]
        }"#,
    );

    // 2. Add a workspace-local plugin under `.draco/plugins/` that reads DRACO_* env vars
    tmp.write_file(
        "project/.draco/plugins/env-inspector/plugin.json",
        r#"{
            "id": "env-inspector",
            "name": "Env Inspector",
            "version": "0.1.0",
            "commands": [{
                "id": "inspect",
                "title": "Inspect Context",
                "executable": "sh",
                "args": ["-c", "echo \"$DRACO_LANGUAGE:$DRACO_LINE:$DRACO_COL\""],
                "input_mode": "none",
                "output_mode": "status"
            }]
        }"#,
    );

    let settings = Settings::default();
    let mut mgr = PluginManager::new(global_plugins);
    mgr.install_from_dir(&src_global, &settings).unwrap();
    mgr.reload(Some(&workspace), &settings);
    assert_eq!(mgr.plugins.len(), 2);

    // Execute global plugin
    let r1 = mgr
        .execute_command(
            "upper-tool",
            "upper",
            &PluginContext {
                file_path: None,
                workspace_root: Some(&workspace),
                language: "PHP",
                line: 4,
                col: 2,
                stdin_text: "hello php".into(),
            },
        )
        .unwrap();
    assert!(r1.success);
    assert_eq!(r1.stdout, "HELLO PHP");
    assert_eq!(r1.output_mode, PluginOutputMode::ReplaceSelection);

    // Execute workspace plugin verifying 1-based DRACO_LINE / DRACO_COL + DRACO_LANGUAGE
    let r2 = mgr
        .execute_command(
            "env-inspector",
            "inspect",
            &PluginContext {
                file_path: None,
                workspace_root: Some(&workspace),
                language: "PHP",
                line: 4,
                col: 2,
                stdin_text: String::new(),
            },
        )
        .unwrap();
    assert!(r2.success);
    assert_eq!(r2.stdout.trim(), "PHP:5:3");

    // Uninstall global plugin
    mgr.uninstall("upper-tool", &settings).unwrap();
    mgr.reload(Some(&workspace), &settings);
    assert_eq!(mgr.plugins.len(), 1);
    assert_eq!(mgr.plugins[0].manifest.id, "env-inspector");
}
