use std::ops::Range;

/// A single cursor or text selection using char offsets into a `ropey::Rope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn caret(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub fn start(self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }

    pub fn range(self) -> Range<usize> {
        self.start()..self.end()
    }

    pub fn is_reversed(self) -> bool {
        self.head < self.anchor
    }

    pub fn clamp(self, max_len: usize) -> Self {
        Self {
            anchor: self.anchor.min(max_len),
            head: self.head.min(max_len),
        }
    }
}

/// Ordered, non-overlapping set of selections (supports multi-cursor editing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary: usize,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::single(Selection::caret(0))
    }
}

impl SelectionSet {
    pub fn single(sel: Selection) -> Self {
        Self {
            selections: vec![sel],
            primary: 0,
        }
    }

    pub fn from_vec(mut selections: Vec<Selection>, primary: usize, max_len: usize) -> Self {
        if selections.is_empty() {
            selections.push(Selection::caret(0));
        }
        let mut set = Self {
            selections,
            primary: primary.min(100_000),
        };
        set.normalize(max_len);
        set
    }

    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }

    pub fn primary(&self) -> Selection {
        self.selections[self.primary.min(self.selections.len() - 1)]
    }

    pub fn primary_index(&self) -> usize {
        self.primary.min(self.selections.len() - 1)
    }

    pub fn set_single(&mut self, sel: Selection, max_len: usize) {
        self.selections.clear();
        self.selections.push(sel.clamp(max_len));
        self.primary = 0;
    }

    pub fn add(&mut self, sel: Selection, max_len: usize) {
        let clamped = sel.clamp(max_len);
        self.selections.push(clamped);
        self.primary = self.selections.len() - 1;
        self.normalize(max_len);
    }

    pub fn collapse_to_primary(&mut self) {
        let p = self.primary();
        self.selections.clear();
        self.selections.push(Selection::caret(p.head));
        self.primary = 0;
    }

    /// Clamps, sorts, and merges overlapping selections while tracking the primary cursor.
    pub fn normalize(&mut self, max_len: usize) {
        if self.selections.is_empty() {
            self.selections.push(Selection::caret(0));
            self.primary = 0;
            return;
        }
        let primary_idx = self.primary.min(self.selections.len() - 1);
        let mut tagged: Vec<(Selection, bool)> = self
            .selections
            .iter()
            .enumerate()
            .map(|(i, &s)| (s.clamp(max_len), i == primary_idx))
            .collect();

        tagged.sort_by_key(|(s, _)| (s.start(), s.end()));

        let mut merged: Vec<(Selection, bool)> = Vec::with_capacity(tagged.len());
        for (sel, is_prim) in tagged {
            if let Some((last, last_prim)) = merged.last_mut() {
                let overlaps = if last.is_empty() && sel.is_empty() {
                    last.start() == sel.start()
                } else {
                    sel.start() < last.end()
                        || (sel.start() == last.end() && (last.is_empty() || sel.is_empty()))
                };
                if overlaps {
                    let new_start = last.start().min(sel.start());
                    let new_end = last.end().max(sel.end());
                    let reversed = if is_prim {
                        sel.is_reversed()
                    } else {
                        last.is_reversed()
                    };
                    *last = if reversed {
                        Selection::new(new_end, new_start)
                    } else {
                        Selection::new(new_start, new_end)
                    };
                    *last_prim = *last_prim || is_prim;
                    continue;
                }
            }
            merged.push((sel, is_prim));
        }

        self.primary = merged.iter().position(|(_, p)| *p).unwrap_or(0);
        self.selections = merged.into_iter().map(|(s, _)| s).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_overlapping_selections_and_tracks_primary() {
        let mut set = SelectionSet::single(Selection::new(2, 6));
        set.add(Selection::new(5, 10), 20);
        set.add(Selection::caret(15), 20);
        assert_eq!(set.len(), 2);
        assert_eq!(set.selections()[0], Selection::new(2, 10));
        assert_eq!(set.selections()[1], Selection::caret(15));
        assert_eq!(set.primary(), Selection::caret(15));
    }

    #[test]
    fn deduplicates_identical_carets() {
        let mut set = SelectionSet::single(Selection::caret(4));
        set.add(Selection::caret(4), 10);
        assert_eq!(set.len(), 1);
        assert_eq!(set.primary(), Selection::caret(4));
    }

    #[test]
    fn clamps_and_collapses_reversed_selections() {
        let mut set = SelectionSet::from_vec(
            vec![Selection::new(50, 2), Selection::new(100, 90)],
            1,
            20,
        );
        assert_eq!(set.len(), 1);
        assert_eq!(set.primary().start(), 2);
        assert_eq!(set.primary().end(), 20);
        set.collapse_to_primary();
        assert!(set.primary().is_empty());
    }
}
