use std::path::Path;

use clap::{App, Arg};
use git2::Repository;

mod config;
mod gitops;
mod packages;
mod release_notes;
mod version;

use packages::PackageUtils;

const RELEASE_NOTES_DIR: &str = "release-notes";

fn today_iso() -> String {
  chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn main() -> anyhow::Result<()> {
  let release_notes_subcommand = App::new("release-notes")
    .about("Generate release notes from git history and print to stdout")
    .subcommand(
      App::new("changelog")
        .about("List all commits since the last release grouped by type")
        .arg(
          Arg::with_name("from")
            .long("from")
            .takes_value(true)
            .help("Start commit range from this git ref (tag, SHA, or branch)"),
        ),
    )
    .subcommand(
      App::new("structured")
        .about("Extract item references matching configured patterns")
        .arg(
          Arg::with_name("from")
            .long("from")
            .takes_value(true)
            .help("Start commit range from this git ref (tag, SHA, or branch)"),
        ),
    );

  let matches = App::new("vup")
    .subcommand(
      App::new("init").about("Create a vup.toml config file").arg(
        Arg::with_name("yes")
          .long("yes")
          .help("Write default config non-interactively (for CI/scripts)"),
      ),
    )
    .subcommand(release_notes_subcommand)
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
        .long("tag")
        .help("Tag the commit with the new version"),
    )
    .arg(
      Arg::with_name("push")
        .short('u')
        .long("push")
        .help("Push the commit and the tag to the remote"),
    )
    .arg(
      Arg::with_name("yes")
        .short('y')
        .long("yes")
        .help("Skip the confirmation prompt (for CI use)"),
    )
    .arg(
      Arg::with_name("no-sign")
        .long("no-sign")
        .help("Create an annotated tag instead of a GPG-signed tag"),
    )
    .arg(
      Arg::with_name("release-notes")
        .short('r')
        .long("release-notes")
        .help("Generate release notes and commit them alongside the version bump"),
    )
    .arg(
      Arg::with_name("from")
        .long("from")
        .takes_value(true)
        .requires("release-notes")
        .help(
          "Start release notes from this git ref (tag, SHA, or branch); implies --release-notes",
        ),
    )
    .get_matches();

  if let Some(init_matches) = matches.subcommand_matches("init") {
    let path = Path::new("vup.toml");
    if path.exists() {
      println!("vup.toml already exists — skipping to avoid overwriting your config.");
      return Ok(());
    }
    let config = if init_matches.is_present("yes") {
      config::Config::default()
    } else {
      config::Config::from_prompts(&mut std::io::stdin().lock())?
    };
    config.write(path)?;
    let pkg_display = config
      .package
      .as_deref()
      .map(|p| format!("\"{}\"", p))
      .unwrap_or_else(|| "(auto-detect)".to_owned());
    println!("Created vup.toml");
    println!("  tag_prefix     = \"{}\"", config.tag_prefix);
    println!("  commit_prefix  = \"{}\"", config.commit_prefix);
    println!("  package        = {}", pkg_display);
    println!("  push_remote    = \"{}\"", config.push_remote);
    return Ok(());
  }

  if let Some(rn_matches) = matches.subcommand_matches("release-notes") {
    let repo = Repository::open(".").map_err(|e| anyhow::anyhow!("not a git repository: {}", e))?;
    let config = config::Config::load()?;
    let mut package: Box<dyn PackageUtils> = packages::get_package(&config);
    let version = package.get_version().to_string();
    let date = today_iso();

    if let Some(cl_matches) = rn_matches.subcommand_matches("changelog") {
      let from = cl_matches.value_of("from");
      let commits = release_notes::collect_commits(&repo, from)?;
      let output = release_notes::generate_changelog(&version, &date, &commits);
      print!("{}", output);
      return Ok(());
    }

    if let Some(st_matches) = rn_matches.subcommand_matches("structured") {
      let from = st_matches.value_of("from");
      let commits = release_notes::collect_commits(&repo, from)?;
      let output = release_notes::generate_structured(
        &version,
        &date,
        &commits,
        &config.release_notes.patterns,
      )?;
      print!("{}", output);
      return Ok(());
    }

    println!("Usage: vup release-notes <changelog|structured> [--from <ref>]");
    return Ok(());
  }

  let wants_release_notes = matches.is_present("release-notes");
  let has_bump =
    matches.is_present("major") || matches.is_present("minor") || matches.is_present("patch");

  if wants_release_notes && !has_bump {
    anyhow::bail!("--release-notes requires a version bump flag (--major, --minor, or --patch)");
  }

  if matches.is_present("verbose") {
    env_logger::init();
    log::info!("Starting pvb");
  }

  let config = config::Config::load()?;
  let mut package: Box<dyn PackageUtils> = packages::get_package(&config);

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

  let confirmed = if matches.is_present("yes") {
    true
  } else {
    println!("Update the version to: {}? [y/N]", version);
    let mut input = String::new();
    std::io::stdin()
      .read_line(&mut input)
      .expect("Cannot read input.");
    input.trim().to_lowercase() == "y"
  };

  if confirmed {
    package.write_package()?;

    let repo = Repository::open(".").expect("Couldn't open repository");
    let package_path = Path::new(package.get_location());
    let should_tag = matches.is_present("tag");
    let should_push = matches.is_present("push");
    let no_sign = matches.is_present("no-sign");

    let notes_path_str;
    let mut paths_to_commit: Vec<&Path> = vec![package_path];

    if wants_release_notes {
      let date = today_iso();
      let from = matches.value_of("from");
      let commits = release_notes::collect_commits(&repo, from)?;
      let notes =
        release_notes::generate_for_version(&version, &date, &commits, &config.release_notes)?;
      release_notes::write_release_notes(Path::new(RELEASE_NOTES_DIR), &version, &notes)?;
      notes_path_str = format!("{}/{}.md", RELEASE_NOTES_DIR, version);
      paths_to_commit.push(Path::new(&notes_path_str));
      println!("Release notes written to {}", notes_path_str);
    }

    gitops::add_and_commit(
      &repo,
      &paths_to_commit,
      &version,
      should_tag,
      should_push,
      &gitops::GitOptions {
        tag_prefix: &config.tag_prefix,
        commit_prefix: &config.commit_prefix,
        remote: &config.push_remote,
        no_sign,
      },
    )
    .expect("Couldn't commit");
  } else {
    println!("Update aborted");
  }

  Ok(())
}
