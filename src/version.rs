use std::fmt::Display;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub struct Version {
  pub major: u64,
  pub minor: u64,
  pub patch: u64,
}

impl<'de> Deserialize<'de> for Version {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s = String::deserialize(deserializer)?;
    let parts: Vec<&str> = s.split('.').collect();
    let major: u64 = parts[0].parse().unwrap_or(0);
    let minor: u64 = parts[1].parse().unwrap_or(0);
    let patch: u64 = parts[2].parse().unwrap_or(0);
    Ok(Version {
      major,
      minor,
      patch,
    })
  }
}

impl Serialize for Version {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let version = format!("{}.{}.{}", self.major, self.minor, self.patch);
    serializer.serialize_str(&version)
  }
}

impl Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
  }
}

#[allow(dead_code)]
impl Version {
  pub fn new(major: u64, minor: u64, patch: u64) -> Version {
    Version {
      major,
      minor,
      patch,
    }
  }

  pub fn from_str(version: &str) -> Version {
    let parts: Vec<&str> = version.split('.').collect();
    let major: u64 = parts[0].parse().unwrap_or(0);
    let minor: u64 = parts[1].parse().unwrap_or(0);
    let patch: u64 = parts[2].parse().unwrap_or(0);
    Version {
      major,
      minor,
      patch,
    }
  }

  pub fn increment_major(&mut self) {
    self.major += 1;
    self.minor = 0;
    self.patch = 0;
  }

  pub fn increment_minor(&mut self) {
    self.minor += 1;
    self.patch = 0;
  }

  pub fn increment_patch(&mut self) {
    self.patch += 1;
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_version_from_str() {
    let version = Version::from_str("1.2.3");
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 3);
  }

  #[test]
  fn test_version_increment_major() {
    let mut version = Version::from_str("1.2.3");
    version.increment_major();
    assert_eq!(version.major, 2);
    assert_eq!(version.minor, 0);
    assert_eq!(version.patch, 0);
  }

  #[test]
  fn test_version_increment_minor() {
    let mut version = Version::from_str("1.2.3");
    version.increment_minor();
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 3);
    assert_eq!(version.patch, 0);
  }

  #[test]
  fn test_version_increment_patch() {
    let mut version = Version::from_str("1.2.3");
    version.increment_patch();
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 4);
  }

  #[test]
  fn test_version_serialize() {
    let version = Version::from_str("1.2.3");
    let serialized = serde_json::to_string(&version).unwrap();
    assert_eq!(serialized, "\"1.2.3\"");
  }

  #[test]
  fn test_version_deserialize() {
    let version: Version = serde_json::from_str("\"1.2.3\"").unwrap();
    assert_eq!(version.major, 1);
    assert_eq!(version.minor, 2);
    assert_eq!(version.patch, 3);
  }

  #[test]
  fn test_version_display() {
    let version = Version::from_str("1.2.3");
    assert_eq!(format!("{}", version), "1.2.3");
  }
}
