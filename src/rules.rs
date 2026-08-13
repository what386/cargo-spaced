use crate::config::Config;
use crate::edit::Edit;
use crate::syntax::{Boundary, ContainerKind, NodeKind};

pub fn edits_for_boundaries(
    source: &str,
    boundaries: &[Boundary],
    config: &Config,
) -> Vec<Edit> {
    boundaries
        .iter()
        .filter_map(|boundary| edit_for_boundary(source, boundary, config))
        .collect()
}

fn edit_for_boundary(source: &str, boundary: &Boundary, config: &Config) -> Option<Edit> {
    // A boundary adjacent to a skipped node is still outside that node and
    // may need spacing. Boundaries wholly inside skipped syntax have both
    // sibling nodes marked as skipped and must remain untouched.
    if boundary.previous.skipped && boundary.next.skipped
        || boundary.parent_kind == ContainerKind::Statements
            && (boundary.previous.skipped || boundary.next.skipped)
    {
        return None;
    }

    let required = required_blank_lines(boundary, config);

    if required == 0 {
        return None;
    }

    let existing = blank_lines_in(&source[boundary.range.clone()]);

    if existing >= required {
        return None;
    }

    // V1 is insertion-only. We don't normalize existing whitespace.
    //
    // This assumes `boundary.range` ends immediately before the next
    // node's attached leading trivia. The parser/trivia layer should make
    // that invariant true.
    Some(Edit::insert(boundary.insertion_offset, newline_for(source)))
}

fn required_blank_lines(boundary: &Boundary, config: &Config) -> usize {
    [
        rule_block_bodied_item_boundary(boundary),
        rule_multiline_let(boundary),
        rule_multiline_block_statement(boundary),
        rule_multiline_declaration(boundary),
        rule_multiline_macro_statement(boundary),
        rule_match_arm_boundary(boundary, config),
    ]
    .into_iter()
    .max()
    .unwrap_or(0)
}

fn rule_multiline_declaration(boundary: &Boundary) -> usize {
    if boundary.parent_kind == ContainerKind::Items
        && boundary.previous.kind == NodeKind::MultilineDeclaration
    {
        1
    } else {
        0
    }
}

fn rule_multiline_macro_statement(boundary: &Boundary) -> usize {
    if boundary.parent_kind == ContainerKind::Statements
        && boundary.previous.kind == NodeKind::MultilineMacroStatement
    {
        1
    } else {
        0
    }
}

fn rule_match_arm_boundary(boundary: &Boundary, config: &Config) -> usize {
    if config.match_arm_spacing
        && boundary.parent_kind == ContainerKind::MatchArms
        && (boundary.previous.multiline || boundary.next.multiline)
    {
        1
    } else {
        0
    }
}

/// I001:
/// Require a blank line between sibling items when either side is
/// block-bodied.
fn rule_block_bodied_item_boundary(boundary: &Boundary) -> usize {
    if boundary.parent_kind == ContainerKind::Items
        && (boundary.previous.kind == NodeKind::BlockBodiedItem
            || boundary.next.kind == NodeKind::BlockBodiedItem)
    {
        1
    } else {
        0
    }
}

/// S001/S002:
/// Require a blank line after a multiline `let` statement.
///
/// A parser backend should classify multiline `let-else` as `LetStatement`,
/// so it naturally follows the same rule.
fn rule_multiline_let(boundary: &Boundary) -> usize {
    if boundary.parent_kind == ContainerKind::Statements
        && boundary.previous.kind == NodeKind::LetStatement
        && boundary.previous.multiline
    {
        1
    } else {
        0
    }
}

/// S003:
/// Require a blank line after a standalone multiline block-expression
/// statement.
fn rule_multiline_block_statement(boundary: &Boundary) -> usize {
    if boundary.parent_kind == ContainerKind::Statements
        && boundary.previous.kind == NodeKind::BlockExpressionStatement
        && boundary.previous.multiline
    {
        1
    } else {
        0
    }
}

/// Count completely empty physical lines in a whitespace region.
///
/// This intentionally stays simple for the skeleton. Once comments/trivia
/// handling is implemented, the boundary range should contain only the
/// whitespace that is actually eligible for replacement/insertion.
fn blank_lines_in(text: &str) -> usize {
    text.lines()
        .filter(|line| line.trim().is_empty())
        .count()
        .saturating_sub(1)
}

fn newline_for(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_blank_lines() {
        assert_eq!(blank_lines_in("\n"), 0);
        assert_eq!(blank_lines_in("\n\n"), 1);
        assert_eq!(blank_lines_in("\n\n\n"), 2);
    }
}
