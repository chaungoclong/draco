use draco::workspace::file_tree::FileTree;
use draco::workspace::fuzzy::fuzzy_score;

use crate::common::TempDir;

#[test]
fn file_tree_hides_git_sorts_dirs_first_and_lazy_loads() {
    let tmp = TempDir::new("unit_tree");
    tmp.write_file(".git/HEAD", "ref: refs/heads/main");
    tmp.write_file("z_file.txt", "");
    tmp.write_file("a_file.rs", "");
    tmp.write_file("b_dir/nested.php", "<?php");
    tmp.write_file("a_dir/sub.rs", "");

    let mut tree = FileTree::open(tmp.path().to_path_buf());
    let children = tree.root_node.children.as_mut().unwrap();
    let names: Vec<&str> = children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["a_dir", "b_dir", "a_file.rs", "z_file.txt"]);

    // Lazy load: directory children are None until toggled
    assert!(children[0].children.is_none());
    assert!(!children[0].expanded);
    children[0].toggle();
    assert!(children[0].expanded);
    assert_eq!(children[0].children.as_ref().unwrap()[0].name, "sub.rs");
}

#[test]
fn fuzzy_score_edge_cases_and_ranking() {
    assert_eq!(fuzzy_score("", "anything.rs"), Some(0));
    assert!(fuzzy_score("xyz", "src/main.rs").is_none());

    let exact_file = fuzzy_score("main", "src/main.rs").unwrap();
    let scattered = fuzzy_score("main", "m/a_dir/i_dir/n_file.rs").unwrap();
    assert!(exact_file > scattered);

    // Acronym match across separators
    let acronym = fuzzy_score("cv", "src/ui/code_view.rs");
    assert!(acronym.is_some());
}
