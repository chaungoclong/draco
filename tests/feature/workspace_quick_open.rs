use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use draco::workspace::file_tree::FileTree;
use draco::workspace::fuzzy::WorkspaceIndexer;

use crate::common::TempDir;

#[test]
fn workspace_tree_respects_gitignore_and_ranks_quick_open() {
    let tmp = TempDir::new("feat_workspace");
    tmp.write_file(".git/HEAD", "ref: refs/heads/main\n");
    tmp.write_file(".gitignore", "*.log\nignored_dir/\n");
    tmp.write_file("ignored_dir/secret.rs", "fn secret() {}");
    tmp.write_file("debug.log", "noisy");
    tmp.write_file("src/main.rs", "fn main() {}");
    tmp.write_file("src/core/buffer.rs", "pub struct Buffer;");
    tmp.write_file("public/index.php", "<?php echo 1;");
    tmp.write_file("README.md", "# Draco");

    // 1. Lazy FileTree hides .git and sorts directories first
    let mut tree = FileTree::open(tmp.path().to_path_buf());
    let root_children = tree.root_node.children.as_mut().unwrap();
    assert!(!root_children.iter().any(|n| n.name == ".git"));
    assert!(root_children[0].is_dir);

    let src_node = root_children.iter_mut().find(|n| n.name == "src").unwrap();
    assert!(!src_node.expanded);
    assert!(src_node.children.is_none());
    src_node.toggle();
    assert!(src_node.expanded);
    assert!(src_node
        .children
        .as_ref()
        .unwrap()
        .iter()
        .any(|n| n.name == "main.rs"));

    // 2. Background WorkspaceIndexer respects .gitignore
    let indexer = WorkspaceIndexer::default();
    let done = Arc::new(AtomicBool::new(false));
    let done_cb = Arc::clone(&done);
    indexer.start_index(
        tmp.path().to_path_buf(),
        Arc::new(move || done_cb.store(true, Ordering::SeqCst)),
    );

    let deadline = Instant::now() + Duration::from_secs(3);
    while !done.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(15));
    }
    assert!(done.load(Ordering::SeqCst), "Indexer timed out");

    let all = indexer.query(tmp.path(), "", 50);
    assert!(!all
        .iter()
        .any(|(_, rel, _)| rel.contains("debug.log") || rel.contains("ignored_dir")));

    let buf_results = indexer.query(tmp.path(), "buf", 10);
    assert_eq!(buf_results[0].1, "src/core/buffer.rs");

    let php_results = indexer.query(tmp.path(), "index.php", 10);
    assert_eq!(php_results[0].1, "public/index.php");
}

#[test]
fn switching_workspace_folder_clears_previous_indexed_files() {
    let dir_a = TempDir::new("feat_ws_switch_a");
    dir_a.write_file("old_alpha.rs", "fn old() {}");
    let dir_b = TempDir::new("feat_ws_switch_b");
    dir_b.write_file("new_beta.rs", "fn new_fn() {}");

    let indexer = WorkspaceIndexer::default();

    // Index Folder A
    let done_a = Arc::new(AtomicBool::new(false));
    let done_a_cb = Arc::clone(&done_a);
    indexer.start_index(
        dir_a.path().to_path_buf(),
        Arc::new(move || done_a_cb.store(true, Ordering::SeqCst)),
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !done_a.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(indexer.query(dir_a.path(), "old_alpha", 10).len(), 1);

    // Switch to Folder B -> must NOT retain any files from Folder A
    let done_b = Arc::new(AtomicBool::new(false));
    let done_b_cb = Arc::clone(&done_b);
    indexer.start_index(
        dir_b.path().to_path_buf(),
        Arc::new(move || done_b_cb.store(true, Ordering::SeqCst)),
    );
    let deadline_b = Instant::now() + Duration::from_secs(3);
    while !done_b.load(Ordering::SeqCst) && Instant::now() < deadline_b {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(done_b.load(Ordering::SeqCst));

    let res_b = indexer.query(dir_b.path(), "", 20);
    assert_eq!(res_b.len(), 1);
    assert_eq!(res_b[0].1, "new_beta.rs");
    assert!(indexer.query(dir_b.path(), "old_alpha", 10).is_empty());
}
