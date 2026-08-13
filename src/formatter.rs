use crate::edit::{Edit, apply_edits};
use crate::{rules, syntax};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub changed: bool,
    pub edits: Vec<Edit>,
}

pub fn format_source(source: &str) -> Result<FormatResult> {
    let boundaries = syntax::collect_boundaries(source)?;
    let edits = rules::edits_for_boundaries(source, &boundaries);

    let output = apply_edits(source, &edits)?;
    let changed = output != source;

    Ok(FormatResult {
        output,
        changed,
        edits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_is_idempotent() {
        let source = "fn main() {}\n";

        let once = format_source(source).unwrap();
        let twice = format_source(&once.output).unwrap();

        assert_eq!(once.output, twice.output);
    }
}
