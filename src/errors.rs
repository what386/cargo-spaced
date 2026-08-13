use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Rust source: {}", self.message)
    }
}

impl StdError for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError {
    pub message: String,
}

impl EditError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formatter edit error: {}", self.message)
    }
}

impl StdError for EditError {}

#[derive(Debug)]
pub struct FileError {
    pub operation: &'static str,
    pub path: PathBuf,
    pub source: io::Error,
}

impl FileError {
    pub fn new(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed to {} {}: {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl StdError for FileError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
pub struct WalkError {
    pub path: PathBuf,
    pub source: walkdir::Error,
}

impl WalkError {
    pub fn new(path: impl Into<PathBuf>, source: walkdir::Error) -> Self {
        Self {
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for WalkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "failed while walking {}: {}",
            self.path.display(),
            self.source
        )
    }
}

impl StdError for WalkError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    pub message: String,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for CliError {}

pub fn missing_path(path: &Path) -> CliError {
    CliError::new(format!("path does not exist: {}", path.display()))
}
