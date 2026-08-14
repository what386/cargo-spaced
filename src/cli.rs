use crate::config::Config;
use crate::errors::{BoxError, CliError, FileError, WalkError, missing_path};
use crate::format_source;
use crate::output;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    paths: Vec<PathBuf>,
    ignores: Vec<PathBuf>,
    check: bool,
    quiet: bool,
    verbose: bool,
    files_with_diff: bool,
}

pub fn run() -> Result<ExitCode, BoxError> {
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
        output::print_help();
        return Ok(ExitCode::SUCCESS);
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        output::print_version();
        return Ok(ExitCode::SUCCESS);
    }

    let cli = Cli::parse_from(args)?;
    let (config, project_root) =
        Config::load(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))?;

    let mut ignores = config.ignore.clone();
    ignores.extend(cli.ignores);
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    let mut needs_formatting = false;

    for path in rust_files(&paths, &project_root, &ignores)? {
        if cli.verbose && !cli.quiet {
            output::print_progress(&path);
        }

        let source =
            fs::read_to_string(&path).map_err(|error| FileError::new("read", &path, error))?;

        let result = format_source(&source, &config)?;

        if !result.changed {
            continue;
        }

        needs_formatting = true;

        if cli.check {
            output::print_result(&path, &source, &result.output, cli.files_with_diff);
        } else {
            fs::write(&path, result.output)
                .map_err(|error| FileError::new("write", &path, error))?;
            if cli.files_with_diff {
                output::print_path(&path);
            }
        }
    }

    Ok(if cli.check && needs_formatting {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
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
        let mut files_with_diff = false;
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

            if options_allowed && (arg == "--files-with-diff" || arg == "-l") {
                files_with_diff = true;
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
            files_with_diff,
        })
    }
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
                files_with_diff: false,
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
    fn parses_output_options() {
        let args = ["cargo-spaced", "-q", "--verbose", "-l"]
            .into_iter()
            .map(OsString::from);

        let cli = Cli::parse_from(args).unwrap();
        assert!(cli.quiet);
        assert!(cli.verbose);
        assert!(cli.files_with_diff);
    }
}
