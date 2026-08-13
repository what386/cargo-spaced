pub mod cli;
mod edit;
mod formatter;
mod rules;
mod syntax;
mod trivia;

pub use formatter::{FormatResult, format_source};
