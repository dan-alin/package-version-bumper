use git2::{Commit, ObjectType, Oid, Repository};
use std::path::Path;
use std::process::Command;

pub struct GitOptions<'a> {
  pub tag_prefix: &'a str,
  pub commit_prefix: &'a str,
  pub remote: &'a str,
  pub no_sign: bool,
}

fn find_last_commit(repo: &Repository) -> Result<Commit<'_>, git2::Error> {
  let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
  obj
    .into_commit()
    .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn add_and_commit(
  repo: &Repository,
  path: &Path,
  version: &str,
  should_tag: bool,
  should_push: bool,
  options: &GitOptions,
) -> Result<Oid, git2::Error> {
  let workdir = repo
    .workdir()
    .ok_or_else(|| git2::Error::from_str("bare repositories are not supported"))?;

  let mut index = repo.index()?;
  index.add_path(path)?;
  let oid = index.write_tree()?;
  let commit_message = format!("{}{}", options.commit_prefix, version);
  let parent_commit = find_last_commit(repo)?;
  let tree = repo.find_tree(oid)?;

  match repo.commit(
    Some("HEAD"),
    &repo.signature()?,
    &repo.signature()?,
    &commit_message,
    &tree,
    &[&parent_commit],
  ) {
    Ok(commit_id) => {
      let mut tag_name = None;
      if should_tag {
        let name = format!("{}{}", options.tag_prefix, version);
        let tag_message = format!("Release {}", name);
        let tag_flag = if options.no_sign { "-a" } else { "-s" };
        let status = Command::new("git")
          .current_dir(workdir)
          .arg("tag")
          .arg(tag_flag)
          .arg(&name)
          .arg("-m")
          .arg(&tag_message)
          .arg(commit_id.to_string())
          .status();

        let err_msg = if options.no_sign {
          "Failed to create annotated tag"
        } else {
          "Failed to create signed tag"
        };
        match status {
          Ok(s) if s.success() => {
            tag_name = Some(name);
          }
          _ => return Err(git2::Error::from_str(err_msg)),
        }
      };

      if should_push {
        let status = Command::new("git")
          .current_dir(workdir)
          .arg("push")
          .arg(options.remote)
          .status();

        match status {
          Ok(s) if s.success() => {}
          _ => return Err(git2::Error::from_str("Failed to push commit")),
        }

        if let Some(name) = tag_name {
          let status = Command::new("git")
            .current_dir(workdir)
            .arg("push")
            .arg(options.remote)
            .arg(&name)
            .status();

          match status {
            Ok(s) if s.success() => {}
            _ => return Err(git2::Error::from_str("Failed to push tag")),
          }
        }
      }

      index.write()?;
      Ok(commit_id)
    }
    Err(err) => Err(err),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use git2::{Repository, Signature};
  use std::fs;
  use tempfile::tempdir;

  fn init_repo_with_commit(dir: &std::path::Path) -> Repository {
    let repo = Repository::init(dir).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test").unwrap();
    cfg.set_str("user.email", "test@test.com").unwrap();
    drop(cfg);

    let sig = Signature::now("Test", "test@test.com").unwrap();
    fs::write(dir.join("init.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("init.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    index.write().unwrap();
    {
      let tree = repo.find_tree(tree_id).unwrap();
      repo
        .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();
    }
    repo
  }

  #[test]
  fn commit_message_uses_commit_prefix() {
    let dir = tempdir().unwrap();
    let repo = init_repo_with_commit(dir.path());

    fs::write(dir.path().join("version.txt"), "1.2.4").unwrap();

    add_and_commit(
      &repo,
      Path::new("version.txt"),
      "1.2.4",
      false,
      false,
      &GitOptions {
        tag_prefix: "v",
        commit_prefix: "release: ",
        remote: "origin",
        no_sign: false,
      },
    )
    .unwrap();

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.message().unwrap(), "release: 1.2.4");
  }

  // Tag creation uses a `git tag -s` subprocess that runs in the process CWD,
  // not the temp repo, and requires GPG signing to be configured. Run manually
  // with `cargo test -- --include-ignored` on a machine with GPG set up.
  #[test]
  #[ignore]
  fn tag_name_uses_tag_prefix() {
    let dir = tempdir().unwrap();
    let repo = init_repo_with_commit(dir.path());

    fs::write(dir.path().join("version.txt"), "1.2.4").unwrap();

    let result = add_and_commit(
      &repo,
      Path::new("version.txt"),
      "1.2.4",
      true,
      false,
      &GitOptions {
        tag_prefix: "release-",
        commit_prefix: "chore(version): ",
        remote: "origin",
        no_sign: false,
      },
    );

    assert!(result.is_ok(), "add_and_commit with tag should succeed");
    assert!(
      repo.find_reference("refs/tags/release-1.2.4").is_ok(),
      "tag refs/tags/release-1.2.4 should exist"
    );
  }

  #[test]
  fn tag_name_uses_tag_prefix_no_sign() {
    let dir = tempdir().unwrap();
    let repo = init_repo_with_commit(dir.path());

    fs::write(dir.path().join("version.txt"), "2.0.0").unwrap();

    let result = add_and_commit(
      &repo,
      Path::new("version.txt"),
      "2.0.0",
      true,
      false,
      &GitOptions {
        tag_prefix: "release-",
        commit_prefix: "chore(version): ",
        remote: "origin",
        no_sign: true,
      },
    );

    assert!(
      result.is_ok(),
      "add_and_commit with --no-sign tag should succeed"
    );
    assert!(
      repo.find_reference("refs/tags/release-2.0.0").is_ok(),
      "tag refs/tags/release-2.0.0 should exist"
    );
  }
}
