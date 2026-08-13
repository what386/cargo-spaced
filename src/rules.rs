use crate::config::Config;
use crate::edit::Edit;
use crate::syntax::{Boundary, ContainerKind, NodeKind};

pub fn edits_for_boundaries(source: &str, boundaries: &[Boundary], config: &Config) -> Vec<Edit> {
    boundaries
        .iter()
        .flat_map(|boundary| edit_for_boundary(source, boundary, config))
        .collect()
}

fn edit_for_boundary(source: &str, boundary: &Boundary, config: &Config) -> Vec<Edit> {
    // A boundary adjacent to a skipped node is still outside that node and
    // may need spacing. Boundaries wholly inside skipped syntax have both
    // sibling nodes marked as skipped and must remain untouched.
    if boundary.previous.skipped && boundary.next.skipped
        || boundary.parent_kind == ContainerKind::Statements
            && (boundary.previous.skipped || boundary.next.skipped)
    {
        return Vec::new();
    }

    let required = required_blank_lines(boundary, config);

    if required == 0 {
        return Vec::new();
    }

    let existing = blank_lines_in(&source[boundary.range.clone()]);
    let mut edits = Vec::new();

    if existing > required {
        if !config.rules.normalize_blank_lines {
            return Vec::new();
        }

        if let Some(edit) = normalize_boundary(source, boundary, required) {
            edits.push(edit);
        }
    }

    if config.rules.normalize_blank_lines
        && let Some(edit) = normalize_attached_trivia(source, boundary)
    {
        edits.push(edit);
    }

    if existing >= required {
        return edits;
    }

    // This assumes `boundary.range` ends immediately before the next node's
    // attached leading trivia. The parser/trivia layer should make that
    // invariant true.
    edits.push(Edit::insert(boundary.insertion_offset, newline_for(source)));
    edits
}

fn normalize_boundary(source: &str, boundary: &Boundary, required: usize) -> Option<Edit> {
    // Leading comments and attributes are part of a node's attached trivia.
    // Only replace the whitespace before that trivia; replacing the complete
    // boundary can delete a doc comment when the parser includes it in the
    // next node's leading range.
    let start = if boundary.range.start > 0
        && source.as_bytes()[boundary.range.start - 1] == b'\r'
        && source.as_bytes().get(boundary.range.start) == Some(&b'\n')
    {
        boundary.range.start - 1
    } else {
        boundary.range.start
    };

    let end = source[start..boundary.range.end]
        .char_indices()
        .find(|(_, character)| !character.is_whitespace())
        .map_or(boundary.range.end, |(offset, _)| start + offset);

    let range = start..end;
    let existing = blank_lines_in(&source[range.clone()]);

    if existing <= required {
        return None;
    }

    let indentation_start = source[..end].rfind('\n').map_or(0, |offset| offset + 1);
    let indentation_length = source[indentation_start..end]
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let indentation = &source[indentation_start..indentation_start + indentation_length];

    Some(Edit::replace(
        range,
        format!(
            "{}{}",
            newline_for(source).repeat(required + 1),
            indentation
        ),
    ))
}

fn normalize_attached_trivia(source: &str, boundary: &Boundary) -> Option<Edit> {
    let start = source[..boundary.next.range.start]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let mut last_trivia_end = None;
    let mut saw_attached_trivia = false;
    let mut saw_blank_line = false;

    let mut line_start = start;

    while line_start < boundary.next.range.end {
        let line_end = source[line_start..boundary.next.range.end]
            .find('\n')
            .map_or(boundary.next.range.end, |offset| line_start + offset);
        let next_line_start = (line_end < boundary.next.range.end).then_some(line_end + 1);
        let line = &source[line_start..next_line_start.unwrap_or(line_end)];
        let content = line.trim_end_matches(['\r', '\n']);
        let trimmed = content.trim();

        if trimmed.is_empty() {
            if saw_attached_trivia {
                saw_blank_line = true;
            }
            line_start = next_line_start.unwrap_or(line_end);
            continue;
        }

        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            saw_attached_trivia = true;
            last_trivia_end = Some(line_start + line.len());
            line_start = next_line_start.unwrap_or(line_end);
            continue;
        }

        if !saw_blank_line {
            return None;
        }

        let token_start = line_start + content.len() - content.trim_start().len();
        let range = last_trivia_end?..token_start;
        return Some(Edit::replace(
            range,
            source[line_start..token_start].to_owned(),
        ));
    }

    None
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
    if config.rules.match_arm_spacing
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
