use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TerminalLayout {
    #[default]
    Bottom,
    Right,
    Floating,
    Maximized,
}


impl TerminalLayout {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bottom => "Bottom",
            Self::Right => "Right",
            Self::Floating => "Floating",
            Self::Maximized => "Maximized",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub font_size: f32,
    pub tab_size: usize,
    pub auto_save: bool,
    pub auto_save_delay_ms: u64,
    pub show_indent_guides: bool,
    pub terminal_layout: TerminalLayout,
    pub disabled_plugins: Vec<String>,
    pub last_workspace: Option<PathBuf>,
    pub recent_workspaces: Vec<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14.5,
            tab_size: 4,
            auto_save: true, // Auto-save ON by default
            auto_save_delay_ms: 800,
            show_indent_guides: true,
            terminal_layout: TerminalLayout::Bottom,
            disabled_plugins: Vec::new(),
            last_workspace: None,
            recent_workspaces: Vec::new(),
        }
    }
}

impl Settings {
    pub fn config_dir() -> PathBuf {
        if let Some(proj) = directories::ProjectDirs::from("dev", "draco", "draco") {
            return proj.config_dir().to_path_buf();
        }
        std::env::temp_dir().join("draco_config")
    }

    pub fn default_path() -> PathBuf {
        Self::config_dir().join("settings.json")
    }

    pub fn plugins_dir() -> PathBuf {
        Self::config_dir().join("plugins")
    }

    pub fn load_from(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn load_default() -> Self {
        Self::load_from(&Self::default_path())
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        fs::write(path, json)
    }

    pub fn save_default(&self) -> std::io::Result<()> {
        self.save_to(&Self::default_path())
    }

    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        !self.disabled_plugins.iter().any(|id| id == plugin_id)
    }

    pub fn set_plugin_enabled(&mut self, plugin_id: &str, enabled: bool) {
        if enabled {
            self.disabled_plugins.retain(|id| id != plugin_id);
        } else if !self.disabled_plugins.iter().any(|id| id == plugin_id) {
            self.disabled_plugins.push(plugin_id.to_string());
        }
    }

    /// Records a workspace folder as the active `last_workspace` and pushes it to the top of `recent_workspaces` (MRU, max 10).
    pub fn record_recent_workspace(&mut self, path: PathBuf) {
        self.last_workspace = Some(path.clone());
        self.recent_workspaces.retain(|p| p != &path);
        self.recent_workspaces.insert(0, path);
        if self.recent_workspaces.len() > 10 {
            self.recent_workspaces.truncate(10);
        }
    }

    /// Removes a workspace folder from `recent_workspaces` (and clears `last_workspace` if it matches).
    pub fn remove_recent_workspace(&mut self, path: &Path) {
        self.recent_workspaces.retain(|p| p.as_path() != path);
        if self.last_workspace.as_deref() == Some(path) {
            self.last_workspace = self.recent_workspaces.first().cloned();
        }
    }

    /// Returns `last_workspace` only if it still exists on disk as a directory.
    pub fn valid_last_workspace(&self) -> Option<PathBuf> {
        self.last_workspace.clone().filter(|p| p.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_defaults_auto_save_on_and_roundtrips_json() {
        let mut s = Settings::default();
        assert!(s.auto_save);
        assert_eq!(s.terminal_layout, TerminalLayout::Bottom);

        s.auto_save = false;
        s.font_size = 16.0;
        s.terminal_layout = TerminalLayout::Right;
        s.set_plugin_enabled("my-plugin", false);
        assert!(!s.is_plugin_enabled("my-plugin"));

        let path = std::env::temp_dir().join(format!(
            "draco_settings_test_{}.json",
            std::process::id()
        ));
        s.save_to(&path).unwrap();
        let loaded = Settings::load_from(&path);
        assert_eq!(loaded, s);
        let _ = fs::remove_file(path);
    }
}
