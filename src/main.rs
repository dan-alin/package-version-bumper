use std::path::Path;

use clap::{App, Arg};
use git2::Repository;
use serde::{Deserialize, Serialize};

mod packages;
mod version;
mod versioning;

use packages::PackageUtils;

#[derive(Debug, Serialize, Deserialize)]
struct Package {
  version: version::Version,
  #[serde(flatten)]
  others: serde_json::Value,
}

fn main() -> anyhow::Result<()> {
  let matches = App::new("pvb")
    .arg(
      Arg::with_name("major")
        .short('m')
        .long("major")
        .help("Increment the major version"),
    )
    .arg(
      Arg::with_name("minor")
        .short('n')
        .long("minor")
        .help("Increment the minor version"),
    )
    .arg(
      Arg::with_name("patch")
        .short('p')
        .long("patch")
        .help("Increment the patch version"),
    )
    .arg(
      Arg::with_name("verbose")
        .short('v')
        .long("verbose")
        .help("Prints debug information"),
    )
    .arg(
      Arg::with_name("tag")
        .short('t')
        .long("no-tag")
        .help("skip tag the commit with the new version"),
    )
    .get_matches();

  if matches.is_present("verbose") {
    env_logger::init();
    log::info!("Starting pvb");
  }

  let mut package: Box<dyn PackageUtils> = packages::get_package();

  match (
    matches.is_present("major"),
    matches.is_present("minor"),
    matches.is_present("patch"),
  ) {
    (true, _, _) => package.increment_major()?,
    (_, true, _) => package.increment_minor()?,
    (_, _, true) => package.increment_patch()?,
    _ => {
      println!("No version increment option specified.");
      return Ok(());
    }
  };

  let version = package.get_version().to_string();

  println!("Update the version to: {}? [y/N]", version);

  let mut confirmation = String::new();
  std::io::stdin()
    .read_line(&mut confirmation)
    .expect("Cannot read input.");

  if confirmation.trim().to_lowercase() == "y" {
    package.write_package()?;

    let repo_root = ".";
    let repo = Repository::open(repo_root).expect("Couldn't open repository");

    let relative_path = Path::new(package.get_location());
    let should_tag = !matches.is_present("tag");

    match versioning::add_and_commit(&repo, relative_path, &version) {
      Ok(oid) => {
        if should_tag {
          versioning::tag_commit(&version, &repo, &oid)?;
        };
      }
      Err(err) => {
        println!("Failed to commit: {}", err);
      }
    }
  } else {
    println!("Update aborted");
  }

  Ok(())
}
