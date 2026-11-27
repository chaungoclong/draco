use draco::core::selection::{Selection, SelectionSet};

#[test]
fn selection_properties_forward_and_reversed() {
    let fwd = Selection::new(3, 10);
    assert_eq!(fwd.start(), 3);
    assert_eq!(fwd.end(), 10);
    assert!(!fwd.is_empty());
    assert!(!fwd.is_reversed());

    let rev = Selection::new(10, 3);
    assert_eq!(rev.start(), 3);
    assert_eq!(rev.end(), 10);
    assert!(rev.is_reversed());
    assert_eq!(rev.range(), 3..10);
}

#[test]
fn merges_multiple_overlapping_ranges_and_preserves_order() {
    let set = SelectionSet::from_vec(
        vec![
            Selection::new(20, 25),
            Selection::new(2, 8),
            Selection::new(6, 12),
            Selection::new(11, 15),
        ],
        0,
        100,
    );
    assert_eq!(set.len(), 2);
    assert_eq!(set.selections()[0], Selection::new(2, 15));
    assert_eq!(set.selections()[1], Selection::new(20, 25));
    assert_eq!(set.primary(), Selection::new(20, 25));
}

#[test]
fn empty_vec_falls_back_to_caret_zero_and_clamps_out_of_bounds() {
    let empty = SelectionSet::from_vec(vec![], 5, 10);
    assert_eq!(empty.len(), 1);
    assert_eq!(empty.primary(), Selection::caret(0));

    let mut set = SelectionSet::single(Selection::new(500, 1000));
    set.normalize(42);
    assert_eq!(set.primary(), Selection::caret(42));
}

#[test]
fn collapse_to_primary_keeps_head_position() {
    let mut set = SelectionSet::single(Selection::new(1, 5));
    set.add(Selection::new(20, 12), 50); // reversed primary, head = 12
    assert_eq!(set.len(), 2);
    set.collapse_to_primary();
    assert_eq!(set.len(), 1);
    assert_eq!(set.primary(), Selection::caret(12));
}
