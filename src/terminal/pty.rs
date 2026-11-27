use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

use crate::terminal::grid::TerminalGrid;

pub struct PtySession {
    pub grid: Arc<Mutex<TerminalGrid>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    last_cols: u16,
    last_rows: u16,
}

impl PtySession {
    pub fn spawn(
        cwd: Option<&Path>,
        cols: u16,
        rows: u16,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: rows.max(4),
            cols: cols.max(10),
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system
            .openpty(size)
            .map_err(|e| format!("openpty failed: {e}"))?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("spawn shell failed: {e}"))?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("clone pty reader failed: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("take pty writer failed: {e}"))?;

        let grid = Arc::new(Mutex::new(TerminalGrid::new(
            size.cols as usize,
            size.rows as usize,
        )));
        let grid_bg = Arc::clone(&grid);

        thread::spawn(move || {
            let mut parser = vte::Parser::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if let Ok(mut g) = grid_bg.lock() {
                            parser.advance(&mut *g, &buf[..n]);
                        }
                        repaint();
                    }
                }
            }
        });

        Ok(Self {
            grid,
            writer,
            master: pair.master,
            last_cols: size.cols,
            last_rows: size.rows,
        })
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(10);
        let rows = rows.max(4);
        if cols == self.last_cols && rows == self.last_rows {
            return;
        }
        self.last_cols = cols;
        self.last_rows = rows;
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        if let Ok(mut g) = self.grid.lock() {
            g.resize(cols as usize, rows as usize);
        }
    }
}
