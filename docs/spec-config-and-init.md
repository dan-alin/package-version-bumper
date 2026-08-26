# Spec: `vup init`, `vup.toml` config file, and CLI improvements

## Problem Statement

Users running `vup` in diverse project setups have no way to customise its behaviour: the commit message format, tag prefix, which package file to use, and which git remote to push to are all hard-coded. Running `vup` in CI is blocked by the mandatory interactive confirmation prompt. The `--tag` flag always creates a GPG-signed tag, leaving teams without GPG configured unable to tag at all. There is also a silent bug where the tool writes to `cargo.toml` (lowercase) on case-sensitive filesystems instead of `Cargo.toml`.

## Solution

Introduce a `vup init` subcommand that scaffolds a `vup.toml` config file in the repository root. The config file makes the tag prefix, commit message prefix, package file path, and push remote all overridable. Add a `--yes` flag to allow non-interactive use in CI. Add a `--no-sign` flag as a fallback for unsigned annotated tags. Fix the `Cargo.toml` casing bug.

## User Stories

1. As a developer, I want to run `vup init` so that a `vup.toml` config file is created in my repo with sensible defaults I can edit.
2. As a developer, I want `vup.toml` to document every available option with inline comments so I know what I can configure without reading source code.
3. As a developer, I want to set `tag_prefix = ""` in `vup.toml` so that tags are created as bare version numbers (e.g. `1.2.3`) instead of `v1.2.3`.
4. As a developer, I want to set `commit_prefix = "release: "` in `vup.toml` so that version bump commits follow my team's convention instead of `chore(version): `.
5. As a developer, I want to set `package = "packages/api/Cargo.toml"` in `vup.toml` so that `vup` targets a specific file instead of auto-detecting from CWD.
6. As a developer, I want to set `push_remote = "upstream"` in `vup.toml` so that pushes go to a named remote instead of the default tracking remote.
7. As a CI engineer, I want to run `vup --patch --yes` so that the version bump runs non-interactively without blocking on stdin.
8. As a developer without GPG configured, I want to pass `--no-sign` so that `vup` creates an annotated tag instead of failing after the commit is already made.
9. As a developer, I want `vup` to correctly update `Cargo.toml` (capital C) on case-sensitive filesystems so that it does not silently create a duplicate lowercase file.
10. As a developer, I want `vup init` to be idempotent so that running it twice does not overwrite a `vup.toml` I have already customised.
11. As a developer, I want CLI flags to take precedence over `vup.toml` values so that I can override config on a per-run basis.
12. As a developer, I want `vup` to work without a `vup.toml` present so that existing workflows are not broken by this change.
13. As a developer, I want `vup init` to print a summary of the file it created so I know what happened.
14. As a CI engineer, I want `vup` to exit with a non-zero code on failure so that my pipeline fails fast on errors.

## Implementation Decisions

- **`Config` struct and `Config::load()`**: A new module (`config.rs`) owns the `Config` struct. `Config::load()` reads `vup.toml` from CWD using the `toml` crate (already a dependency). If the file is absent it returns `Config::default()`. If the file is present but malformed it returns an error. Fields: `tag_prefix: String` (default `"v"`), `commit_prefix: String` (default `"chore(version): "`), `package: Option<String>` (default `None` — auto-detect), `push_remote: String` (default `"origin"`).

- **`vup init` subcommand**: `main.rs` checks for an `init` subcommand before the flag-matching block. It calls `Config::write_default()` which serialises `Config::default()` with inline TOML comments to `vup.toml`. If `vup.toml` already exists, it prints a warning and exits without overwriting.

- **`get_package(config: &Config)`**: The existing `get_package()` function gains a `&Config` parameter. If `config.package` is `Some(path)`, it opens that path directly. Otherwise it falls back to the existing auto-detection order (`Cargo.toml` → `package.json`). The `"cargo.toml"` casing bug is fixed to `"Cargo.toml"` in this same change.

- **`gitops::add_and_commit` signature extension**: The function gains three new parameters: `tag_prefix: &str`, `commit_prefix: &str`, `remote: &str`. Call sites in `main.rs` pass values from `Config`. The tag name becomes `format!("{}{}", tag_prefix, version)` and the commit message becomes `format!("{}{}", commit_prefix, version)`.

- **`--yes` flag**: Added as a clap arg in `main.rs`. When present, the confirmation prompt is skipped and the update proceeds as if the user typed `y`.

- **`--no-sign` flag**: Added as a clap arg. When present, `gitops::add_and_commit` calls `git tag -a` instead of `git tag -s`. The flag is ignored if `--tag` is not also present.

- **CLI flag precedence over config**: Config values are resolved at startup into a `Config` struct. CLI flags override the relevant config fields before use. For this iteration `--yes` and `--no-sign` are the only new flags; remote and prefixes are config-file-only.

## Testing Decisions

A good test verifies external behaviour — what `vup` produces in the filesystem and git history — not internal wiring. Tests must not assert on private struct fields or intermediate state.

- **`Config::load()`**: Unit-tested with a temp directory. Cases: missing file returns defaults; valid TOML returns parsed values; malformed TOML returns an error. Pattern mirrors the existing `Version` unit tests in `version.rs`.
- **`Config::write_default()` / `vup init`**: Tested by calling `write_default()` into a temp dir, then asserting the file exists and can be re-parsed by `Config::load()` without error.
- **`get_package(config)`**: Tested by creating a temp dir with a `Cargo.toml` or `package.json` and asserting the right package type is returned. The explicit `package` path override is also tested.
- **`gitops::add_and_commit`**: Tested against a temp git repo (using `git2::Repository::init`). Assert the commit message and tag name match the supplied prefix values.
- No integration tests against a live remote are required; push behaviour is exercised via the existing manual workflow.

## Out of Scope

- Pre-release version support (e.g. `1.2.3-alpha.1`) — the `Version` struct would need significant rework.
- Bumping multiple package files in a single run (monorepos with both `Cargo.toml` and `package.json`).
- A `--remote` CLI flag — remote is config-only for this iteration.
- Changelog generation.
- GitHub Actions / CI workflow files.

## Further Notes

- The `cargo.toml` casing bug is a one-character fix and should be treated as a standalone patch regardless of the rest of this spec.
- The `rusty-hook` pre-commit setup means contributors get formatting and lint checks automatically after `cargo build`.
- All new config keys must be optional with documented defaults so that `vup.toml` is fully backwards-compatible with the no-config workflow.
