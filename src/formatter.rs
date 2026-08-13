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

    fn format(source: &str) -> String {
        format_source(source).unwrap().output
    }

    #[test]
    fn separates_block_bodied_items() {
        let source = "const X: usize = 1;\nfn main() {}\nstruct S { value: usize }\nimpl S { fn value(&self) -> usize { self.value } }\n";
        let expected = "const X: usize = 1;\n\nfn main() {}\n\nstruct S { value: usize }\n\nimpl S { fn value(&self) -> usize { self.value } }\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn separates_multiline_lets_and_block_statements() {
        let source = "fn main() {\n    let value = thing\n        .foo();\n    if value {\n        consume(value);\n    }\n    finish();\n}\n";
        let expected = "fn main() {\n    let value = thing\n        .foo();\n\n    if value {\n        consume(value);\n    }\n\n    finish();\n}\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn preserves_comment_and_attribute_attachment() {
        let source = "fn first() {}\n#[inline]\n/// second\nfn second() {}\nfn third() {}\nlet_not_code!();\n";
        let expected = "fn first() {}\n\n#[inline]\n/// second\nfn second() {}\n\nfn third() {}\n\nlet_not_code!();\n";
        assert_eq!(format(source), expected);

        let source = "fn main() {\n    let value = thing\n        .foo(); // trailing\n    consume(value);\n}\n";
        let expected = "fn main() {\n    let value = thing\n        .foo(); // trailing\n\n    consume(value);\n}\n";
        assert_eq!(format(source), expected);

        let source = "fn first() {}\n// Handles the fallback.\nfn second() {}\n";
        let expected = "fn first() {}\n\n// Handles the fallback.\nfn second() {}\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn does_not_split_expressions_or_excluded_containers() {
        let source = "fn main() {\n    if condition {\n        foo();\n    } else {\n        bar();\n    }\n    match value {\n        A => { foo(); }\n        B => { bar(); }\n    }.method();\n}\n";
        let expected = "fn main() {\n    if condition {\n        foo();\n    } else {\n        bar();\n    }\n\n    match value {\n        A => { foo(); }\n        B => { bar(); }\n    }.method();\n}\n";
        assert_eq!(format(source), expected);

        let source = "#[rustfmt::skip]\nfn skipped() {}\nfn next() {}\nfn main() {\n    #[rustfmt::skip]\n    let value = thing\n        .foo();\n    consume(value);\n}\n";
        let expected = "#[rustfmt::skip]\nfn skipped() {}\nfn next() {}\n\nfn main() {\n    #[rustfmt::skip]\n    let value = thing\n        .foo();\n    consume(value);\n}\n";
        assert_eq!(format(source), expected);

        let source = "fn main() {\n    #[rustfmt::skip]\n    if condition {\n        let value = thing\n            .foo();\n        consume(value);\n    }\n    finish();\n}\n";
        assert_eq!(format(source), source);
    }

    #[test]
    fn preserves_existing_blank_lines_and_crlf() {
        let source = "fn first() {}\r\n\r\n\r\nfn second() {}\r\n";
        assert_eq!(format(source), source);
    }

    #[test]
    fn formats_multiline_let_else() {
        let source = "fn main() {\n    let Some(value) = value else {\n        return;\n    };\n    consume(value);\n}\n";
        let expected = "fn main() {\n    let Some(value) = value else {\n        return;\n    };\n\n    consume(value);\n}\n";
        assert_eq!(format(source), expected);
    }
}
