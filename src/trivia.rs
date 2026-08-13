use std::ops::Range;

/// Information about comments, attributes, and whitespace attached to a syntax
/// node.
///
/// This module is intentionally parser-agnostic. Once a lossless parser is
/// selected, parser-specific token walking can live here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedTrivia {
    pub leading: Range<usize>,
    pub trailing: Range<usize>,
}

/// Determine the logical source range occupied by leading trivia attached to a
/// node.
///
/// Intended rules:
///
/// - outer attributes belong to the following item
/// - doc comments belong to the following item
/// - ordinary leading comments normally belong to the following node
/// - blank lines can terminate attachment
///
/// `node_range` should cover the node's actual syntax tokens.
pub fn leading_trivia(
    _source: &str,
    node_range: Range<usize>,
) -> Range<usize> {
    node_range.start..node_range.start
}

/// Determine the logical source range occupied by trailing trivia attached to a
/// node.
///
/// Intended rules:
///
/// - end-of-line comments belong to the preceding node
/// - whitespace after the newline does not
pub fn trailing_trivia(
    _source: &str,
    node_range: Range<usize>,
) -> Range<usize> {
    node_range.end..node_range.end
}
