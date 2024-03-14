use git2::{Commit, ObjectType, Oid, Repository};
use std::path::Path;

fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
  let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
  obj
    .into_commit()
    .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn add_and_commit(repo: &Repository, path: &Path, version: &str) -> Result<Oid, git2::Error> {
  let mut index = repo.index()?;
  index.add_path(path)?;
  let oid = index.write_tree()?;
  let message = format!("build: {}", version);
  let parent_commit = find_last_commit(repo)?;
  let tree = repo.find_tree(oid)?;

  match repo.commit(
    Some("HEAD"), // point HEAD to our new commit
    &repo.signature()?,
    &repo.signature()?,
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

pub fn tag_commit(version: &str, repo: &Repository, oid: &Oid) -> Result<Oid, git2::Error> {
  // Get the branch you want to tag
  //let branch = repo.find_branch("main", git2::BranchType::Local)?;
  let mut index = repo.index()?;
  // Get the OID (object ID) of the branch reference
  //let commit = branch.get().peel_to_commit()?;

  let object = repo.find_object(*oid, None)?;

  // Create a tag
  match repo.tag(version, &object, &repo.signature()?, "", false) {
    Ok(tag_id) => {
      index.write()?;
      Ok(tag_id)
    }
    Err(err) => {
      println!("Failed to tag: {}", err);
      Err(err)
    }
  }
}
