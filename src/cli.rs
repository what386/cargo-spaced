use crate::config::Config;
use crate::errors::{BoxError, CliError, FileError, WalkError, missing_path};
use crate::format_source;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    paths: Vec<PathBuf>,
    ignores: Vec<PathBuf>,
    check: bool,
    quiet: bool,
    verbose: bool,
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
    let (config, project_root) =
        Config::load(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))?;

    let mut ignores = config.ignore;
    ignores.extend(cli.ignores);
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    let mut needs_formatting = false;

    for path in rust_files(&paths, &project_root, &ignores)? {
        let source =
            fs::read_to_string(&path).map_err(|error| FileError::new("read", &path, error))?;

        let result = format_source(&source)?;

        if !result.changed {
            if cli.verbose && !cli.quiet {
                eprintln!("checked {}: unchanged", path.display());
            }

            continue;
        }

        needs_formatting = true;
        let summary = (cli.verbose && !cli.quiet).then(|| change_summary(&source, &result));

        if cli.check {
            if !cli.quiet {
                if cli.verbose {
                    eprintln!(
                        "would format {}: {}",
                        path.display(),
                        summary.as_deref().unwrap()
                    );
                } else {
                    eprintln!("would format {}", path.display());
                }
            }
        } else {
            fs::write(&path, result.output)
                .map_err(|error| FileError::new("write", &path, error))?;
            if !cli.quiet {
                if cli.verbose {
                    eprintln!(
                        "formatted {}: {}",
                        path.display(),
                        summary.as_deref().unwrap()
                    );
                } else {
                    eprintln!("formatted {}", path.display());
                }
            }
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
        let mut ignores = Vec::new();
        let mut check = false;
        let mut quiet = false;
        let mut verbose = false;
        let mut options_allowed = true;

        let mut args = args.into_iter().skip(1);

        while let Some(arg) = args.next() {
            if options_allowed && arg == "--" {
                options_allowed = false;
                continue;
            }

            if options_allowed && arg == "--check" {
                check = true;
                continue;
            }

            if options_allowed && (arg == "--quiet" || arg == "-q") {
                quiet = true;
                continue;
            }

            if options_allowed && arg == "--verbose" {
                verbose = true;
                continue;
            }

            if options_allowed && arg == "--ignore" {
                let Some(ignore) = args.next() else {
                    return Err(CliError::new("--ignore requires a path"));
                };

                ignores.push(PathBuf::from(ignore));
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

        Ok(Self {
            paths,
            ignores,
            check,
            quiet,
            verbose,
        })
    }
}

fn print_help() {
    println!(
        "cargo-spaced - Insert deterministic blank lines into Rust source code\n\n\
Usage: cargo spaced [OPTIONS] [PATH]...\n\n\
If no paths are supplied, the current directory is scanned recursively.\n\n\
Options:\n    --check       Check formatting without modifying files\n    -q, --quiet   Suppress informational output\n    --verbose     Report every processed Rust file\n    --ignore PATH Ignore a file or directory\n    -h, --help    Print this help message\n    -V, --version Print version information"
    );
}

fn rust_files(
    paths: &[PathBuf],
    project_root: &Path,
    ignores: &[PathBuf],
) -> Result<Vec<PathBuf>, BoxError> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            if is_rust_file(path) && !is_ignored(path, project_root, ignores) {
                files.push(path.clone());
            }

            continue;
        }

        if !path.exists() {
            return Err(missing_path(path).into());
        }

        for entry in WalkDir::new(path).into_iter().filter_entry(|entry| {
            !should_skip(entry.path()) && !is_ignored(entry.path(), project_root, ignores)
        }) {
            let entry = entry.map_err(|error| WalkError::new(path, error))?;

            if entry.file_type().is_file()
                && is_rust_file(entry.path())
                && !is_ignored(entry.path(), project_root, ignores)
            {
                files.push(entry.into_path());
            }
        }
    }

    files.sort();
    files.dedup();

    Ok(files)
}

fn is_ignored(path: &Path, project_root: &Path, ignores: &[PathBuf]) -> bool {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    ignores.iter().any(|ignore| {
        let ignored = if ignore.is_absolute() {
            ignore.clone()
        } else {
            project_root.join(ignore)
        };

        absolute == ignored || absolute.starts_with(&ignored)
    })
}

fn change_summary(source: &str, result: &crate::FormatResult) -> String {
    let lines = result
        .edits
        .iter()
        .map(|edit| {
            source[..edit.range.start]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count()
                + 1
        })
        .collect::<std::collections::BTreeSet<_>>();

    let line_list = lines
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    let change_word = if result.edits.len() == 1 {
        "change"
    } else {
        "changes"
    };

    format!("{} {change_word} on lines {line_list}", result.edits.len())
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
                ignores: Vec::new(),
                check: true,
                quiet: false,
                verbose: false,
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

    #[test]
    fn parses_repeated_ignore_options() {
        let args = [
            "cargo-spaced",
            "--ignore",
            "generated",
            "--ignore",
            "old.rs",
        ]
        .into_iter()
        .map(OsString::from);

        assert_eq!(
            Cli::parse_from(args).unwrap().ignores,
            vec![PathBuf::from("generated"), PathBuf::from("old.rs")]
        );
    }

    #[test]
    fn parses_quiet_and_verbose_options() {
        let args = ["cargo-spaced", "-q", "--verbose"]
            .into_iter()
            .map(OsString::from);

        let cli = Cli::parse_from(args).unwrap();
        assert!(cli.quiet);
        assert!(cli.verbose);
    }

    #[test]
    fn summarizes_changes_using_original_source_lines() {
        let source = "first\nsecond\nthird\n";
        let result = crate::FormatResult {
            output: String::new(),
            changed: true,
            edits: vec![
                crate::edit::Edit::insert(6, "\n"),
                crate::edit::Edit::insert(6, "\n"),
                crate::edit::Edit::insert(13, "\n"),
            ],
        };

        assert_eq!(change_summary(source, &result), "3 changes on lines 2, 3");
    }
}
