use crate::config::Config;
use crate::edit::{Edit, apply_edits};
use crate::errors::BoxError;
use crate::{rules, syntax};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub changed: bool,
    pub edits: Vec<Edit>,
}

pub fn format_source(source: &str, config: &Config) -> Result<FormatResult, BoxError> {
    let boundaries = syntax::collect_boundaries(source)?;
    let edits = rules::edits_for_boundaries(source, &boundaries, config);

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
    use crate::config::Config;

    fn format(source: &str) -> String {
        format_source(source, &Config::default()).unwrap().output
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
        let expected = "#[rustfmt::skip]\nfn skipped() {}\n\nfn next() {}\n\nfn main() {\n    #[rustfmt::skip]\n    let value = thing\n        .foo();\n    consume(value);\n}\n";
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

    #[test]
    fn separates_multiline_declarations() {
        let source = "const VALUE: usize = calculate(\n    first,\n    second,\n);\nstatic CONFIG: Config = Config {\n    enabled: true,\n};\ntype ResultType = std::result::Result<\n    Value,\n    Error,\n>;\nfn consume() {}\n";
        let expected = "const VALUE: usize = calculate(\n    first,\n    second,\n);\n\nstatic CONFIG: Config = Config {\n    enabled: true,\n};\n\ntype ResultType = std::result::Result<\n    Value,\n    Error,\n>;\n\nfn consume() {}\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn separates_multiline_macro_statements_without_touching_bodies() {
        let source = "fn main() {\n    println!(\n        \"value: {}\",\n        value,\n    );\n    some_macro! {\n        first();\n        second();\n    }\n    finish();\n}\n";
        let expected = "fn main() {\n    println!(\n        \"value: {}\",\n        value,\n    );\n\n    some_macro! {\n        first();\n        second();\n    }\n\n    finish();\n}\n";
        assert_eq!(format(source), expected);
    }

    #[test]
    fn match_arm_spacing_is_opt_in_and_only_affects_multiline_arms() {
        let source = "fn main(value: Value) {\n    match value {\n        Value::A => {\n            first();\n        }\n        Value::B => second(),\n        Value::C => {\n            third();\n        }\n    }\n}\n";
        assert_eq!(format(source), source);

        let config = Config {
            match_arm_spacing: true,
            ..Config::default()
        };

        let expected = "fn main(value: Value) {\n    match value {\n        Value::A => {\n            first();\n        }\n\n        Value::B => second(),\n\n        Value::C => {\n            third();\n        }\n    }\n}\n";
        assert_eq!(
            format_source(source, &config).unwrap().output,
            expected
        );
    }
}
