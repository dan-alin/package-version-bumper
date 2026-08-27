# Spec: `vup release-notes` command

## Problem Statement

Developers and CI engineers running `vup` have no way to generate release notes as part of the version bump workflow. Writing release documentation manually is error-prone and time-consuming. Teams that track work through item references (user stories, bugs) in commit messages have no automated way to extract a structured list of shipped items for a given release.

## Solution

Introduce a `vup release-notes` subcommand with two modes:

- **`changelog`** — lists all commits since the last git tag (or a given ref), grouped by conventional commit prefix, in Markdown.
- **`structured`** — extracts only commits that match user-defined patterns, outputting item references (e.g. `#US-17`, `#BUG-42`) grouped by section, with no commit message prose.

Output goes to stdout so pipelines can redirect it as an artifact (`vup release-notes structured > release-notes.md`). The command is fully independent of the version bump workflow.

## User Stories

1. As a developer, I want to run `vup release-notes changelog` so that I get a Markdown list of all commits since the last release without writing anything manually.
2. As a developer, I want to run `vup release-notes structured` so that I get a clean list of user story and bug references shipped in this release.
3. As a CI engineer, I want to pipe `vup release-notes structured` to a file so that I can upload it as a build artifact without leaving uncommitted files in the working tree.
4. As a developer, I want the commit range to default to since the last git tag so that I get exactly the commits in the current release.
5. As a developer, I want to pass `--from <ref>` so that I can generate notes from any git ref — tag, SHA, or branch name.
6. As a developer, I want to configure pattern-to-section mappings in `vup.toml` so that I can define what counts as a User Story, a Bug Fix, or any other category without changing source code.
7. As a developer, I want a commit that matches multiple patterns to appear under each matching section so that cross-cutting items (e.g. a feat that also has a `#US` ref) are not lost.
8. As a developer, I want unmatched commits to be silently omitted in structured mode so that the output contains only tracked items with no noise.
9. As a developer, I want unmatched commits to appear in the changelog mode output so that I have a complete picture of everything that shipped.
10. As a developer, I want the structured mode output to show only item references (e.g. `#US-17`), not commit message prose, so that the release note is clean and tool-parseable.
11. As a developer, I want the changelog mode output to show the full commit subject grouped by conventional prefix so that I have a readable narrative of changes.
12. As a developer, I want the output to be valid Markdown so that it renders correctly on GitHub, GitLab, and CI artifact viewers.
13. As a developer, I want the release notes header to include the version and date so that each release note is self-identifying.
14. As a developer, I want `vup release-notes` to work without a `vup.toml` present so that existing projects are not broken (changelog mode works with no config; structured mode with no patterns produces no output).
15. As a developer, I want the patterns to be defined as regex strings so that I am not locked into any specific ticket reference convention.
16. As a CI engineer, I want `vup release-notes` to exit with a non-zero code on failure (e.g. invalid ref, no git repo) so that my pipeline fails fast.

## Implementation Decisions

- **New `release_notes` module**: owns all release-note logic. `main.rs` adds a `release-notes` subcommand with two sub-subcommands: `changelog` and `structured`. Both accept an optional `--from <ref>` flag.

- **`collect_commits(repo, from_ref: Option<&str>) -> Result<Vec<CommitInfo>>`**: walks git history from `from_ref` (or the last tag if absent, or all commits if no tag exists) to HEAD using the `git2` crate. Returns a `Vec<CommitInfo>` — a simple struct holding the commit subject and full message body.

- **`generate_changelog(version: &str, commits: &[CommitInfo]) -> String`**: pure function. Groups commits by their conventional prefix (the text before the first `:`). Commits with no recognised prefix go into an "Other" section. Returns a Markdown string.

- **`generate_structured(version: &str, commits: &[CommitInfo], patterns: &[PatternConfig]) -> String`**: pure function. For each commit, tests every pattern regex against the commit subject and body; extracts all matches and assigns them to the corresponding section. A commit can contribute to multiple sections. Only sections with at least one match are emitted. Returns a Markdown string.

- **`PatternConfig`**: a new config type — `{ regex: String, section: String }`. Owned by `Config` under a `release_notes.patterns` array. Regex compilation happens once at call time, not at config load time, to keep `Config` cheaply constructible.

- **`Config` extension**: a new optional `[release_notes]` table with a `patterns` array. Absence of the table is valid and produces an empty pattern list. `deny_unknown_fields` is applied at the `release_notes` level too.

- **Output format**:

  Changelog mode:
  ```
  ## v1.3.0 — 2026-08-27

  ### Features
  - add login page

  ### Bug Fixes
  - fix redirect loop

  ### Other
  - chore: update dependencies
  ```

  Structured mode (item refs only, no prose):
  ```
  ## v1.3.0 — 2026-08-27

  ### User Stories
  - #US-17
  - #US-23

  ### Bug Fixes
  - #BUG-42
  ```

- **Version in header**: `generate_changelog` and `generate_structured` receive the current version string (read from the package file via the existing `get_package` logic) and the current date formatted as `YYYY-MM-DD`.

- **`--from` flag**: accepts any string and passes it directly to `git2` ref resolution. Invalid refs surface as an `anyhow::Error` with a clear message.

## Testing Decisions

A good test asserts on the string output of the pure generate functions or the `Vec<CommitInfo>` returned by `collect_commits` — not on internal regex state or intermediate maps.

- **`collect_commits`**: tested against a temp git repo initialised with `git2::Repository::init` (same pattern as existing `gitops` tests). Cases: no tags returns all commits; tag present returns only commits after tag; `--from` ref overrides the default; invalid ref returns an error.

- **`generate_changelog`**: tested with a hardcoded `Vec<CommitInfo>`. Cases: commits grouped by prefix; commits with no prefix go to "Other"; empty commit list produces only the header.

- **`generate_structured`**: tested with a hardcoded `Vec<CommitInfo>` and `Vec<PatternConfig>`. Cases: commit matching one pattern appears in one section; commit matching two patterns appears in both; commit matching no pattern is omitted; no patterns configured produces only the header (or empty output).

- **`Config` extension**: extend the existing `valid_toml_parses_all_fields` and `absent_file_returns_defaults` tests to cover the `release_notes.patterns` field.

## Out of Scope

- Writing the release notes file to disk — callers redirect stdout.
- Integration with GitHub Releases, GitLab Releases, or any external API.
- Amending or updating an existing `CHANGELOG.md` file.
- Grouping commits by author.
- Pre-release version support.
- Filtering by file path or subdirectory (monorepo per-package notes).

## Further Notes

- The `structured` mode is intentionally strict: if no patterns are configured, it produces no sections (only the header). This is the correct behaviour for CI — an unconfigured run produces empty output rather than a noisy dump.
- Regex compilation errors in `PatternConfig` should surface at note-generation time with the offending pattern quoted in the error message, so the user knows which entry in `vup.toml` to fix.
- The `git2` crate is already a dependency; no new dependencies are needed for git traversal. A regex crate (`regex`) is already a dependency.
