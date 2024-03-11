use clap::{App, Arg};
use git2::{Commit, ObjectType, Oid, Repository, Signature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use std::path::Path;

mod version;

#[derive(Debug, Serialize, Deserialize)]
struct Package {
  version: version::Version,
  #[serde(flatten)]
  other: HashMap<String, serde_json::Value>,
}

fn main() -> serde_json::Result<()> {
  let data = fs::read_to_string("package.json").expect("Unable to read file");

  let mut package_json: Package =
    serde_json::from_str(&data).expect("Package json should contain a version field.");

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
    // .arg(
    //     Arg::with_name("file")
    //         .short('f')
    //         .long("file")
    //         .value_name("FILE_PATH")
    //         .help("Path to the file to open")
    //         .takes_value(true)
    //         .required(true),
    // )
    .get_matches();

  match (
    matches.is_present("major"),
    matches.is_present("minor"),
    matches.is_present("patch"),
  ) {
    (true, _, _) => package_json.version.increment_major(),
    (_, true, _) => package_json.version.increment_minor(),
    (_, _, true) => package_json.version.increment_patch(),
    _ => {
      println!("No version increment option specified.");
      return Ok(());
    }
  };

  let version = package_json.version.to_string();

  println!("Update the version to: {}? [y/N]", version);

  let mut confirmation = String::new();
  std::io::stdin()
    .read_line(&mut confirmation)
    .expect("Cannot read input.");

  if confirmation.trim().to_lowercase() == "y" {
    let value = serde_json::to_string_pretty(&package_json)?;

    fs::write("package.json", value).expect("Unable to write file");

    let repo_root = ".";
    let repo = Repository::open(repo_root).expect("Couldn't open repository");

    let relative_path = Path::new("package.json");

    add_and_commit(&repo, relative_path, &version).expect("Couldn't add file to repo");
  } else {
    println!("Update aborted");
  }

  Ok(())
}

fn add_and_commit(repo: &Repository, path: &Path, version: &str) -> Result<Oid, git2::Error> {
  let mut index = repo.index()?;
  index.add_path(path)?;
  let oid = index.write_tree()?;
  let message = format!("build: {}", version);
  let signature = Signature::now("alin", "danalin06@gmail.com")?;
  let parent_commit = find_last_commit(repo)?;
  let tree = repo.find_tree(oid)?;

  match repo.commit(
    Some("HEAD"), // point HEAD to our new commit
    &signature,
    &signature,
    &message,
    &tree,
    &[&parent_commit],
  ) {
    Ok(commit_id) => {
      index.write()?;
      Ok(commit_id)
    }
    Err(err) => Err(err),
  }
}

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
  let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
  obj
    .into_commit()
    .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}
