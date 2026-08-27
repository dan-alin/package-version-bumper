use anyhow::Result;
use serde::Deserialize;
use std::io::{BufRead, Write as IoWrite};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatternConfig {
  pub regex: String,
  pub section: String,
  #[serde(default)]
  pub template: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ReleaseNotesConfig {
  #[serde(default)]
  pub patterns: Vec<PatternConfig>,
}

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
  #[serde(default)]
  pub release_notes: ReleaseNotesConfig,
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
      release_notes: ReleaseNotesConfig::default(),
    }
  }
}

fn strip_line_ending(s: &str) -> String {
  s.trim_end_matches(['\n', '\r']).to_owned()
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

  pub fn from_prompts(reader: &mut impl BufRead) -> Result<Config> {
    print!("Tag prefix (empty string = bare tags like \"1.2.3\" instead of \"v1.2.3\") [v]: ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let tag_prefix = {
      let t = strip_line_ending(&line);
      if t.trim().is_empty() {
        default_tag_prefix()
      } else {
        t
      }
    };

    print!("Commit message prefix [chore(version): ]: ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let commit_prefix = {
      let t = strip_line_ending(&line);
      if t.trim().is_empty() {
        default_commit_prefix()
      } else {
        t
      }
    };

    let package_hint = if Path::new("Cargo.toml").exists() {
      " (auto-detects Cargo.toml — press Enter to keep, or type a path)"
    } else if Path::new("package.json").exists() {
      " (auto-detects package.json — press Enter to keep, or type a path)"
    } else {
      " (press Enter to auto-detect, or type an explicit path)"
    };
    print!("Package file{}: ", package_hint);
    std::io::stdout().flush()?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let package = {
      let t = strip_line_ending(&line);
      if t.trim().is_empty() {
        None
      } else {
        Some(t)
      }
    };

    let push_remote = loop {
      print!("Git remote name (not a URL, required): ");
      std::io::stdout().flush()?;
      let mut line = String::new();
      reader.read_line(&mut line)?;
      let t = strip_line_ending(&line);
      if t.trim().is_empty() {
        println!("push_remote cannot be empty — please enter a remote name.");
      } else {
        break t;
      }
    };

    Ok(Config {
      tag_prefix,
      commit_prefix,
      package,
      push_remote,
      release_notes: ReleaseNotesConfig::default(),
    })
  }

  pub fn write(&self, path: &Path) -> Result<()> {
    let package_line = match &self.package {
      None => "# package = \"packages/api/Cargo.toml\"".to_owned(),
      Some(p) => format!("package = \"{}\"", p),
    };
    let contents = format!(
      "# Tag prefix prepended to the version number when creating git tags.\n\
       # Set to \"\" for bare version tags like \"1.2.3\" instead of \"v1.2.3\".\n\
       tag_prefix = \"{tag_prefix}\"\n\
       \n\
       # Prefix for the version-bump commit message.\n\
       commit_prefix = \"{commit_prefix}\"\n\
       \n\
       # Explicit path to the package file (Cargo.toml or package.json).\n\
       # If unset, vup auto-detects from the current working directory.\n\
       {package_line}\n\
       \n\
       # Git remote to push commits and tags to.\n\
       push_remote = \"{push_remote}\"\n",
      tag_prefix = self.tag_prefix,
      commit_prefix = self.commit_prefix,
      package_line = package_line,
      push_remote = self.push_remote,
    );
    std::fs::write(path, contents).map_err(Into::into)
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
    assert!(config.release_notes.patterns.is_empty());
  }

  #[test]
  fn valid_toml_parses_all_fields() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(
      &path,
      concat!(
        "tag_prefix = \"release-\"\n",
        "commit_prefix = \"bump: \"\n",
        "package = \"crates/api/Cargo.toml\"\n",
        "push_remote = \"upstream\"\n",
        "\n",
        "[[release_notes.patterns]]\n",
        "regex = \"#US-\\\\d+\"\n",
        "section = \"User Stories\"\n",
      ),
    )
    .unwrap();

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.tag_prefix, "release-");
    assert_eq!(config.commit_prefix, "bump: ");
    assert_eq!(config.package.as_deref(), Some("crates/api/Cargo.toml"));
    assert_eq!(config.push_remote, "upstream");
    assert_eq!(config.release_notes.patterns.len(), 1);
    assert_eq!(config.release_notes.patterns[0].regex, r"#US-\d+");
    assert_eq!(config.release_notes.patterns[0].section, "User Stories");
    assert!(config.release_notes.patterns[0].template.is_none());
  }

  #[test]
  fn pattern_template_parses_when_present() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(
      &path,
      concat!(
        "[[release_notes.patterns]]\n",
        "regex = \"#US-(\\\\d+)\"\n",
        "section = \"User Stories\"\n",
        "template = \"#$1\"\n",
      ),
    )
    .unwrap();
    let config = Config::load_from(&path).unwrap();
    assert_eq!(
      config.release_notes.patterns[0].template.as_deref(),
      Some("#$1")
    );
  }

  #[test]
  fn pattern_template_absent_returns_none() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(
      &path,
      concat!(
        "[[release_notes.patterns]]\n",
        "regex = \"#US-\\\\d+\"\n",
        "section = \"User Stories\"\n",
      ),
    )
    .unwrap();
    let config = Config::load_from(&path).unwrap();
    assert!(config.release_notes.patterns[0].template.is_none());
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

  #[test]
  fn write_default_creates_parseable_file() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    Config::default().write(&path).unwrap();
    assert!(path.exists());
    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.tag_prefix, "v");
    assert_eq!(config.commit_prefix, "chore(version): ");
    assert!(config.package.is_none());
    assert_eq!(config.push_remote, "origin");
    assert!(config.release_notes.patterns.is_empty());
  }

  #[test]
  fn write_round_trips_non_default_config() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    let original = Config {
      tag_prefix: "release-".to_owned(),
      commit_prefix: "bump: ".to_owned(),
      package: Some("packages/api/Cargo.toml".to_owned()),
      push_remote: "upstream".to_owned(),
      release_notes: ReleaseNotesConfig::default(),
    };
    original.write(&path).unwrap();
    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.tag_prefix, original.tag_prefix);
    assert_eq!(loaded.commit_prefix, original.commit_prefix);
    assert_eq!(loaded.package, original.package);
    assert_eq!(loaded.push_remote, original.push_remote);
  }

  #[test]
  fn from_prompts_all_defaults_accepted() {
    // blank for tag_prefix, commit_prefix, package; empty then "origin" for push_remote (re-prompt)
    let input = b"\n\n\n\norigin\n";
    let mut reader = std::io::Cursor::new(&input[..]);
    let config = Config::from_prompts(&mut reader).unwrap();
    assert_eq!(config.tag_prefix, "v");
    assert_eq!(config.commit_prefix, "chore(version): ");
    assert!(config.package.is_none());
    assert_eq!(config.push_remote, "origin");
  }

  #[test]
  fn from_prompts_all_fields_overridden() {
    let input = b"release-\nbump: \npackages/api/Cargo.toml\nupstream\n";
    let mut reader = std::io::Cursor::new(&input[..]);
    let config = Config::from_prompts(&mut reader).unwrap();
    assert_eq!(config.tag_prefix, "release-");
    assert_eq!(config.commit_prefix, "bump: ");
    assert_eq!(config.package.as_deref(), Some("packages/api/Cargo.toml"));
    assert_eq!(config.push_remote, "upstream");
  }

  #[test]
  fn from_prompts_empty_push_remote_reprompts() {
    // empty push_remote first, then valid value
    let input = b"\n\n\n\nupstream\n";
    let mut reader = std::io::Cursor::new(&input[..]);
    let config = Config::from_prompts(&mut reader).unwrap();
    assert_eq!(config.push_remote, "upstream");
  }

  #[test]
  fn from_prompts_package_blank_returns_none() {
    let input = b"\n\n\norigin\n";
    let mut reader = std::io::Cursor::new(&input[..]);
    let config = Config::from_prompts(&mut reader).unwrap();
    assert!(config.package.is_none());
  }

  #[test]
  fn from_prompts_package_typed_returns_some() {
    let input = b"\n\nmy/path/Cargo.toml\norigin\n";
    let mut reader = std::io::Cursor::new(&input[..]);
    let config = Config::from_prompts(&mut reader).unwrap();
    assert_eq!(config.package.as_deref(), Some("my/path/Cargo.toml"));
  }

  #[test]
  fn release_notes_patterns_absent_returns_empty() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(&path, "tag_prefix = \"v\"\n").unwrap();
    let config = Config::load_from(&path).unwrap();
    assert!(config.release_notes.patterns.is_empty());
  }

  #[test]
  fn release_notes_patterns_parses_correctly() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(
      &path,
      concat!(
        "tag_prefix = \"v\"\n",
        "commit_prefix = \"chore(version): \"\n",
        "push_remote = \"origin\"\n",
        "\n",
        "[[release_notes.patterns]]\n",
        "regex = \"#US-\\\\d+\"\n",
        "section = \"User Stories\"\n",
        "\n",
        "[[release_notes.patterns]]\n",
        "regex = \"#BUG-\\\\d+\"\n",
        "section = \"Bug Fixes\"\n",
      ),
    )
    .unwrap();

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.release_notes.patterns.len(), 2);
    assert_eq!(config.release_notes.patterns[0].regex, r"#US-\d+");
    assert_eq!(config.release_notes.patterns[0].section, "User Stories");
    assert_eq!(config.release_notes.patterns[1].regex, r"#BUG-\d+");
    assert_eq!(config.release_notes.patterns[1].section, "Bug Fixes");
  }

  #[test]
  fn release_notes_unknown_key_returns_error() {
    let dir = temp_dir();
    let path = dir.path().join("vup.toml");
    fs::write(&path, "[release_notes]\nunknown_key = \"oops\"\n").unwrap();
    assert!(Config::load_from(&path).is_err());
  }
}
