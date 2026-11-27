use ropey::Rope;
use tree_sitter::Tree;

use crate::core::selection::Selection;

/// Tracks previous selections during AST Expand (`Shift+Alt+Right`) so
/// AST Shrink (`Shift+Alt+Left`) can unwind step-by-step like JetBrains / VS Code.
#[derive(Debug, Default, Clone)]
pub struct AstSelectionHistory {
    stack: Vec<Selection>,
    last_expanded: Option<Selection>,
}

impl AstSelectionHistory {
    pub fn clear(&mut self) {
        self.stack.clear();
        self.last_expanded = None;
    }

    /// Expands `current` to the next larger enclosing Tree-sitter AST node.
    pub fn expand(&mut self, tree: &Tree, rope: &Rope, current: Selection) -> Option<Selection> {
        if self.last_expanded != Some(current) {
            self.stack.clear();
        }
        let next = extend_selection(tree, rope, current)?;
        self.stack.push(current);
        self.last_expanded = Some(next);
        Some(next)
    }

    /// Shrinks back to the previous selection in the AST expansion stack.
    pub fn shrink(&mut self, current: Selection) -> Option<Selection> {
        if self.last_expanded != Some(current) {
            self.clear();
            return None;
        }
        let prev = self.stack.pop()?;
        self.last_expanded = if self.stack.is_empty() {
            None
        } else {
            Some(prev)
        };
        Some(prev)
    }
}

/// Finds the smallest enclosing Tree-sitter AST node strictly larger than `current`.
pub fn extend_selection(tree: &Tree, rope: &Rope, current: Selection) -> Option<Selection> {
    let len_chars = rope.len_chars();
    if len_chars == 0 {
        return None;
    }

    let range = current.range();
    let start_char = range.start.min(len_chars);
    let end_char = range.end.min(len_chars);
    let start_byte = rope.char_to_byte(start_char);
    let end_byte = rope.char_to_byte(end_char);

    let root = tree.root_node();
    let mut node = if start_byte == end_byte {
        let probe = if start_byte >= root.end_byte() && root.end_byte() > 0 {
            root.end_byte() - 1
        } else {
            start_byte
        };
        root.descendant_for_byte_range(probe, probe)?
    } else {
        root.descendant_for_byte_range(start_byte, end_byte)?
    };

    loop {
        let nr = node.byte_range();
        let strictly_larger =
            (nr.start < start_byte || nr.end > end_byte) && nr.end > nr.start;
        if strictly_larger {
            let new_start = rope.byte_to_char(nr.start.min(rope.len_bytes()));
            let new_end = rope.byte_to_char(nr.end.min(rope.len_bytes()));
            if new_start != start_char || new_end != end_char {
                return Some(Selection::new(new_start, new_end));
            }
        }
        match node.parent() {
            Some(p) => node = p,
            None => return None,
        }
    }
}
