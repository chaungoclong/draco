use draco::core::buffer::Document;
use draco::core::search::{self, SearchOptions};
use draco::core::LanguageId;

#[test]
fn empty_query_returns_no_matches_and_zero_replacements() {
    let mut doc = Document::from_str(1, "hello world", LanguageId::PlainText);
    let opts = SearchOptions::default();
    assert!(search::find_all(&doc.rope, "", &opts).unwrap().is_empty());
    assert_eq!(search::replace_all(&mut doc, "", "x", &opts).unwrap(), 0);
}

#[test]
fn case_sensitive_vs_insensitive_and_whole_word() {
    let rope = ropey::Rope::from_str("cat Catalog CAT scatter cat");

    let ci = search::find_all(
        &rope,
        "cat",
        &SearchOptions {
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
        },
    )
    .unwrap();
    assert_eq!(ci.len(), 5);

    let cs_ww = search::find_all(
        &rope,
        "cat",
        &SearchOptions {
            case_sensitive: true,
            whole_word: true,
            use_regex: false,
        },
    )
    .unwrap();
    assert_eq!(cs_ww.len(), 2);
}

#[test]
fn utf8_regex_search_and_atomic_replace_undo() {
    let mut doc = Document::from_str(
        1,
        "Giá: 100đ, Tổng: 250đ, Giảm: 50đ",
        LanguageId::PlainText,
    );
    let opts = SearchOptions {
        case_sensitive: true,
        whole_word: false,
        use_regex: true,
    };
    let matches = search::find_all(&doc.rope, r"\d+đ", &opts).unwrap();
    assert_eq!(matches.len(), 3);
    assert_eq!(doc.rope.slice(matches[0].clone()).to_string(), "100đ");
    assert_eq!(doc.rope.slice(matches[1].clone()).to_string(), "250đ");

    let n = search::replace_all(&mut doc, r"\d+đ", "0đ", &opts).unwrap();
    assert_eq!(n, 3);
    assert_eq!(doc.rope.to_string(), "Giá: 0đ, Tổng: 0đ, Giảm: 0đ");

    assert!(doc.undo());
    assert_eq!(doc.rope.to_string(), "Giá: 100đ, Tổng: 250đ, Giảm: 50đ");
}

#[test]
fn find_next_and_prev_on_empty_and_wrapped() {
    assert_eq!(search::find_next(&[], 0), None);
    assert_eq!(search::find_prev(&[], 0), None);

    let matches = vec![2..5, 10..13, 20..23];
    assert_eq!(search::find_next(&matches, 0), Some(0));
    assert_eq!(search::find_next(&matches, 10), Some(1));
    assert_eq!(search::find_next(&matches, 21), Some(0)); // wrap
    assert_eq!(search::find_prev(&matches, 2), Some(2)); // wrap
    assert_eq!(search::find_prev(&matches, 15), Some(1));
}
