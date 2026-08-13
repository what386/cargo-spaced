use crate::errors::ConfigError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "cargo-spaced.toml";

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub ignore: Vec<PathBuf>,
    #[serde(default)]
    pub rules: Rules,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    #[serde(default)]
    pub match_arm_spacing: bool,
    #[serde(default = "default_normalize_blank_lines")]
    pub normalize_blank_lines: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            match_arm_spacing: false,
            normalize_blank_lines: true,
        }
    }
}

fn default_normalize_blank_lines() -> bool {
    true
}

impl Config {
    pub fn load(start: impl AsRef<Path>) -> Result<(Self, PathBuf), ConfigError> {
        let root = project_root(start.as_ref());
        let path = root.join(CONFIG_FILE);

        if !path.exists() {
            return Ok((Self::default(), root));
        }

        let text =
            fs::read_to_string(&path).map_err(|error| ConfigError::read(path.clone(), error))?;

        let config = toml::from_str(&text)
            .map_err(|error| ConfigError::parse(path.clone(), error.to_string()))?;

        Ok((config, root))
    }
}

fn project_root(start: &Path) -> PathBuf {
    let start = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(start)
    };

    let directory = if start.is_dir() {
        start
    } else {
        start.parent().unwrap_or(&start).to_path_buf()
    };

    for ancestor in directory.ancestors() {
        if ancestor.join("Cargo.toml").is_file() {
            return ancestor.to_path_buf();
        }
    }

    directory
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_ignore_paths() {
        let config: Config = toml::from_str(
            r#"
ignore = ["generated/", "src/legacy.rs"]
"#,
        )
        .unwrap();

        assert_eq!(
            config.ignore,
            vec![PathBuf::from("generated/"), PathBuf::from("src/legacy.rs")]
        );

        assert!(!config.rules.match_arm_spacing);
        assert!(config.rules.normalize_blank_lines);
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = toml::from_str::<Config>("ignored = [\"target\"]");
        assert!(result.is_err());
    }

    #[test]
    fn deserializes_match_arm_spacing() {
        let config: Config =
            toml::from_str("[rules]\nmatch_arm_spacing = true\nnormalize_blank_lines = true")
                .unwrap();

        assert!(config.rules.match_arm_spacing);
        assert!(config.rules.normalize_blank_lines);
    }
}
