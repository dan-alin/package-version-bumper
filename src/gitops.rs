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
      if should_tag {
        let tag_name = format!("v{}", version);
        let tag_message = format!("Release v{}", version);
        let status = Command::new("git")
          .arg("tag")
          .arg("-s")
          .arg(&tag_name)
          .arg("-m")
          .arg(&tag_message)
          .arg(commit_id.to_string())
          .status();

        match status {
          Ok(s) if s.success() => {}
          _ => return Err(git2::Error::from_str("Failed to create signed tag")),
        }
      };

      index.write()?;
      Ok(commit_id)
    }
    Err(err) => Err(err),
  }
}
