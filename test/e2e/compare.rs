use tgdiff::view::{build, ComparisonView};

fn line_texts(lines: &[tgdiff::view::LineView]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.segments.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

#[test]
fn identical_texts_report_no_differences() {
    let result: ComparisonView = build("a\nb\n", "a\nb\n");
    assert!(result.identical);
    assert_eq!(result.difference_count, 0);
    assert!(result.blocks.iter().all(|block| block.kind == "equal"));
}

#[test]
fn changed_line_marks_only_the_differing_middle() {
    let result = build("the 123 end\n", "the 456 end\n");
    assert!(!result.identical);
    assert_eq!(result.difference_count, 1);

    let block = &result.blocks[0];
    assert_eq!(block.kind, "changed");
    let left = &block.left[0].segments;
    assert_eq!(left[0].text, "the ");
    assert!(!left[0].changed);
    assert_eq!(left[1].text, "123");
    assert!(left[1].changed);
    assert_eq!(left[2].text, " end");
    assert!(!left[2].changed);
    assert_eq!(line_texts(&block.right), vec!["the 456 end"]);
}

#[test]
fn text_added_on_one_side_is_listed_with_line_numbers() {
    let result = build("", "x\ny\n");
    assert!(!result.identical);
    let added = result
        .blocks
        .iter()
        .find(|block| block.kind == "added")
        .expect("an added block");
    assert!(added.left.is_empty());
    assert_eq!(line_texts(&added.right), vec!["x", "y"]);
    assert_eq!(added.right[0].number, 1);
    assert_eq!(added.right[1].number, 2);
}
