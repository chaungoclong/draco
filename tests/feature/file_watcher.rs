use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use draco::core::buffer::Document;
use draco::workspace::watcher::{WatchEvent, WorkspaceWatcher};

use crate::common::TempDir;

#[test]
fn workspace_watcher_detects_external_modify_and_reloads_document() {
    let tmp = TempDir::new("feat_watcher");
    let file_path = tmp.write_file("watched.rs", "fn v1() {}\n");

    let mut doc = Document::from_file(1, file_path.clone()).unwrap();
    let repainted = Arc::new(AtomicBool::new(false));
    let repainted_cb = Arc::clone(&repainted);

    let watcher = WorkspaceWatcher::watch(
        tmp.path(),
        Arc::new(move || repainted_cb.store(true, Ordering::SeqCst)),
    )
    .expect("start watcher");

    // Modify the file externally on disk
    std::thread::sleep(Duration::from_millis(40));
    fs::write(&file_path, "fn v2_external() {}\n").unwrap();

    let mut saw_modify = false;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline && !saw_modify {
        for ev in watcher.poll_events() {
            if let WatchEvent::FileModified(p) = ev
                && fs::canonicalize(&p).ok() == fs::canonicalize(&file_path).ok() {
                    saw_modify = true;
                }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_modify, "Expected FileModified event from notify");
    assert!(repainted.load(Ordering::SeqCst));

    // Reload updates document rope & bumps version; second reload is a no-op
    let v_before = doc.version;
    assert!(doc.reload_from_disk().unwrap());
    assert_eq!(doc.rope.to_string(), "fn v2_external() {}\n");
    assert_eq!(doc.version, v_before + 1);
    assert!(!doc.dirty);
    assert!(!doc.reload_from_disk().unwrap());

    // Creating a new file triggers TreeChanged
    tmp.write_file("new_file.php", "<?php echo 1;");
    let mut saw_tree = false;
    let deadline2 = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline2 && !saw_tree {
        for ev in watcher.poll_events() {
            if matches!(ev, WatchEvent::TreeChanged) {
                saw_tree = true;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_tree, "Expected TreeChanged event when creating a file");
}
