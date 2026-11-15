//! Line-based two-way text comparison using Myers' O(ND) algorithm.

use std::ops::Range;

/// Kind of a block in a text comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Same lines on both sides.
    Equal,
    /// Lines present on both sides with different content.
    Changed,
    /// Lines present only on the right side.
    Added,
    /// Lines present only on the left side.
    Removed,
}

/// A run of consecutive lines of one kind, with its 0-based, end-exclusive line ranges on each side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Kind of this block.
    pub kind: BlockKind,
    /// Line range on the left side.
    pub left: Range<usize>,
    /// Line range on the right side.
    pub right: Range<usize>,
}

/// One step of the edit script between the left and right inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    Equal,
    Delete,
    Insert,
}

/// Line-by-line comparison of a left text and a right text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextComparison {
    left: Vec<String>,
    right: Vec<String>,
    blocks: Vec<Block>,
}

impl TextComparison {
    /// Compares two texts line by line and returns the result.
    ///
    /// Lines are split on `\n`, a trailing `\n` does not add a line, and `\r` is kept, so CRLF and LF differ.
    /// Lines are split on `\n`, a trailing `\n` does not add a line, and `\r` is kept, so CRLF and LF differ.
    /// Never fails. Empty text has zero lines.
    ///
    /// ```
    /// use tgdiff::text::TextComparison;
    ///
    /// let comparison = TextComparison::new("a\nb\n", "a\nc\n");
    /// assert!(!comparison.is_identical());
    /// assert_eq!(comparison.difference_blocks().len(), 1);
    /// ```
    pub fn new(left: &str, right: &str) -> Self {
        let left_refs = split_lines(left);
        let right_refs = split_lines(right);
        let ops = myers(&left_refs, &right_refs);
        Self {
            left: left_refs.iter().map(|line| line.to_string()).collect(),
            right: right_refs.iter().map(|line| line.to_string()).collect(),
            blocks: group_ops(&ops),
        }
    }

    /// Returns the left lines without line terminators.
    pub fn left_lines(&self) -> &[String] {
        &self.left
    }

    /// Returns the right lines without line terminators.
    pub fn right_lines(&self) -> &[String] {
        &self.right
    }

    /// Returns all blocks in document order, including equal blocks.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Returns true when both texts have the same lines in the same order.
    pub fn is_identical(&self) -> bool {
        self.blocks
            .iter()
            .all(|block| block.kind == BlockKind::Equal)
    }

    /// Returns the indexes into `blocks()` of every non-equal block, in order.
    pub fn difference_blocks(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.kind != BlockKind::Equal)
            .map(|(index, _)| index)
            .collect()
    }

    /// Returns the index of the first difference block after `after`, or the first one when `after` is None.
    ///
    /// Returns None when no difference block follows.
    ///
    /// ```
    /// use tgdiff::text::TextComparison;
    ///
    /// let comparison = TextComparison::new("a\nb\nc\nd", "a\nB\nc\nD");
    /// let first = comparison.next_difference(None).unwrap();
    /// let second = comparison.next_difference(Some(first)).unwrap();
    /// assert_eq!(comparison.next_difference(Some(second)), None);
    /// assert_eq!(comparison.prev_difference(second), Some(first));
    /// ```
    pub fn next_difference(&self, after: Option<usize>) -> Option<usize> {
        let start = after.map_or(0, |index| index + 1);
        (start..self.blocks.len()).find(|&index| self.blocks[index].kind != BlockKind::Equal)
    }

    /// Returns the index of the last difference block before `before`.
    ///
    /// Returns None when no difference block comes before it.
    pub fn prev_difference(&self, before: usize) -> Option<usize> {
        (0..before.min(self.blocks.len()))
            .rev()
            .find(|&index| self.blocks[index].kind != BlockKind::Equal)
    }
}

/// Splits text into lines on `\n`, ignoring one trailing `\n`.
///
/// Returns an empty list for empty text. Never fails.
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .collect()
}

/// Finds a shortest edit script between `left` and `right` with Myers' algorithm.
///
/// Returns the operations in order. Memory grows with the edit distance times the total item count.
/// Panics only if the search loop ends without reaching both ends, which the algorithm rules out.
pub(crate) fn myers<T: PartialEq>(left: &[T], right: &[T]) -> Vec<Op> {
    let n = left.len() as isize;
    let m = right.len() as isize;
    let max = n + m;
    let offset = max;
    let mut v = vec![0isize; (2 * max + 2) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let i = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) {
                v[i + 1]
            } else {
                v[i - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && left[x as usize] == right[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                return backtrack(&trace, n, m, offset);
            }
            k += 2;
        }
    }
    unreachable!("Myers search always reaches the end of both inputs")
}

/// Walks the saved search frontiers backwards and returns the edit script in forward order.
///
/// Returns an empty list when both inputs are empty. Never fails.
fn backtrack(trace: &[Vec<isize>], n: isize, m: isize, offset: isize) -> Vec<Op> {
    let (mut x, mut y) = (n, m);
    let mut ops = Vec::new();

    for (d, v) in trace.iter().enumerate().rev() {
        let d = d as isize;
        if d == 0 {
            while x > 0 {
                ops.push(Op::Equal);
                x -= 1;
            }
            break;
        }
        let k = x - y;
        let i = (k + offset) as usize;
        let down = k == -d || (k != d && v[i - 1] < v[i + 1]);
        let prev_k = if down { k + 1 } else { k - 1 };
        let px = v[(prev_k + offset) as usize];
        let py = px - prev_k;
        let (mx, my) = if down { (px, py + 1) } else { (px + 1, py) };

        while x > mx && y > my {
            ops.push(Op::Equal);
            x -= 1;
            y -= 1;
        }
        ops.push(if down { Op::Insert } else { Op::Delete });
        x = px;
        y = py;
    }
    ops.reverse();
    ops
}

/// Groups an edit script into blocks with line ranges on each side.
///
/// Returns the blocks in order. A run of changes with both removed and added lines becomes one changed block. Never fails.
fn group_ops(ops: &[Op]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let (mut left_pos, mut right_pos) = (0usize, 0usize);
    let mut i = 0;

    while i < ops.len() {
        if ops[i] == Op::Equal {
            let start = i;
            while i < ops.len() && ops[i] == Op::Equal {
                i += 1;
            }
            let len = i - start;
            blocks.push(Block {
                kind: BlockKind::Equal,
                left: left_pos..left_pos + len,
                right: right_pos..right_pos + len,
            });
            left_pos += len;
            right_pos += len;
        } else {
            let (mut deleted, mut inserted) = (0usize, 0usize);
            while i < ops.len() && ops[i] != Op::Equal {
                match ops[i] {
                    Op::Delete => deleted += 1,
                    _ => inserted += 1,
                }
                i += 1;
            }
            let kind = match (deleted > 0, inserted > 0) {
                (true, true) => BlockKind::Changed,
                (true, false) => BlockKind::Removed,
                _ => BlockKind::Added,
            };
            blocks.push(Block {
                kind,
                left: left_pos..left_pos + deleted,
                right: right_pos..right_pos + inserted,
            });
            left_pos += deleted;
            right_pos += inserted;
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_newline_does_not_change_identity() {
        let comparison = TextComparison::new("a\nb", "a\nb\n");
        assert!(comparison.is_identical());
        assert_eq!(comparison.left_lines().len(), 2);
    }

    #[test]
    fn consecutive_changes_form_one_changed_block() {
        let comparison = TextComparison::new("a\nb\nc\n", "a\nX\nY\nc\n");
        assert_eq!(
            comparison.blocks()[1],
            Block {
                kind: BlockKind::Changed,
                left: 1..2,
                right: 1..3,
            }
        );
    }

    #[test]
    fn removed_block_at_start_has_empty_right_range() {
        let comparison = TextComparison::new("x\na\n", "a\n");
        assert_eq!(
            comparison.blocks()[0],
            Block {
                kind: BlockKind::Removed,
                left: 0..1,
                right: 0..0,
            }
        );
    }

    #[test]
    fn next_difference_moves_forward_and_stops_at_the_end() {
        let comparison = TextComparison::new("a\nb\n", "a\nX\n");
        assert_eq!(comparison.next_difference(None), Some(1));
        assert_eq!(comparison.next_difference(Some(1)), None);
    }

    #[test]
    fn prev_difference_clamps_an_index_past_the_end() {
        let comparison = TextComparison::new("a\nX\nc\n", "a\nY\nc\nd\n");
        assert_eq!(comparison.prev_difference(10), Some(3));
        assert_eq!(comparison.prev_difference(3), Some(1));
        assert_eq!(comparison.prev_difference(1), None);
    }
}
