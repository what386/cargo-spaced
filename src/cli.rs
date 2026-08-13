use crate::format_source;
use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(name = "cargo-spaced")]
#[command(about = "Insert deterministic blank lines into Rust source code")]
pub struct Cli {
    /// Files or directories to format.
    ///
    /// If omitted, the current directory is scanned recursively.
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,

    /// Check whether files need formatting without modifying them.
    #[arg(long)]
    check: bool,
}

pub fn run() -> Result<()> {
    // Cargo invokes subcommands as:
    //
    //     cargo spaced ...
    //
    // by executing:
    //
    //     cargo-spaced spaced ...
    //
    // Clap would otherwise interpret "spaced" as a positional path.
    let args = std::env::args_os();
    let mut args: Vec<_> = args.collect();

    if args.get(1).is_some_and(|arg| arg == "spaced") {
        args.remove(1);
    }

    let cli = Cli::parse_from(args);
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    let mut needs_formatting = false;

    for path in rust_files(&paths)? {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;

        let result = format_source(&source)?;

        if !result.changed {
            continue;
        }

        needs_formatting = true;

        if cli.check {
            eprintln!("would format {}", path.display());
        } else {
            fs::write(&path, result.output)
                .with_context(|| format!("failed to write {}", path.display()))?;
            eprintln!("formatted {}", path.display());
        }
    }

    if cli.check && needs_formatting {
        anyhow::bail!("formatting required");
    }

    Ok(())
}

fn rust_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_rust_file(path) {
                files.push(path.clone());
            }
            continue;
        }

        if !path.exists() {
            anyhow::bail!("path does not exist: {}", path.display());
        }

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| !should_skip(entry.path()))
        {
            let entry = entry.with_context(|| {
                format!("failed while walking {}", path.display())
            })?;

            if entry.file_type().is_file() && is_rust_file(entry.path()) {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();

    Ok(files)
}

fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

fn should_skip(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_rust_files() {
        assert!(is_rust_file(Path::new("src/lib.rs")));
        assert!(!is_rust_file(Path::new("Cargo.toml")));
    }

    #[test]
    fn skips_target_and_git() {
        assert!(should_skip(Path::new("target")));
        assert!(should_skip(Path::new(".git")));
        assert!(!should_skip(Path::new("src")));
    }
}
