use clap::{App, Arg};
use git2::{Commit, ObjectType, Oid, Repository, Signature};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

fn main() -> std::io::Result<()> {
  let mut line = String::new();
  let mut current_position = 0;
  let mut old_version = String::new();

  let mut splitting_char_map: HashMap<String, char> = HashMap::new();
  //TODO Handle config for different extension in the map
  // for example key, quotes, etc..
  // {
  //    key: "version"
  //    splitting_char: '='
  //
  // }
  splitting_char_map.insert(String::from("package.json"), ':');
  splitting_char_map.insert(String::from("cargo.toml"), '=');

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

  let path = matches.value_of("file").unwrap_or("package.json");

  let mut file = OpenOptions::new().read(true).write(true).open(&path)?;

  let mut reader = BufReader::new(&file);

  while reader.read_line(&mut line)? > 0 {
    if line.trim_start().to_lowercase().contains("version") {
      let splitting_char = match splitting_char_map.get(path) {
        Some(character) => *character,
        None => ':',
      };

      let parts: Vec<&str> = line.split(splitting_char).collect();
      println!(" splitting {} line {}", splitting_char, line);

      if parts.len() >= 2 {
        old_version = parts[1]
          .trim()
          .chars()
          .filter(|c| !(*c == '"' || *c == '\n' || *c == ','))
          .collect::<String>();
        break;
      } else {
        println!("Invalid format for version line in package.json");
        return Ok(());
      }
    }
    current_position += line.len() + 1;
    line.clear();
  }

  let version = match (
    matches.is_present("major"),
    matches.is_present("minor"),
    matches.is_present("patch"),
  ) {
    (true, _, _) => increment_major_version(&old_version),
    (_, true, _) => increment_minor_version(&old_version),
    (_, _, true) => increment_patch_version(&old_version),
    _ => {
      println!("No version increment option specified.");
      return Ok(());
    }
  };
  println!("Current version is: {}", old_version);
  println!("Update the version to: {}? [y/N]", version);
  let test = String::from("test");

  println!("{}", test);
  let mut confirmation = String::new();
  std::io::stdin().read_line(&mut confirmation)?;

  if confirmation.trim().to_lowercase() == "y" {
    // Seek to the version line position and overwrite the entire line
    file.seek(SeekFrom::Start(current_position as u64))?;
    let updated_line = format!("\"version\": \"{}\"", version);
    file.write_all(updated_line.as_bytes())?;
    println!("Version updated successfully!");

    let repo_root = ".";
    let repo = Repository::open(repo_root).expect("Couldn't open repository");

    let relative_path = Path::new(&path);

    add_and_commit(&repo, &relative_path, &version).expect("Couldn't add file to repo");
  } else {
    println!("Update aborted");
  }

  Ok(())
}

fn increment_major_version(current_version: &str) -> String {
  let parts: Vec<&str> = current_version.split('.').collect();
  let major: u64 = parts[0].parse().unwrap_or(0);
  format!("{}.{}.{}", major + 1, 0, 0)
}

fn increment_minor_version(current_version: &str) -> String {
  let parts: Vec<&str> = current_version.split('.').collect();
  let major: u64 = parts[0].parse().unwrap_or(0);
  let minor: u64 = parts[1].parse().unwrap_or(0);
  format!("{}.{}.{}", major, minor + 1, 0)
}

fn increment_patch_version(current_version: &str) -> String {
  let parts: Vec<&str> = current_version.split('.').collect();
  let major: u64 = parts[0].parse().unwrap_or(0);
  let minor: u64 = parts[1].parse().unwrap_or(0);
  let patch: u64 = parts[2].parse().unwrap_or(0);
  format!("{}.{}.{}", major, minor, patch + 1)
}

fn add_and_commit(repo: &Repository, path: &Path, version: &str) -> Result<Oid, git2::Error> {
  let mut index = repo.index()?;
  index.add_path(path)?;
  let oid = index.write_tree()?;
  let message = format!("build: {}", version);
  let signature = Signature::now("alin", "danalin06@gmail.com")?;
  let parent_commit = find_last_commit(&repo)?;
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
