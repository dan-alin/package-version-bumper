use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::{packages::PackageUtils, version::Version};

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoPackage {
  pub version: Version,
  #[serde(flatten)]
  pub others: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoToml {
  pub package: CargoPackage,
  #[serde(skip)]
  pub package_path: String,
  #[serde(flatten)]
  pub others: serde_json::Value,
}

impl PackageUtils for CargoToml {
  fn write_package(&self) -> Result<()> {
    let data = toml::to_string_pretty(&self)?;
    fs::write(&self.package_path, data)?;
    Ok(())
  }

  fn get_version(&mut self) -> &Version {
    &self.package.version
  }

  fn get_location(&self) -> &String {
    &self.package_path
  }

  fn increment_major(&mut self) -> Result<()> {
    self.package.version.increment_major();
    Ok(())
  }

  fn increment_minor(&mut self) -> Result<()> {
    self.package.version.increment_minor();
    Ok(())
  }

  fn increment_patch(&mut self) -> Result<()> {
    self.package.version.increment_patch();
    Ok(())
  }
}
