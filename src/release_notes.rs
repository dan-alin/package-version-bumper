use anyhow::{anyhow, Result};
use git2::{ObjectType, Repository};
use regex::Regex;
use std::collections::HashSet;

use crate::config::PatternConfig;

#[derive(Debug, Clone)]
pub struct CommitInfo {
  pub subject: String,
  pub body: String,
}

pub fn collect_commits(repo: &Repository, from_ref: Option<&str>) -> Result<Vec<CommitInfo>> {
  let head_commit = repo
    .head()
    .map_err(|e| anyhow!("cannot read HEAD: {}", e))?
    .peel_to_commit()
    .map_err(|e| anyhow!("HEAD is not a commit: {}", e))?;

  let stop_oid = match from_ref {
    Some(r) => {
      let obj = repo
        .revparse_single(r)
        .map_err(|_| anyhow!("invalid ref: '{}'", r))?;
      let commit = obj
        .peel(ObjectType::Commit)
        .map_err(|_| anyhow!("ref '{}' does not point to a commit", r))?;
      Some(commit.id())
    }
    None => find_last_tag_commit(repo),
  };

  let mut revwalk = repo.revwalk()?;
  revwalk.push(head_commit.id())?;

  let mut commits = Vec::new();
  for oid_result in revwalk {
    let oid = oid_result?;
    if Some(oid) == stop_oid {
      break;
    }
    let commit = repo.find_commit(oid)?;
    let message = commit.message().unwrap_or("").to_string();
    let (subject, body) = split_message(&message);
    commits.push(CommitInfo { subject, body });
  }

  Ok(commits)
}

fn split_message(message: &str) -> (String, String) {
  let mut iter = message.splitn(2, '\n');
  let subject = iter.next().unwrap_or("").trim_end().to_string();
  let body = iter
    .next()
    .unwrap_or("")
    .trim_start_matches('\n')
    .trim_end()
    .to_string();
  (subject, body)
}

fn find_last_tag_commit(repo: &Repository) -> Option<git2::Oid> {
  let tag_names = repo.tag_names(None).ok()?;

  let mut tagged_oids: HashSet<git2::Oid> = HashSet::new();
  for name in tag_names.iter().flatten() {
    let full_ref = format!("refs/tags/{}", name);
    if let Ok(obj) = repo.revparse_single(&full_ref) {
      if let Ok(commit) = obj.peel(ObjectType::Commit) {
        tagged_oids.insert(commit.id());
      }
    }
  }

  if tagged_oids.is_empty() {
    return None;
  }

  let head = repo.head().ok()?.peel_to_commit().ok()?;
  let mut revwalk = repo.revwalk().ok()?;
  revwalk.push(head.id()).ok()?;

  for oid_result in revwalk {
    let oid = oid_result.ok()?;
    if tagged_oids.contains(&oid) {
      return Some(oid);
    }
  }

  None
}

fn render_sections(header: &str, sections: Vec<(&str, Vec<String>)>) -> String {
  let mut output = format!("{}\n", header);
  for (name, items) in &sections {
    if !items.is_empty() {
      output.push_str(&format!("\n### {}\n", name));
      for item in items {
        output.push_str(&format!("- {}\n", item));
      }
    }
  }
  output
}

const KNOWN_PREFIXES: &[(&str, &str)] = &[("feat", "Features"), ("fix", "Bug Fixes")];

struct PrefixSection {
  prefix: &'static str,
  heading: &'static str,
  items: Vec<String>,
}

pub fn generate_changelog(version: &str, date: &str, commits: &[CommitInfo]) -> String {
  let mut prefix_sections: Vec<PrefixSection> = KNOWN_PREFIXES
    .iter()
    .map(|(prefix, heading)| PrefixSection {
      prefix,
      heading,
      items: Vec::new(),
    })
    .collect();
  let mut other: Vec<String> = Vec::new();

  for commit in commits {
    let subject = &commit.subject;
    let matched = if let Some(colon_pos) = subject.find(':') {
      let prefix = subject[..colon_pos].trim();
      if let Some(entry) = prefix_sections.iter_mut().find(|s| s.prefix == prefix) {
        entry
          .items
          .push(subject[colon_pos + 1..].trim().to_string());
        true
      } else {
        false
      }
    } else {
      false
    };

    if !matched {
      other.push(subject.clone());
    }
  }

  let header = format!("## v{} \u{2014} {}", version, date);
  let mut sections: Vec<(&str, Vec<String>)> = prefix_sections
    .into_iter()
    .map(|s| (s.heading, s.items))
    .collect();
  if !other.is_empty() {
    sections.push(("Other", other));
  }
  render_sections(&header, sections)
}

pub fn generate_structured(
  version: &str,
  date: &str,
  commits: &[CommitInfo],
  patterns: &[PatternConfig],
) -> Result<String> {
  let compiled: Result<Vec<(Regex, usize)>> = patterns
    .iter()
    .enumerate()
    .map(|(i, p)| {
      Regex::new(&p.regex)
        .map_err(|_| anyhow!("invalid regex pattern: '{}'", p.regex))
        .map(|r| (r, i))
    })
    .collect();
  let compiled = compiled?;

  let mut buckets: Vec<Vec<String>> = patterns.iter().map(|_| Vec::new()).collect();

  for commit in commits {
    let text = if commit.body.is_empty() {
      commit.subject.clone()
    } else {
      format!("{}\n{}", commit.subject, commit.body)
    };
    for (regex, idx) in &compiled {
      for cap in regex.captures_iter(&text) {
        let item = match &patterns[*idx].template {
          None => cap[0].to_string(),
          Some(tmpl) => {
            let mut out = String::new();
            cap.expand(tmpl, &mut out);
            out
          }
        };
        if item.is_empty() {
          continue;
        }
        if !buckets[*idx].contains(&item) {
          buckets[*idx].push(item);
        }
      }
    }
  }

  let header = format!("## v{} \u{2014} {}", version, date);
  let sections: Vec<(&str, Vec<String>)> = patterns
    .iter()
    .zip(buckets)
    .map(|(p, items)| (p.section.as_str(), items))
    .collect();
  Ok(render_sections(&header, sections))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::config::PatternConfig;
  use git2::{Repository, Signature};
  use std::fs;
  use tempfile::tempdir;

  fn init_repo(dir: &std::path::Path) -> Repository {
    let repo = Repository::init(dir).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test").unwrap();
    cfg.set_str("user.email", "test@test.com").unwrap();
    drop(cfg);

    let sig = Signature::now("Test", "test@test.com").unwrap();
    fs::write(dir.join("file.txt"), "init").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo
      .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
      .unwrap();
    drop(tree);
    repo
  }

  fn add_commit(repo: &Repository, message: &str, content: &str) -> git2::Oid {
    let sig = Signature::now("Test", "test@test.com").unwrap();
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join("file.txt"), content).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo
      .commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
      .unwrap()
  }

  fn add_lightweight_tag(repo: &Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.tag_lightweight(name, head.as_object(), false).unwrap();
  }

  // --- collect_commits ---

  #[test]
  fn no_tags_returns_all_commits() {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    add_commit(&repo, "feat: one", "a");
    add_commit(&repo, "feat: two", "b");

    let commits = collect_commits(&repo, None).unwrap();
    assert_eq!(commits.len(), 3);
    assert_eq!(commits[0].subject, "feat: two");
    assert_eq!(commits[2].subject, "Initial commit");
  }

  #[test]
  fn tag_present_returns_only_commits_after_tag() {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    add_commit(&repo, "chore(version): 1.0.0", "v1");
    add_lightweight_tag(&repo, "v1.0.0");
    add_commit(&repo, "feat: new feature", "a");
    add_commit(&repo, "fix: a bug", "b");

    let commits = collect_commits(&repo, None).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "fix: a bug");
    assert_eq!(commits[1].subject, "feat: new feature");
  }

  #[test]
  fn explicit_from_ref_uses_given_ref() {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    add_commit(&repo, "feat: alpha", "a");
    add_lightweight_tag(&repo, "v1.0.0");
    add_commit(&repo, "feat: beta", "b");
    add_commit(&repo, "feat: gamma", "c");

    let commits = collect_commits(&repo, Some("v1.0.0")).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "feat: gamma");
    assert_eq!(commits[1].subject, "feat: beta");
  }

  #[test]
  fn invalid_from_ref_returns_error() {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let result = collect_commits(&repo, Some("refs/tags/does-not-exist"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("invalid ref"));
  }

  #[test]
  fn commit_info_holds_subject_and_body() {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    let sig = Signature::now("Test", "test@test.com").unwrap();
    let workdir = repo.workdir().unwrap().to_path_buf();
    fs::write(workdir.join("file.txt"), "x").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo
      .commit(
        Some("HEAD"),
        &sig,
        &sig,
        "feat: subject line\n\nBody text here.",
        &tree,
        &[&parent],
      )
      .unwrap();

    let commits = collect_commits(&repo, None).unwrap();
    assert_eq!(commits[0].subject, "feat: subject line");
    assert!(commits[0].body.contains("Body text here."));
  }

  // --- generate_changelog ---

  fn ci(subject: &str) -> CommitInfo {
    CommitInfo {
      subject: subject.to_string(),
      body: String::new(),
    }
  }

  #[test]
  fn changelog_groups_feat_under_features() {
    let commits = vec![ci("feat: add login page")];
    let out = generate_changelog("1.3.0", "2026-08-27", &commits);
    assert!(out.contains("### Features"));
    assert!(out.contains("- add login page"));
  }

  #[test]
  fn changelog_groups_fix_under_bug_fixes() {
    let commits = vec![ci("fix: redirect loop")];
    let out = generate_changelog("1.3.0", "2026-08-27", &commits);
    assert!(out.contains("### Bug Fixes"));
    assert!(out.contains("- redirect loop"));
  }

  #[test]
  fn changelog_unrecognised_prefix_goes_to_other() {
    let commits = vec![ci("chore: update dependencies")];
    let out = generate_changelog("1.3.0", "2026-08-27", &commits);
    assert!(out.contains("### Other"));
    assert!(out.contains("- chore: update dependencies"));
  }

  #[test]
  fn changelog_header_format() {
    let out = generate_changelog("1.3.0", "2026-08-27", &[]);
    assert!(out.starts_with("## v1.3.0 \u{2014} 2026-08-27"));
  }

  #[test]
  fn changelog_empty_commits_produces_only_header() {
    let out = generate_changelog("1.3.0", "2026-08-27", &[]);
    assert!(out.trim().starts_with("## v1.3.0"));
    assert!(!out.contains("###"));
  }

  #[test]
  fn changelog_no_prefix_goes_to_other() {
    let commits = vec![ci("some random commit message")];
    let out = generate_changelog("1.0.0", "2026-01-01", &commits);
    assert!(out.contains("### Other"));
    assert!(out.contains("- some random commit message"));
  }

  // --- generate_structured ---

  fn pattern(regex: &str, section: &str) -> PatternConfig {
    PatternConfig {
      regex: regex.to_string(),
      section: section.to_string(),
      template: None,
    }
  }

  fn pattern_with_template(regex: &str, section: &str, template: &str) -> PatternConfig {
    PatternConfig {
      regex: regex.to_string(),
      section: section.to_string(),
      template: Some(template.to_string()),
    }
  }

  fn ci_body(subject: &str, body: &str) -> CommitInfo {
    CommitInfo {
      subject: subject.to_string(),
      body: body.to_string(),
    }
  }

  #[test]
  fn structured_commit_matching_one_pattern_appears_in_one_section() {
    let commits = vec![ci("feat: add login #US-17")];
    let patterns = vec![pattern(r"#US-\d+", "User Stories")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("### User Stories"));
    assert!(out.contains("- #US-17"));
  }

  #[test]
  fn structured_commit_matching_two_patterns_appears_in_both() {
    let commits = vec![ci("feat: impl story #US-17 fixes #BUG-42")];
    let patterns = vec![
      pattern(r"#US-\d+", "User Stories"),
      pattern(r"#BUG-\d+", "Bug Fixes"),
    ];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("### User Stories"));
    assert!(out.contains("- #US-17"));
    assert!(out.contains("### Bug Fixes"));
    assert!(out.contains("- #BUG-42"));
  }

  #[test]
  fn structured_commit_matching_no_pattern_is_omitted() {
    let commits = vec![ci("chore: update deps")];
    let patterns = vec![pattern(r"#US-\d+", "User Stories")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(!out.contains("### User Stories"));
    assert!(!out.contains("chore"));
  }

  #[test]
  fn structured_output_items_are_refs_not_prose() {
    let commits = vec![ci("feat: implement something important #US-23")];
    let patterns = vec![pattern(r"#US-\d+", "User Stories")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("- #US-23"));
    assert!(!out.contains("implement something important"));
  }

  #[test]
  fn structured_sections_with_no_matches_not_emitted() {
    let commits = vec![ci("feat: something #US-1")];
    let patterns = vec![
      pattern(r"#US-\d+", "User Stories"),
      pattern(r"#BUG-\d+", "Bug Fixes"),
    ];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("### User Stories"));
    assert!(!out.contains("### Bug Fixes"));
  }

  #[test]
  fn structured_no_patterns_produces_only_header() {
    let commits = vec![ci("feat: something")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &[]).unwrap();
    assert!(out.trim().starts_with("## v1.0.0"));
    assert!(!out.contains("###"));
  }

  #[test]
  fn structured_header_format() {
    let out = generate_structured("2.0.0", "2026-08-27", &[], &[]).unwrap();
    assert!(out.starts_with("## v2.0.0 \u{2014} 2026-08-27"));
  }

  #[test]
  fn structured_matches_in_body_are_captured() {
    let commits = vec![ci_body("feat: some feature", "Implements #US-99")];
    let patterns = vec![pattern(r"#US-\d+", "User Stories")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("- #US-99"));
  }

  #[test]
  fn structured_template_substitutes_capture_group() {
    let commits = vec![ci("feat: add login #US-17")];
    let patterns = vec![pattern_with_template(r"#US-(\d+)", "User Stories", "#$1")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("- #17"));
    assert!(!out.contains("#US-17"));
  }

  #[test]
  fn structured_template_whole_match_via_dollar_zero() {
    let commits = vec![ci("feat: thing #US-17")];
    let patterns = vec![pattern_with_template(r"#US-\d+", "User Stories", "$0")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("- #US-17"));
  }

  #[test]
  fn structured_no_template_uses_full_match() {
    let commits = vec![ci("feat: add login #US-17")];
    let patterns = vec![pattern(r"#US-\d+", "User Stories")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(out.contains("- #US-17"));
  }

  #[test]
  fn structured_template_empty_expansion_is_skipped() {
    // $9 references a non-existent group → expand produces "" → item is skipped
    let commits = vec![ci("feat: thing #US-17")];
    let patterns = vec![pattern_with_template(r"#US-\d+", "User Stories", "$9")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert!(!out.contains("### User Stories"));
  }

  #[test]
  fn structured_template_deduplicates_within_section() {
    let commits = vec![ci("feat: one #US-17"), ci("fix: two #US-17")];
    let patterns = vec![pattern_with_template(r"#US-(\d+)", "User Stories", "#$1")];
    let out = generate_structured("1.0.0", "2026-08-27", &commits, &patterns).unwrap();
    assert_eq!(out.matches("- #17").count(), 1);
  }

  #[test]
  fn structured_invalid_regex_returns_error() {
    let patterns = vec![pattern(r"[invalid", "Bad")];
    let result = generate_structured("1.0.0", "2026-08-27", &[], &patterns);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("[invalid"));
  }
}
