use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitLineChange {
    Added,
    Modified,
    Deleted,
}

/// Parses `git diff -U0 --no-color` unified diff hunk headers into 0-indexed line change markers.
pub fn parse_unified_diff_hunks(diff: &str) -> HashMap<usize, GitLineChange> {
    let mut map = HashMap::new();
    for line in diff.lines() {
        let Some(rest) = line.strip_prefix("@@ ") else {
            continue;
        };
        let Some(header) = rest.split(" @@").next() else {
            continue;
        };
        let mut parts = header.split_whitespace();
        let Some(old_part) = parts.next().and_then(|s| s.strip_prefix('-')) else {
            continue;
        };
        let Some(new_part) = parts.next().and_then(|s| s.strip_prefix('+')) else {
            continue;
        };

        let (_, old_count) = parse_hunk_range(old_part);
        let (new_start, new_count) = parse_hunk_range(new_part);

        if old_count == 0 && new_count > 0 {
            let start = new_start.saturating_sub(1);
            for l in start..start + new_count {
                map.insert(l, GitLineChange::Added);
            }
        } else if old_count > 0 && new_count == 0 {
            let marker_line = new_start.saturating_sub(1);
            map.entry(marker_line).or_insert(GitLineChange::Deleted);
        } else if old_count > 0 && new_count > 0 {
            let start = new_start.saturating_sub(1);
            for l in start..start + new_count {
                map.insert(l, GitLineChange::Modified);
            }
        }
    }
    map
}

fn parse_hunk_range(spec: &str) -> (usize, usize) {
    if let Some((a, b)) = spec.split_once(',') {
        (a.parse().unwrap_or(0), b.parse().unwrap_or(0))
    } else {
        (spec.parse().unwrap_or(0), 1)
    }
}

/// Computes Git gutter diff markers for a single file on disk using `git diff -U0`.
pub fn compute_git_line_changes(path: &Path) -> HashMap<usize, GitLineChange> {
    let Some(parent) = path.parent() else {
        return HashMap::new();
    };
    let Some(file_name) = path.file_name() else {
        return HashMap::new();
    };

    let output = Command::new("git")
        .arg("-C")
        .arg(parent)
        .args(["diff", "-U0", "--no-color", "--"])
        .arg(file_name)
        .output();

    let Ok(out) = output else {
        return HashMap::new();
    };
    if !out.status.success() {
        return HashMap::new();
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_unified_diff_hunks(&stdout)
}

/// Non-blocking background Git diff cache per file path.
#[derive(Default)]
pub struct GitDiffTracker {
    cache: Arc<Mutex<HashMap<PathBuf, HashMap<usize, GitLineChange>>>>,
}

impl GitDiffTracker {
    pub fn clear(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.clear();
        }
    }

    pub fn request_refresh(&self, path: PathBuf, repaint: Arc<dyn Fn() + Send + Sync>) {
        let cache = Arc::clone(&self.cache);
        thread::spawn(move || {
            let changes = compute_git_line_changes(&path);
            if let Ok(mut guard) = cache.lock() {
                guard.insert(path, changes);
            }
            repaint();
        });
    }

    pub fn changes_for(&self, path: &Path) -> HashMap<usize, GitLineChange> {
        self.cache
            .lock()
            .ok()
            .and_then(|g| g.get(path).cloned())
            .unwrap_or_default()
    }
}
