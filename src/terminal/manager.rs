use std::path::Path;
use std::sync::Arc;

use crate::settings::TerminalLayout;
use crate::terminal::pty::PtySession;

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    pub session: PtySession,
}

pub struct TerminalManager {
    pub tabs: Vec<TerminalTab>,
    pub active: usize,
    pub layout: TerminalLayout,
    next_id: u64,
}

impl TerminalManager {
    pub fn new(layout: TerminalLayout) -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            layout,
            next_id: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn spawn_tab(
        &mut self,
        cwd: Option<&Path>,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<usize, String> {
        let id = self.next_id;
        let session = PtySession::spawn(cwd, 100, 20, repaint)?;
        self.next_id += 1;
        let title = format!("Terminal {id}");
        self.tabs.push(TerminalTab { id, title, session });
        self.active = self.tabs.len() - 1;
        Ok(self.active)
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(idx);
        if self.tabs.is_empty() {
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    pub fn select_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active = idx;
        }
    }

    pub fn active_session_mut(&mut self) -> Option<&mut PtySession> {
        self.tabs.get_mut(self.active).map(|t| &mut t.session)
    }
}
