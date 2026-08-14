use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_file() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cargo-spaced-cli-{}-{nonce}.rs",
        std::process::id()
    ))
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-spaced"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn check_prints_a_diff_and_returns_failure_without_modifying_the_file() {
    let path = temp_file();
    let source = "fn first() {}\nfn second() {}\n";
    fs::write(&path, source).unwrap();

    let output = run(&["--check", path.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .starts_with(&format!("Diff in {}:", path.display()))
    );
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    fs::remove_file(path).unwrap();
}

#[test]
fn files_with_diff_prints_only_changed_paths() {
    let path = temp_file();
    fs::write(&path, "fn first() {}\nfn second() {}\n").unwrap();

    let output = run(&[
        "--quiet",
        "--check",
        "--files-with-diff",
        path.to_str().unwrap(),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", path.display())
    );
    assert!(output.stderr.is_empty());
    fs::remove_file(path).unwrap();
}

#[test]
fn normal_runs_are_silent_and_format_the_file() {
    let path = temp_file();
    fs::write(&path, "fn first() {}\nfn second() {}\n").unwrap();

    let output = run(&[path.to_str().unwrap()]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "fn first() {}\n\nfn second() {}\n"
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn verbose_reports_each_file_on_stdout() {
    let path = temp_file();
    fs::write(&path, "fn first() {}\n").unwrap();

    let output = run(&["--verbose", path.to_str().unwrap()]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("Formatting {}\n", path.display())
    );
    assert!(output.stderr.is_empty());
    fs::remove_file(path).unwrap();
}
