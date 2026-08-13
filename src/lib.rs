pub mod cli;
mod edit;
pub mod errors;
mod formatter;
mod rules;
mod syntax;

pub use formatter::{FormatResult, format_source};
