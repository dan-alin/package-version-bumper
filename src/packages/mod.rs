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

pub fn get_package() -> Box<(dyn PackageUtils + 'static)> {
  let package_json = Path::new("package.json").exists();
  let cargo_toml = Path::new("Cargo.toml").exists();

  // if package_json && cargo_toml {
  //   #[allow(unreachable_code, clippy::diverging_sub_expression)]
  //   return unimplemented!("Both package.json and Cargo.toml found. This is not supported yet.");
  // } else
  if cargo_toml {
    let data = fs::read_to_string("Cargo.toml").expect("Unable to read file");
    let cargo_toml = toml::from_str(&data).expect("Cargo.toml should contain a version field.");

    Box::new(CargoToml {
      package_path: "cargo.toml".to_owned(),
      ..cargo_toml
    })
  } else if package_json {
    let data = fs::read_to_string("package.json").expect("Unable to read file");
    let package_json =
      serde_json::from_str(&data).expect("Package json should contain a version field.");

    return Box::new(PackageJson {
      package_path: "package.json".to_owned(),
      ..package_json
    });
  } else {
    panic!("No package file found")
  }
}
