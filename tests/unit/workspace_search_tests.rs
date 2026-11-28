use draco::ui::code_view::overview_marker_y;
use draco::workspace::search::{
    replace_in_workspace, search_workspace_sync, WorkspaceSearchOptions,
};

use crate::common::TempDir;

#[test]
fn workspace_search_respects_gitignore_filters_and_utf8_cols() {
    let tmp = TempDir::new("ws_search");
    tmp.write_file(".gitignore", "ignored_dir/\n");
    tmp.write_file("src/main.rs", "fn main() {\n    let s = \"Xin chào Draco\";\n}\n");
    tmp.write_file("src/lib.rs", "pub fn draco_init() {}\n");
    tmp.write_file("notes.txt", "draco in plain text\n");
    tmp.write_file("ignored_dir/secret.rs", "fn draco_hidden() {}\n");

    // Default case-insensitive search skips .gitignore
    let opts = WorkspaceSearchOptions::default();
    let (matches, truncated) = search_workspace_sync(tmp.path(), "draco", &opts, 100).unwrap();
    assert!(!truncated);
    assert_eq!(matches.len(), 3);
    assert!(!matches.iter().any(|m| m.rel_path.contains("ignored_dir")));

    // Verify UTF-8 char column on Vietnamese line
    let main_m = matches.iter().find(|m| m.rel_path == "src/main.rs").unwrap();
    assert_eq!(main_m.line, 1);
    assert_eq!(main_m.col, "    let s = \"Xin chào ".chars().count());

    // Include filter `*.rs` excludes `notes.txt`
    let rs_opts = WorkspaceSearchOptions {
        include_filter: "*.rs".into(),
        ..Default::default()
    };
    let (rs_matches, _) = search_workspace_sync(tmp.path(), "draco", &rs_opts, 100).unwrap();
    assert_eq!(rs_matches.len(), 2);

    // Exclude filter `lib.rs` leaves only `src/main.rs`
    let exc_opts = WorkspaceSearchOptions {
        include_filter: "*.rs".into(),
        exclude_filter: "lib.rs".into(),
        ..Default::default()
    };
    let (exc_matches, _) = search_workspace_sync(tmp.path(), "draco", &exc_opts, 100).unwrap();
    assert_eq!(exc_matches.len(), 1);
    assert_eq!(exc_matches[0].rel_path, "src/main.rs");
}

#[test]
fn workspace_replace_updates_matching_files_and_skips_ignored() {
    let tmp = TempDir::new("ws_replace");
    tmp.write_file(".gitignore", "build/\n");
    tmp.write_file("a.rs", "let old_name = 1;\nlet old_name_2 = old_name;\n");
    tmp.write_file("b.rs", "fn old_name() {}\n");
    tmp.write_file("build/out.rs", "let old_name = 99;\n");

    let opts = WorkspaceSearchOptions {
        whole_word: true,
        case_sensitive: true,
        ..Default::default()
    };
    let sum = replace_in_workspace(tmp.path(), "old_name", "new_name", &opts).unwrap();
    assert_eq!(sum.files_changed, 2);
    assert_eq!(sum.replacements, 3);

    let a_text = std::fs::read_to_string(tmp.path().join("a.rs")).unwrap();
    assert_eq!(a_text, "let new_name = 1;\nlet old_name_2 = new_name;\n");
    let ignored_text = std::fs::read_to_string(tmp.path().join("build/out.rs")).unwrap();
    assert_eq!(ignored_text, "let old_name = 99;\n");
}

#[test]
fn overview_ruler_marker_y_maps_bounds_accurately() {
    assert_eq!(overview_marker_y(0, 100, 10.0, 200.0), 10.0);
    assert!((overview_marker_y(99, 100, 10.0, 200.0) - 207.0).abs() < 0.01);
    assert_eq!(overview_marker_y(0, 1, 20.0, 100.0), 20.0);
}
