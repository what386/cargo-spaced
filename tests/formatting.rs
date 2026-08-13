use cargo_spaced::config::Config;
use cargo_spaced::format_source;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn formatting_fixtures() {
    let unformatted_dir = Path::new("tests/fixtures/unformatted");
    let formatted_dir = Path::new("tests/fixtures/formatted");

    let mut cases = fixture_files(unformatted_dir);
    cases.sort();

    assert!(!cases.is_empty(), "no formatting fixtures found");

    for input_path in cases {
        let file_name = input_path.file_name().unwrap();
        let expected_path = formatted_dir.join(file_name);

        assert!(
            expected_path.exists(),
            "missing formatted fixture for {}",
            input_path.display()
        );

        let input = fs::read_to_string(&input_path).unwrap();
        let expected = fs::read_to_string(&expected_path).unwrap();

        let once = format_source(&input, &Config::default()).unwrap();
        assert_eq!(
            once.output,
            expected,
            "formatted output differs for {}",
            file_name.to_string_lossy()
        );

        let twice = format_source(&once.output, &Config::default()).unwrap();
        assert_eq!(
            twice.output,
            expected,
            "formatter is not idempotent for {}",
            file_name.to_string_lossy()
        );
    }
}

#[test]
fn every_formatted_fixture_has_an_input() {
    let unformatted_dir = Path::new("tests/fixtures/unformatted");
    let formatted_dir = Path::new("tests/fixtures/formatted");

    for expected_path in fixture_files(formatted_dir) {
        let file_name = expected_path.file_name().unwrap();
        let input_path = unformatted_dir.join(file_name);

        assert!(
            input_path.exists(),
            "formatted fixture has no matching input: {}",
            expected_path.display()
        );
    }
}

fn fixture_files(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect()
}
