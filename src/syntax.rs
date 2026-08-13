use anyhow::Result;
use std::ops::Range;

/// A syntax container whose direct children may participate in spacing rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    ItemList,
    StatementList,
    Other,
}

/// A coarse classification used by spacing rules.
///
/// Keep this intentionally smaller than the parser's complete syntax-kind
/// enumeration. The formatter should only expose distinctions needed by
/// formatting policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    BlockBodiedItem,
    LetStatement,
    BlockExpressionStatement,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfo {
    /// Byte range of the node's syntax, excluding attached leading trivia.
    pub range: Range<usize>,

    /// Byte range including leading comments/attributes that should stay
    /// attached to this node.
    pub leading_range: Range<usize>,

    /// Byte range including trailing comments that should stay attached to
    /// this node.
    pub trailing_range: Range<usize>,

    pub kind: NodeKind,

    /// Whether the node spans more than one physical source line.
    pub multiline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    pub parent_kind: ContainerKind,
    pub previous: NodeInfo,
    pub next: NodeInfo,

    /// The whitespace/trivia region between the logical end of `previous`
    /// and the logical beginning of `next`.
    pub range: Range<usize>,
}

/// Parse `source` and collect boundaries between relevant sibling syntax nodes.
///
/// This is deliberately the parser-specific seam of the formatter.
///
/// A likely implementation strategy is:
///
/// 1. Parse the source with a lossless Rust parser.
/// 2. Walk item lists and statement lists.
/// 3. Classify each direct child into `NodeKind`.
/// 4. Extend node ranges through attached comments/attributes via `trivia`.
/// 5. Emit one `Boundary` for each adjacent pair of relevant siblings.
///
/// The initial skeleton returns no boundaries, so formatting is a no-op until
/// a parser backend is added.
pub fn collect_boundaries(_source: &str) -> Result<Vec<Boundary>> {
    Ok(Vec::new())
}

pub fn line_count(source: &str, range: &Range<usize>) -> usize {
    source[range.clone()].bytes().filter(|&byte| byte == b'\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_physical_lines() {
        let source = "a\nb\nc";
        assert_eq!(line_count(source, &(0..source.len())), 3);
    }
}
