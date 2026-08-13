use crate::errors::EditError;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<usize>,
    pub replacement: String,
}

impl Edit {
    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self {
            range: offset..offset,
            replacement: text.into(),
        }
    }

    pub fn replace(range: Range<usize>, text: impl Into<String>) -> Self {
        Self {
            range,
            replacement: text.into(),
        }
    }
}

pub fn apply_edits(source: &str, edits: &[Edit]) -> Result<String, EditError> {
    if edits.is_empty() {
        return Ok(source.to_owned());
    }

    let mut edits = edits.to_vec();
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));

    validate_edits(source, &edits)?;

    let mut output = source.to_owned();

    for edit in edits.into_iter().rev() {
        output.replace_range(edit.range, &edit.replacement);
    }

    Ok(output)
}

fn validate_edits(source: &str, edits: &[Edit]) -> Result<(), EditError> {
    let mut previous_end = 0;

    for (index, edit) in edits.iter().enumerate() {
        if edit.range.start > edit.range.end || edit.range.end > source.len() {
            return Err(EditError::new(format!(
                "edit range is outside source bounds: {:?}",
                edit.range
            )));
        }

        if !source.is_char_boundary(edit.range.start) || !source.is_char_boundary(edit.range.end) {
            return Err(EditError::new(
                "edit range is not on UTF-8 character boundaries",
            ));
        }

        if index > 0 && edit.range.start < previous_end {
            return Err(EditError::new("overlapping edits are not allowed"));
        }

        previous_end = edit.range.end;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_insertions_in_source_order() {
        let source = "ab";
        let edits = vec![Edit::insert(1, "X"), Edit::insert(2, "Y")];

        assert_eq!(apply_edits(source, &edits).unwrap(), "aXbY");
    }

    #[test]
    fn applies_replacements() {
        let source = "hello world";
        let edits = vec![Edit::replace(6..11, "Rust")];

        assert_eq!(apply_edits(source, &edits).unwrap(), "hello Rust");
    }

    #[test]
    fn rejects_overlapping_edits() {
        let source = "abcdef";
        let edits = vec![Edit::replace(1..4, "x"), Edit::replace(3..5, "y")];

        assert!(apply_edits(source, &edits).is_err());
    }
}
