use crate::text::{myers, BlockKind, Op, TextComparison};
use serde::Serialize;

/// Side-by-side comparison sent to the UI.
#[derive(Debug, Serialize)]
pub struct ComparisonView {
    pub identical: bool,
    pub difference_count: usize,
    pub blocks: Vec<BlockView>,
}

/// One block of aligned lines. `kind` is one of `equal`, `changed`, `added` or `removed`.
#[derive(Debug, Serialize)]
pub struct BlockView {
    pub kind: &'static str,
    pub left: Vec<LineView>,
    pub right: Vec<LineView>,
}

/// A line with its 1-based number and the segments that make it up.
#[derive(Debug, Serialize)]
pub struct LineView {
    pub number: usize,
    pub segments: Vec<Segment>,
}

/// A run of characters in a line. `changed` marks text that differs from the other side.
#[derive(Debug, Serialize)]
pub struct Segment {
    pub text: String,
    pub changed: bool,
}

/// Compares two texts and returns their blocks with 1-based line numbers.
/// Never fails. Empty text has no lines.
///
/// ```
/// let view = tgdiff::view::build("a\nb", "a\nc");
/// assert!(!view.identical);
/// assert_eq!(view.difference_count, 1);
/// ```
pub fn build(left: &str, right: &str) -> ComparisonView {
    let comparison = TextComparison::new(left, right);
    let left_lines = comparison.left_lines();
    let right_lines = comparison.right_lines();
    let mut blocks: Vec<BlockView> = comparison
        .blocks()
        .iter()
        .map(|block| {
            let highlight = block.kind == BlockKind::Changed;
            let left_block = &left_lines[block.left.clone()];
            let right_block = &right_lines[block.right.clone()];
            BlockView {
                kind: kind_name(block.kind),
                left: side_lines(
                    left_block,
                    block.left.start,
                    right_block,
                    highlight.then_some(Op::Delete),
                ),
                right: side_lines(
                    right_block,
                    block.right.start,
                    left_block,
                    highlight.then_some(Op::Insert),
                ),
            }
        })
        .collect();
    blocks.extend(trailing_row(
        left,
        right,
        left_lines.len(),
        right_lines.len(),
    ));
    ComparisonView {
        identical: comparison.is_identical(),
        difference_count: comparison.difference_blocks().len(),
        blocks,
    }
}

/// Returns an extra display-only row for a trailing newline, so the UI shows the empty
/// last line. It is an `equal` block and does not affect differences or `identical`.
/// Returns `None` when neither text ends with a newline.
fn trailing_row(
    left: &str,
    right: &str,
    left_count: usize,
    right_count: usize,
) -> Option<BlockView> {
    let end_row = |ends_with_newline: bool, count: usize| {
        if ends_with_newline {
            vec![LineView {
                number: count + 1,
                segments: Vec::new(),
            }]
        } else {
            Vec::new()
        }
    };
    if !left.ends_with('\n') && !right.ends_with('\n') {
        return None;
    }
    Some(BlockView {
        kind: "equal",
        left: end_row(left.ends_with('\n'), left_count),
        right: end_row(right.ends_with('\n'), right_count),
    })
}

/// Maps a block kind to the lowercase name used by the UI.
fn kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Equal => "equal",
        BlockKind::Changed => "changed",
        BlockKind::Added => "added",
        BlockKind::Removed => "removed",
    }
}

/// Numbers one side of a block, starting at `start` (0-based). When `changed_op` is set,
/// each line is aligned with the line at the same offset on the other side on character level.
fn side_lines(
    own: &[String],
    start: usize,
    other: &[String],
    changed_op: Option<Op>,
) -> Vec<LineView> {
    own.iter()
        .enumerate()
        .map(|(offset, text)| {
            let segments = if let Some(changed_op) = changed_op {
                let own_chars: Vec<char> = text.chars().collect();
                let other_chars: Vec<char> = other
                    .get(offset)
                    .map(|line| line.chars().collect())
                    .unwrap_or_default();
                // Ops always run from the left line to the right line; on the right
                // side the direction is swapped, and deletions become insertions.
                let (from, to) = if changed_op == Op::Delete {
                    (&own_chars, &other_chars)
                } else {
                    (&other_chars, &own_chars)
                };
                let ops = myers(from, to);
                segments_from_ops(&own_chars, &ops, changed_op)
            } else {
                vec![Segment {
                    text: text.clone(),
                    changed: false,
                }]
            };
            LineView {
                number: start + offset + 1,
                segments,
            }
        })
        .collect()
}

/// Builds this side's line segments from char-level edit `ops` between the two lines,
/// flagging characters of `changed_op` kind as changed. Empty runs are dropped.
fn segments_from_ops(own: &[char], ops: &[Op], changed_op: Op) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut index = 0usize;
    for op in ops {
        let changed = *op == changed_op;
        if changed || *op == Op::Equal {
            let text = own[index].to_string();
            match segments.last_mut() {
                Some(segment) if segment.changed == changed => segment.text.push_str(&text),
                _ => segments.push(Segment { text, changed }),
            }
            index += 1;
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_changed_line_as_changed_block() {
        let view = build("a\nb\n", "a\nc\n");
        let kinds: Vec<&str> = view.blocks.iter().map(|block| block.kind).collect();
        assert_eq!(kinds, ["equal", "changed", "equal"]);
        assert_eq!(view.difference_count, 1);
        assert!(!view.identical);
    }

    #[test]
    fn identical_texts_have_no_differences() {
        let view = build("x\n", "x\n");
        assert!(view.identical);
        assert_eq!(view.difference_count, 0);
    }

    #[test]
    fn highlights_only_the_added_suffix_within_a_changed_line() {
        let view = build("test\n", "test123\n");
        let right = &view.blocks[0].right[0];
        let changed: Vec<&str> = right
            .segments
            .iter()
            .filter(|segment| segment.changed)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(changed, ["123"]);
    }

    #[test]
    fn highlights_the_middle_of_a_line_on_both_sides() {
        let view = build("abcXdef\n", "abcYdef\n");
        let changed = |side: &[LineView]| -> Vec<String> {
            side[0]
                .segments
                .iter()
                .filter(|segment| segment.changed)
                .map(|segment| segment.text.clone())
                .collect()
        };
        assert_eq!(changed(&view.blocks[0].left), ["X"]);
        assert_eq!(changed(&view.blocks[0].right), ["Y"]);
    }

    #[test]
    fn highlights_each_differing_character_separately() {
        let view = build("lala\n", "lolo\n");
        let changed = |side: &[LineView]| -> Vec<String> {
            side[0]
                .segments
                .iter()
                .filter(|segment| segment.changed)
                .map(|segment| segment.text.clone())
                .collect()
        };
        assert_eq!(changed(&view.blocks[0].left), ["a", "a"]);
        assert_eq!(changed(&view.blocks[0].right), ["o", "o"]);
    }

    #[test]
    fn marks_whole_line_when_it_has_no_counterpart() {
        let view = build("x\n", "y\nz\n");
        let unpaired = &view.blocks[0].right[1];
        assert_eq!(unpaired.number, 2);
        assert_eq!(unpaired.segments.len(), 1);
        assert!(unpaired.segments[0].changed);
        assert_eq!(unpaired.segments[0].text, "z");
    }

    #[test]
    fn added_lines_are_not_highlighted_within() {
        let view = build("a\n", "a\nb\n");
        assert_eq!(view.blocks[1].kind, "added");
        let right = &view.blocks[1].right[0];
        assert!(right.segments.iter().all(|segment| !segment.changed));
    }

    #[test]
    fn empty_texts_have_no_blocks() {
        let view = build("", "");
        assert!(view.identical);
        assert!(view.blocks.is_empty());
    }

    #[test]
    fn changed_block_numbers_lines_from_its_start() {
        let view = build("a\nb\n", "a\nc\n");
        assert_eq!(view.blocks[1].left[0].number, 2);
        assert_eq!(view.blocks[1].right[0].number, 2);
    }

    #[test]
    fn highlights_by_character_after_multibyte_text() {
        let view = build("café 1\n", "café 2\n");
        let changed: Vec<&str> = view.blocks[0].right[0]
            .segments
            .iter()
            .filter(|segment| segment.changed)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(changed, ["2"]);
    }

    #[test]
    fn trailing_newline_adds_an_empty_last_row_without_a_difference() {
        let view = build("a\n", "a\n");
        assert!(view.identical);
        assert_eq!(view.difference_count, 0);
        let last = view.blocks.last().unwrap();
        assert_eq!(last.left[0].number, 2);
        assert!(last.left[0].segments.is_empty());
    }

    #[test]
    fn text_without_trailing_newline_adds_no_extra_row() {
        let view = build("a", "a");
        assert_eq!(view.blocks.len(), 1);
    }
}
