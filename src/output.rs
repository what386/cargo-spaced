use std::path::Path;

pub fn print_help() {
    println!(
        "cargo-spaced - Insert deterministic blank lines into Rust source code\n\n\
Usage: cargo spaced [OPTIONS] [PATH]...\n\n\
If no paths are supplied, the current directory is scanned recursively.\n\n\
Options:\n    --check       Check formatting without modifying files\n    -l, --files-with-diff\n                  Print names of files requiring formatting\n    -q, --quiet   Suppress progress output\n    --verbose     Report every processed Rust file\n    --ignore PATH Ignore a file or directory\n    -h, --help    Print this help message\n    -V, --version Print version information"
    );
}

pub fn print_version() {
    println!("cargo-spaced {}", env!("CARGO_PKG_VERSION"));
}

pub fn print_progress(path: &Path) {
    println!("Formatting {}", path.display());
}

pub fn print_path(path: &Path) {
    println!("{}", path.display());
}

pub fn print_result(path: &Path, source: &str, formatted: &str, files_with_diff: bool) {
    if files_with_diff {
        println!("{}", path.display());
    } else {
        print!("{}", diff_text(path, source, formatted));
    }
}

fn diff_text(path: &Path, source: &str, formatted: &str) -> String {
    let old_lines = split_lines(source);
    let new_lines = split_lines(formatted);
    let mut table = vec![vec![0usize; new_lines.len() + 1]; old_lines.len() + 1];

    for old in (0..old_lines.len()).rev() {
        for new in (0..new_lines.len()).rev() {
            table[old][new] = if old_lines[old] == new_lines[new] {
                table[old + 1][new + 1] + 1
            } else {
                table[old + 1][new].max(table[old][new + 1])
            };
        }
    }

    let mut lines = Vec::new();
    let (mut old, mut new) = (0, 0);
    while old < old_lines.len() || new < new_lines.len() {
        if old < old_lines.len() && new < new_lines.len() && old_lines[old] == new_lines[new] {
            lines.push(DiffLine::same(old_lines[old].to_owned(), old + 1));
            old += 1;
            new += 1;
        } else if new == new_lines.len()
            || (old < old_lines.len() && table[old + 1][new] >= table[old][new + 1])
        {
            lines.push(DiffLine::removed(old_lines[old].to_owned(), old + 1));
            old += 1;
        } else {
            lines.push(DiffLine::added(new_lines[new].to_owned(), old + 1));
            new += 1;
        }
    }

    let changed = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (line.kind != ' ').then_some(index))
        .collect::<Vec<_>>();
    let mut output = String::new();
    let mut group_start = 0;

    while group_start < changed.len() {
        let first = changed[group_start];
        let mut group_end = group_start + 1;
        while group_end < changed.len() && changed[group_end] <= changed[group_end - 1] + 6 {
            group_end += 1;
        }

        let start = first.saturating_sub(3);
        let end = (changed[group_end - 1] + 4).min(lines.len());
        output.push_str(&format!(
            "Diff in {}:{}:\n",
            path.display(),
            lines[first].old_line.max(1)
        ));
        for line in &lines[start..end] {
            output.push(line.kind);
            output.push_str(&line.text);
            if !line.text.ends_with('\n') {
                output.push('\n');
            }
        }

        group_start = group_end;
    }

    output
}

fn split_lines(source: &str) -> Vec<&str> {
    if source.is_empty() {
        Vec::new()
    } else {
        source.split_inclusive('\n').collect()
    }
}

struct DiffLine {
    kind: char,
    text: String,
    old_line: usize,
}

impl DiffLine {
    fn same(text: String, old_line: usize) -> Self {
        Self {
            kind: ' ',
            text,
            old_line,
        }
    }

    fn removed(text: String, old_line: usize) -> Self {
        Self {
            kind: '-',
            text,
            old_line,
        }
    }

    fn added(text: String, old_line: usize) -> Self {
        Self {
            kind: '+',
            text,
            old_line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_rustfmt_style_diff() {
        assert_eq!(
            diff_text(
                Path::new("src/lib.rs"),
                "fn first() {}\nfn second() {}\n",
                "fn first() {}\n\nfn second() {}\n"
            ),
            "Diff in src/lib.rs:2:\n fn first() {}\n+\n fn second() {}\n"
        );
    }
}
