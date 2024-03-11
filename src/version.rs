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
