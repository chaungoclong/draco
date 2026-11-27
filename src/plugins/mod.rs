use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginInputMode {
    #[default]
    None,
    Selection,
    Document,
    SelectionOrDocument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginOutputMode {
    #[default]
    Status,
    Insert,
    ReplaceSelection,
    ReplaceDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub input_mode: PluginInputMode,
    #[serde(default)]
    pub output_mode: PluginOutputMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub dir: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PluginContext<'a> {
    pub file_path: Option<&'a Path>,
    pub workspace_root: Option<&'a Path>,
    pub language: &'a str,
    pub line: usize,
    pub col: usize,
    pub stdin_text: String,
}

#[derive(Debug, Clone)]
pub struct PluginExecResult {
    pub output_mode: PluginOutputMode,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub struct PluginManager {
    plugins_dir: PathBuf,
    pub plugins: Vec<InstalledPlugin>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            plugins: Vec::new(),
        }
    }

    pub fn default_manager() -> Self {
        Self::new(Settings::plugins_dir())
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Scans `plugins_dir` (and optional workspace `.draco/plugins`) for `plugin.json` manifests.
    pub fn reload(&mut self, workspace_root: Option<&Path>, settings: &Settings) {
        self.plugins.clear();
        let mut search_dirs = vec![self.plugins_dir.clone()];
        if let Some(root) = workspace_root {
            search_dirs.push(root.join(".draco").join("plugins"));
        }

        for base in search_dirs {
            let Ok(entries) = fs::read_dir(&base) else {
                continue;
            };
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let manifest_path = dir.join("plugin.json");
                let Ok(raw) = fs::read_to_string(&manifest_path) else {
                    continue;
                };
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&raw) {
                    if self.plugins.iter().any(|p| p.manifest.id == manifest.id) {
                        continue;
                    }
                    let enabled = settings.is_plugin_enabled(&manifest.id);
                    self.plugins.push(InstalledPlugin {
                        manifest,
                        dir,
                        enabled,
                    });
                }
            }
        }
        self.plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    }

    /// Installs a plugin from a directory containing `plugin.json` into `plugins_dir/<id>/`.
    pub fn install_from_dir(
        &mut self,
        source_dir: &Path,
        settings: &Settings,
    ) -> Result<PluginManifest, String> {
        let manifest_path = source_dir.join("plugin.json");
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Cannot read plugin.json: {e}"))?;
        let manifest: PluginManifest = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid plugin.json: {e}"))?;
        if manifest.id.trim().is_empty()
            || manifest.id.contains('/')
            || manifest.id.contains('\\')
            || manifest.id.contains("..")
        {
            return Err("Invalid plugin id".to_string());
        }

        let target = self.plugins_dir.join(&manifest.id);
        fs::create_dir_all(&target).map_err(|e| format!("Create plugin dir failed: {e}"))?;
        copy_dir_recursive(source_dir, &target)
            .map_err(|e| format!("Copy plugin files failed: {e}"))?;

        self.reload(None, settings);
        Ok(manifest)
    }

    pub fn uninstall(&mut self, plugin_id: &str, settings: &Settings) -> Result<(), String> {
        let target = self.plugins_dir.join(plugin_id);
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|e| format!("Uninstall failed: {e}"))?;
        }
        self.reload(None, settings);
        Ok(())
    }

    pub fn all_enabled_commands(&self) -> Vec<(String, String, PluginCommand)> {
        let mut out = Vec::new();
        for p in &self.plugins {
            if !p.enabled {
                continue;
            }
            for cmd in &p.manifest.commands {
                out.push((p.manifest.id.clone(), p.manifest.name.clone(), cmd.clone()));
            }
        }
        out
    }

    pub fn execute_command(
        &self,
        plugin_id: &str,
        command_id: &str,
        ctx: &PluginContext<'_>,
    ) -> Result<PluginExecResult, String> {
        let plugin = self
            .plugins
            .iter()
            .find(|p| p.manifest.id == plugin_id && p.enabled)
            .ok_or_else(|| format!("Plugin '{plugin_id}' not found or disabled"))?;
        let cmd_spec = plugin
            .manifest
            .commands
            .iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| format!("Command '{command_id}' not found"))?;

        let mut command = Command::new(&cmd_spec.executable);
        command
            .args(&cmd_spec.args)
            .current_dir(&plugin.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("DRACO_LANGUAGE", ctx.language)
            .env("DRACO_LINE", (ctx.line + 1).to_string())
            .env("DRACO_COL", (ctx.col + 1).to_string());
        if let Some(fp) = ctx.file_path {
            command.env("DRACO_FILE_PATH", fp);
        }
        if let Some(root) = ctx.workspace_root {
            command.env("DRACO_WORKSPACE_ROOT", root);
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn '{}': {e}", cmd_spec.executable))?;

        if cmd_spec.input_mode != PluginInputMode::None {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(ctx.stdin_text.as_bytes());
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Plugin execution failed: {e}"))?;

        Ok(PluginExecResult {
            output_mode: cmd_spec.output_mode,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_reload_execute_and_uninstall_plugin() {
        let base = std::env::temp_dir().join(format!("draco_plug_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let src_dir = base.join("src_plugin");
        let plugins_dir = base.join("installed");
        fs::create_dir_all(&src_dir).unwrap();

        let manifest = r#"{
            "id": "upper-demo",
            "name": "Uppercase Demo",
            "version": "1.0.0",
            "description": "Transforms input to uppercase",
            "author": "Draco",
            "commands": [{
                "id": "to_upper",
                "title": "Convert to UPPERCASE",
                "executable": "tr",
                "args": ["a-z", "A-Z"],
                "input_mode": "document",
                "output_mode": "replace_document"
            }]
        }"#;
        fs::write(src_dir.join("plugin.json"), manifest).unwrap();

        let settings = Settings::default();
        let mut mgr = PluginManager::new(plugins_dir);
        let installed = mgr.install_from_dir(&src_dir, &settings).unwrap();
        assert_eq!(installed.id, "upper-demo");
        assert_eq!(mgr.all_enabled_commands().len(), 1);

        let res = mgr
            .execute_command(
                "upper-demo",
                "to_upper",
                &PluginContext {
                    file_path: None,
                    workspace_root: None,
                    language: "Rust",
                    line: 0,
                    col: 0,
                    stdin_text: "hello draco".to_string(),
                },
            )
            .unwrap();
        assert!(res.success);
        assert_eq!(res.stdout, "HELLO DRACO");
        assert_eq!(res.output_mode, PluginOutputMode::ReplaceDocument);

        mgr.uninstall("upper-demo", &settings).unwrap();
        assert!(mgr.plugins.is_empty());
        let _ = fs::remove_dir_all(&base);
    }
}
