use draco::lsp::client::parse_workspace_edit;
use draco::workspace::git_diff::{parse_unified_diff_hunks, GitLineChange};
use serde_json::json;

#[test]
fn parses_git_unified_diff_added_modified_and_deleted_hunks() {
    let diff = "\
diff --git a/src/main.rs b/src/main.rs
index 1111111..2222222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -2,0 +3,2 @@ fn main() {
+    let a = 1;
+    let b = 2;
@@ -10 +12 @@ fn helper() {
-    old_line();
+    new_line();
@@ -20,2 +21,0 @@ fn end() {
-    removed1();
-    removed2();
";

    let map = parse_unified_diff_hunks(diff);
    assert_eq!(map.get(&2), Some(&GitLineChange::Added));
    assert_eq!(map.get(&3), Some(&GitLineChange::Added));
    assert_eq!(map.get(&11), Some(&GitLineChange::Modified));
    assert_eq!(map.get(&20), Some(&GitLineChange::Deleted));
    assert_eq!(map.len(), 4);
}

#[test]
fn parses_lsp_workspace_edit_changes_and_document_changes() {
    let changes_val = json!({
        "changes": {
            "file:///tmp/demo.rs": [
                {
                    "range": {
                        "start": { "line": 1, "character": 4 },
                        "end": { "line": 1, "character": 7 }
                    },
                    "newText": "renamed_var"
                }
            ]
        }
    });
    let parsed = parse_workspace_edit(&changes_val);
    let edits = parsed.get(&std::path::PathBuf::from("/tmp/demo.rs")).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].start_line, 1);
    assert_eq!(edits[0].start_col, 4);
    assert_eq!(edits[0].end_col, 7);
    assert_eq!(edits[0].new_text, "renamed_var");

    let doc_changes_val = json!({
        "documentChanges": [
            {
                "textDocument": { "uri": "file:///tmp/other.rs", "version": 1 },
                "edits": [
                    {
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 3 }
                        },
                        "newText": "pub"
                    }
                ]
            }
        ]
    });
    let parsed2 = parse_workspace_edit(&doc_changes_val);
    let edits2 = parsed2
        .get(&std::path::PathBuf::from("/tmp/other.rs"))
        .unwrap();
    assert_eq!(edits2.len(), 1);
    assert_eq!(edits2[0].new_text, "pub");
}
