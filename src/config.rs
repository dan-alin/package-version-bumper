use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
  #[serde(default = "default_tag_prefix")]
  pub tag_prefix: String,
  #[serde(default = "default_commit_prefix")]
  pub commit_prefix: String,
  pub package: Option<String>,
  #[serde(default = "default_push_remote")]
  pub push_remote: String,
}

fn default_tag_prefix() -> String {
  "v".to_owned()
}

fn default_commit_prefix() -> String {
  "chore(version): ".to_owned()
}

fn default_push_remote() -> String {
  "origin".to_owned()
}

impl Default for Config {
  fn default() -> Self {
    Config {
      tag_prefix: default_tag_prefix(),
      commit_prefix: default_commit_prefix(),
      package: None,
      push_remote: default_push_remote(),
    }
  }
}

impl Config {
  pub fn load() -> Result<Self> {
    Self::load_from(Path::new("vup.toml"))
  }

  pub(crate) fn load_from(path: &Path) -> Result<Self> {
    match std::fs::read_to_string(path) {
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
      Err(e) => Err(e.into()),
      Ok(data) => toml::from_str(&data).map_err(Into::into),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
  }

  #[test]
  fn absent_file_returns_defaults() {
    let dir = temp_dir();
    let config = Config::load_from(&dir.path().join("vup.toml")).unwrap();
    assert_eq!(config.tag_prefix, "v");
    assert_eq!(config.commit_prefix, "chore(version): ");
    assert!(config.package.is_none());
    assert_eq!(config.push_remote, "origin");
  }

  #[test]
  fn valid_toml_parses_all_fields() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(
      &path,
      r#"
tag_prefix = "release-"
commit_prefix = "bump: "
package = "crates/api/Cargo.toml"
push_remote = "upstream"
"#,
    )
    .unwrap();

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.tag_prefix, "release-");
    assert_eq!(config.commit_prefix, "bump: ");
    assert_eq!(config.package.as_deref(), Some("crates/api/Cargo.toml"));
    assert_eq!(config.push_remote, "upstream");
  }

  #[test]
  fn malformed_toml_returns_error() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(&path, "tag_prefix = [unclosed").unwrap();

    assert!(Config::load_from(&path).is_err());
  }

  #[test]
  fn unknown_keys_return_error() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(&path, "unknown_key = \"oops\"\n").unwrap();

    assert!(Config::load_from(&path).is_err());
  }
}
