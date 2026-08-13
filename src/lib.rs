pub mod cli;
mod edit;
mod formatter;
mod rules;
mod syntax;

pub use formatter::{FormatResult, format_source};
