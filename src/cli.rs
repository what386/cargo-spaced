use crate::errors::{BoxError, CliError, FileError, WalkError, missing_path};
use crate::format_source;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    paths: Vec<PathBuf>,
    check: bool,
}

pub fn run() -> Result<(), BoxError> {
    // Cargo invokes subcommands as:
    //
    //     cargo spaced ...
    //
    // by executing:
    //
    //     cargo-spaced spaced ...
    //
    // The parser would otherwise interpret "spaced" as a positional path.
    let mut args: Vec<_> = std::env::args_os().collect();

    if args.get(1).is_some_and(|arg| arg == "spaced") {
        args.remove(1);
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("cargo-spaced {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let cli = Cli::parse_from(args)?;
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    let mut needs_formatting = false;

    for path in rust_files(&paths)? {
        let source =
            fs::read_to_string(&path).map_err(|error| FileError::new("read", &path, error))?;

        let result = format_source(&source)?;

        if !result.changed {
            continue;
        }

        needs_formatting = true;

        if cli.check {
            eprintln!("would format {}", path.display());
        } else {
            fs::write(&path, result.output)
                .map_err(|error| FileError::new("write", &path, error))?;
            eprintln!("formatted {}", path.display());
        }
    }

    if cli.check && needs_formatting {
        return Err(CliError::new("formatting required").into());
    }

    Ok(())
}

impl Cli {
    fn parse_from<I>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut paths = Vec::new();
        let mut check = false;
        let mut options_allowed = true;

        for arg in args.into_iter().skip(1) {
            if options_allowed && arg == "--" {
                options_allowed = false;
                continue;
            }

            if options_allowed && arg == "--check" {
                check = true;
                continue;
            }

            if options_allowed && arg.to_str().is_some_and(|arg| arg.starts_with('-')) {
                return Err(CliError::new(format!(
                    "unexpected argument: {}",
                    arg.to_string_lossy()
                )));
            }

            paths.push(PathBuf::from(arg));
        }

        Ok(Self { paths, check })
    }
}

fn print_help() {
    println!(
        "cargo-spaced - Insert deterministic blank lines into Rust source code\n\n\
Usage: cargo spaced [OPTIONS] [PATH]...\n\n\
If no paths are supplied, the current directory is scanned recursively.\n\n\
Options:\n    --check       Check formatting without modifying files\n    -h, --help    Print this help message\n    -V, --version Print version information"
    );
}

fn rust_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, BoxError> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_rust_file(path) {
                files.push(path.clone());
            }

            continue;
        }

        if !path.exists() {
            return Err(missing_path(path).into());
        }

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| !should_skip(entry.path()))
        {
            let entry = entry.map_err(|error| WalkError::new(path, error))?;

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

    #[test]
    fn parses_check_and_paths() {
        let args = ["cargo-spaced", "--check", "src", "file.rs"]
            .into_iter()
            .map(OsString::from);

        assert_eq!(
            Cli::parse_from(args).unwrap(),
            Cli {
                paths: vec![PathBuf::from("src"), PathBuf::from("file.rs")],
                check: true,
            }
        );
    }

    #[test]
    fn supports_option_terminator_and_rejects_unknown_options() {
        let args = ["cargo-spaced", "--", "--literal.rs"]
            .into_iter()
            .map(OsString::from);

        assert_eq!(
            Cli::parse_from(args).unwrap().paths,
            vec![PathBuf::from("--literal.rs")]
        );

        let args = ["cargo-spaced", "--unknown"]
            .into_iter()
            .map(OsString::from);

        assert!(Cli::parse_from(args).is_err());
    }
}
