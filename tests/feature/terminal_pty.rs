use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use draco::settings::TerminalLayout;
use draco::terminal::manager::TerminalManager;
use draco::terminal::pty::PtySession;

#[test]
fn pty_shell_executes_command_and_resizes() {
    let repainted = Arc::new(AtomicBool::new(false));
    let repainted_cb = Arc::clone(&repainted);

    let mut pty = PtySession::spawn(
        None,
        80,
        20,
        Arc::new(move || repainted_cb.store(true, Ordering::SeqCst)),
    )
    .expect("spawn PTY");

    pty.write_bytes(b"echo DRACO_PTY_VERIFIED\n");

    let deadline = Instant::now() + Duration::from_secs(4);
    let mut found = false;
    while Instant::now() < deadline {
        if let Ok(grid) = pty.grid.lock()
            && (0..grid.rows).any(|r| grid.row_text(r).contains("DRACO_PTY_VERIFIED")) {
                found = true;
                break;
            }
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(found, "PTY output did not contain DRACO_PTY_VERIFIED");
    assert!(repainted.load(Ordering::SeqCst));

    pty.resize(110, 28);
    let grid = pty.grid.lock().unwrap();
    assert_eq!(grid.cols, 110);
    assert_eq!(grid.rows, 28);
}

#[test]
fn multi_tab_terminal_manager_lifecycle_and_all_layouts() {
    let mut mgr = TerminalManager::new(TerminalLayout::Bottom);
    assert!(mgr.is_empty());

    let t0 = mgr.spawn_tab(None, Arc::new(|| {})).unwrap();
    let t1 = mgr.spawn_tab(None, Arc::new(|| {})).unwrap();
    let t2 = mgr.spawn_tab(None, Arc::new(|| {})).unwrap();
    assert_eq!((t0, t1, t2), (0, 1, 2));
    assert_eq!(mgr.len(), 3);
    assert_eq!(mgr.active, 2);

    mgr.select_tab(1);
    assert_eq!(mgr.active, 1);

    for layout in [
        TerminalLayout::Bottom,
        TerminalLayout::Right,
        TerminalLayout::Floating,
        TerminalLayout::Maximized,
    ] {
        mgr.layout = layout;
        assert_eq!(mgr.layout, layout);
    }

    mgr.close_tab(1);
    assert_eq!(mgr.len(), 2);
    assert_eq!(mgr.active, 1);

    mgr.close_tab(1);
    mgr.close_tab(0);
    assert!(mgr.is_empty());
}
