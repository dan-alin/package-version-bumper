use git2::{Commit, ObjectType, Oid, Repository};
use std::path::Path;
use std::process::Command;

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
) -> Result<Oid, git2::Error> {
  let mut index = repo.index()?;
  index.add_path(path)?;
  let oid = index.write_tree()?;
  let commit_message = format!("chore(version): {}", version);
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
        let name = format!("v{}", version);
        let tag_message = format!("Release v{}", version);
        let status = Command::new("git")
          .arg("tag")
          .arg("-s")
          .arg(&name)
          .arg("-m")
          .arg(&tag_message)
          .arg(commit_id.to_string())
          .status();

        match status {
          Ok(s) if s.success() => {
            tag_name = Some(name);
          }
          _ => return Err(git2::Error::from_str("Failed to create signed tag")),
        }
      };

      if should_push {
        // Push the current branch
        let status = Command::new("git").arg("push").status();

        match status {
          Ok(s) if s.success() => {}
          _ => return Err(git2::Error::from_str("Failed to push commit")),
        }

        // Push the tag if it was created
        if let Some(name) = tag_name {
          let status = Command::new("git")
            .arg("push")
            .arg("origin")
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
