use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    FileModified(PathBuf),
    TreeChanged,
}

fn should_ignore_path(path: &Path) -> bool {
    path.components().any(|c| {
        let s = c.as_os_str();
        s == ".git" || s == "target" || s == "node_modules"
    })
}

/// Cross-platform OS file watcher (`inotify` / `FSEvents` / `ReadDirectoryChangesW`) powered by `notify`.
pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<WatchEvent>,
}

impl WorkspaceWatcher {
    pub fn watch(root: &Path, repaint: Arc<dyn Fn() + Send + Sync>) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<WatchEvent>();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                let Ok(event) = res else {
                    return;
                };
                if event.paths.iter().all(|p| should_ignore_path(p)) {
                    return;
                }
                let mut sent = false;
                match event.kind {
                    EventKind::Modify(_) => {
                        for p in event.paths {
                            if !should_ignore_path(&p) && p.is_file() {
                                let _ = tx.send(WatchEvent::FileModified(p));
                                sent = true;
                            }
                        }
                    }
                    EventKind::Create(_) | EventKind::Remove(_) => {
                        let _ = tx.send(WatchEvent::TreeChanged);
                        sent = true;
                    }
                    _ => {}
                }
                if sent {
                    repaint();
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create file watcher: {e}"))?;

        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch {}: {e}", root.display()))?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn poll_events(&self) -> Vec<WatchEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }
}
