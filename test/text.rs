use tgdiff::{Block, BlockKind, TextComparison};

fn kinds(comparison: &TextComparison) -> Vec<BlockKind> {
    comparison.blocks().iter().map(|block| block.kind).collect()
}

#[test]
fn identical_texts_have_one_equal_block() {
    let comparison = TextComparison::new("a\nb\n", "a\nb\n");
    assert!(comparison.is_identical());
    assert_eq!(kinds(&comparison), vec![BlockKind::Equal]);
    assert!(comparison.difference_blocks().is_empty());
}

#[test]
fn one_line_change_is_a_changed_block() {
    let comparison = TextComparison::new("a\nb\nc\n", "a\nX\nc\n");
    assert_eq!(
        comparison.blocks(),
        &[
            Block {
                kind: BlockKind::Equal,
                left: 0..1,
                right: 0..1
            },
            Block {
                kind: BlockKind::Changed,
                left: 1..2,
                right: 1..2
            },
            Block {
                kind: BlockKind::Equal,
                left: 2..3,
                right: 2..3
            },
        ]
    );
}

#[test]
fn insert_at_start_and_removal_at_end_are_added_and_removed() {
    let inserted = TextComparison::new("b\n", "a\nb\n");
    assert_eq!(
        inserted.blocks()[0],
        Block {
            kind: BlockKind::Added,
            left: 0..0,
            right: 0..1
        }
    );

    let removed = TextComparison::new("a\nb\n", "a\n");
    assert_eq!(
        removed.blocks()[1],
        Block {
            kind: BlockKind::Removed,
            left: 1..2,
            right: 1..1
        }
    );
}

#[test]
fn empty_against_text_is_a_single_added_block() {
    let comparison = TextComparison::new("", "x\ny\n");
    assert_eq!(
        comparison.blocks(),
        &[Block {
            kind: BlockKind::Added,
            left: 0..0,
            right: 0..2
        }]
    );
}

#[test]
fn two_empty_texts_are_identical_with_no_blocks() {
    let comparison = TextComparison::new("", "");
    assert!(comparison.is_identical());
    assert!(comparison.blocks().is_empty());
}

#[test]
fn crlf_and_lf_lines_differ() {
    let comparison = TextComparison::new("a\r\n", "a\n");
    assert!(!comparison.is_identical());
    assert_eq!(kinds(&comparison), vec![BlockKind::Changed]);
}

#[test]
fn blocks_cover_every_line_on_both_sides_in_order() {
    let left = "one\ntwo\nthree\nfour\n";
    let right = "zero\none\nTWO\nfour\nfive\n";
    let comparison = TextComparison::new(left, right);

    let mut left_next = 0;
    let mut right_next = 0;
    for block in comparison.blocks() {
        assert_eq!(block.left.start, left_next);
        assert_eq!(block.right.start, right_next);
        left_next = block.left.end;
        right_next = block.right.end;
    }
    assert_eq!(left_next, comparison.left_lines().len());
    assert_eq!(right_next, comparison.right_lines().len());
}

#[test]
fn navigation_moves_between_difference_blocks() {
    let comparison = TextComparison::new("a\nb\nc\nd\n", "a\nX\nc\nY\n");
    let differences = comparison.difference_blocks();
    assert_eq!(differences, vec![1, 3]);

    assert_eq!(comparison.next_difference(None), Some(1));
    assert_eq!(comparison.next_difference(Some(1)), Some(3));
    assert_eq!(comparison.next_difference(Some(3)), None);

    assert_eq!(comparison.prev_difference(3), Some(1));
    assert_eq!(comparison.prev_difference(1), None);
    assert_eq!(comparison.prev_difference(99), Some(3));
}
