use git2::{Commit, ObjectType, Oid, Repository};
use std::path::Path;

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
  let message = format!("chore(version): {}", version);
  let parent_commit = find_last_commit(repo)?;
  let tree = repo.find_tree(oid)?;

  match repo.commit(
    Some("HEAD"),
    &repo.signature()?,
    &repo.signature()?,
    &message,
    &tree,
    &[&parent_commit],
  ) {
    Ok(commit_id) => {
      if should_tag {
        repo.tag(
          &format!("v{}", version),
          &repo.find_object(commit_id, None)?,
          &repo.signature()?,
          &message,
          false,
        )?;
      };

      index.write()?;
      Ok(commit_id)
    }
    Err(err) => Err(err),
  }
}
