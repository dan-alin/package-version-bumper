use crate::version::Version;
use anyhow::Result;
use std::fs;
use std::path::Path;

use self::cargo::CargoToml;
use self::npm::PackageJson;

mod cargo;
mod npm;

pub trait PackageUtils {
  fn write_package(&self) -> Result<()>;

  fn get_version(&mut self) -> &Version;

  fn get_location(&self) -> &String;

  fn increment_major(&mut self) -> Result<()>;

  fn increment_minor(&mut self) -> Result<()>;

  fn increment_patch(&mut self) -> Result<()>;
}

pub fn get_package(config: &crate::config::Config) -> Box<dyn PackageUtils + 'static> {
  if let Some(explicit_path) = &config.package {
    let path = Path::new(explicit_path);
    let data = fs::read_to_string(path).expect("Unable to read file");
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if file_name == "Cargo.toml" {
      let cargo_toml: CargoToml =
        toml::from_str(&data).expect("Cargo.toml should contain a version field.");
      return Box::new(CargoToml {
        package_path: explicit_path.clone(),
        ..cargo_toml
      });
    } else {
      let package_json: PackageJson =
        serde_json::from_str(&data).expect("package.json should contain a version field.");
      return Box::new(PackageJson {
        package_path: explicit_path.clone(),
        ..package_json
      });
    }
  }

  let package_json = Path::new("package.json").exists();
  let cargo_toml = Path::new("Cargo.toml").exists();

  if cargo_toml {
    let data = fs::read_to_string("Cargo.toml").expect("Unable to read file");
    let cargo_toml = toml::from_str(&data).expect("Cargo.toml should contain a version field.");

    Box::new(CargoToml {
      package_path: "Cargo.toml".to_owned(),
      ..cargo_toml
    })
  } else if package_json {
    let data = fs::read_to_string("package.json").expect("Unable to read file");
    let package_json =
      serde_json::from_str(&data).expect("Package json should contain a version field.");

    Box::new(PackageJson {
      package_path: "package.json".to_owned(),
      ..package_json
    })
  } else {
    panic!("No package file found")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::Config;
  use std::env;
  use std::fs;
  use tempfile::tempdir;

  #[test]
  fn cargo_package_location_is_correctly_cased() {
    let original_dir = env::current_dir().unwrap();
    let temp_dir = env::temp_dir().join("vup_test_cargo_casing");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(
      temp_dir.join("Cargo.toml"),
      "[package]\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    env::set_current_dir(&temp_dir).unwrap();
    let package = get_package(&Config::default());
    let location = package.get_location().clone();
    env::set_current_dir(original_dir).unwrap();

    assert_eq!(location, "Cargo.toml");
  }

  #[test]
  fn explicit_package_path_overrides_auto_detect() {
    let dir = tempdir().unwrap();
    let cargo_path = dir.path().join("Cargo.toml");
    fs::write(&cargo_path, "[package]\nversion = \"0.1.0\"\n").unwrap();

    let config = Config {
      package: Some(cargo_path.to_str().unwrap().to_owned()),
      ..Config::default()
    };

    let package = get_package(&config);
    assert_eq!(package.get_location(), cargo_path.to_str().unwrap());
  }
}
