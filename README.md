# vup — Package Version Bumper

A CLI tool to bump semver versions in `Cargo.toml` or `package.json`, commit the change, and optionally create a signed tag and push to remote.

## Installation

```shell
cargo install vup-rs
```

## How it works

1. Detects whether the current directory contains a `Cargo.toml` or `package.json` (see [Package detection](#package-detection))
2. Increments the requested version component (major, minor, or patch)
3. Prompts for confirmation before making any changes
4. Writes the updated version to the package file
5. Commits the change with message `chore(version): X.Y.Z`
6. Optionally creates a GPG-signed git tag (`-t`)
7. Optionally pushes the commit and tag to remote (`-u`)

## Usage

```shell
vup [OPTIONS]
```

`vup` must be run from the directory that contains your `Cargo.toml` or `package.json`. It does not search parent directories.

### Options

| Flag | Long | Description |
|------|------|-------------|
| `-m` | `--major` | Increment the major version (resets minor and patch to 0) |
| `-n` | `--minor` | Increment the minor version (resets patch to 0) |
| `-p` | `--patch` | Increment the patch version |
| `-t` | `--tag` | Create a GPG-signed git tag named `v{version}` after committing |
| `-u` | `--push` | Push the commit and tag (if created) to the remote |
| `-v` | `--verbose` | Print debug information |
| `-y` | `--yes` | Skip the confirmation prompt (useful in CI pipelines) |
|     | `--no-sign` | Create an annotated tag instead of a GPG-signed tag (use with `--tag`) |

## Examples

**Bump patch version:**

```shell
vup --patch
```

**Bump minor version and create a signed tag:**

```shell
vup --minor --tag
```

**Bump major version, create a signed tag, and push everything:**

```shell
vup --major --tag --push
```

**Bump patch version non-interactively (CI):**

```shell
vup --patch --yes
```

## Package detection

`vup` looks for package files in the current directory in this order:

1. `Cargo.toml` — if found, updates the `[package].version` field
2. `package.json` — if found, updates the `version` field

If both files exist, `Cargo.toml` takes priority.

## Configuration

`vup` is configured via a `vup.toml` file in the current directory. Use `vup init` to create one.

### Interactive setup

Running `vup init` walks you through each configuration field, shows the default in brackets, and writes `vup.toml` with your chosen values and inline comments:

```shell
vup init
```

```
Tag prefix (empty string = bare tags like "1.2.3" instead of "v1.2.3") [v]:
Commit message prefix [chore(version): ]:
Package file (auto-detects Cargo.toml — press Enter to keep, or type a path):
Git remote name (not a URL, required): origin
Created vup.toml
  tag_prefix     = "v"
  commit_prefix  = "chore(version): "
  package        = (auto-detect)
  push_remote    = "origin"
```

Press Enter to accept the default for any field. For `push_remote` you must type a value (the remote name, not a URL).

### Non-interactive setup (CI)

Use `--yes` to write the default `vup.toml` without any prompts:

```shell
vup init --yes
```

`vup init` always skips and warns if `vup.toml` already exists, so it is safe to call in a bootstrap script.

### Configuration fields

| Field | Default | Description |
|---|---|---|
| `tag_prefix` | `"v"` | Prepended to the version when creating git tags. Set to `""` for bare tags like `1.2.3`. |
| `commit_prefix` | `"chore(version): "` | Prefix for the version-bump commit message. |
| `package` | _(auto-detect)_ | Explicit path to `Cargo.toml` or `package.json`. Omit to auto-detect. |
| `push_remote` | `"origin"` | Git remote name to push commits and tags to. |

## Release notes

`vup release-notes` generates release notes from git history and prints Markdown to stdout.

### Modes

| Subcommand | Description |
|---|---|
| `changelog` | Lists all commits since the last git tag, grouped by conventional commit type |
| `structured` | Extracts item references (e.g. `#US-17`) matching patterns defined in `vup.toml` |

Both modes accept an optional `--from <ref>` flag to override the default commit range start.

### Examples

**Generate a changelog since the last tag:**

```shell
vup release-notes changelog
```

**Generate a changelog from a specific tag or SHA:**

```shell
vup release-notes changelog --from v1.2.0
```

**Generate structured release notes (item refs only):**

```shell
vup release-notes structured
```

**Pipe structured notes to a file for a CI artifact:**

```shell
vup release-notes structured > release-notes.md
```

**Generate structured notes from a specific ref:**

```shell
vup release-notes structured --from v1.2.0
```

### Configuring structured mode

Add a `[release_notes]` table to `vup.toml` to define pattern-to-section mappings. Each entry is a regex that is tested against each commit's subject and body; all matches are extracted as item references.

```toml
[[release_notes.patterns]]
regex = "#US-\\d+"
section = "User Stories"

[[release_notes.patterns]]
regex = "#BUG-\\d+"
section = "Bug Fixes"
```

A commit matching multiple patterns appears under each matching section. Unmatched commits are silently omitted. Running `structured` with no patterns configured produces only the header.

## Known limitations

- **Both `Cargo.toml` and `package.json` present:** `Cargo.toml` is always used. Bumping both files in a single run is not yet supported.
- **`--tag` requires GPG signing:** The tag is created with `git tag -s`, which requires a GPG key configured in your git environment. Use `--no-sign` alongside `--tag` to create an annotated tag instead, which works without GPG.

## Contributing

```shell
cargo build
```

The first build sets up pre-commit hooks via [`rusty-hook`](https://github.com/swellaby/rusty-hook). After that, the hooks run automatically on each commit.

Bug reports and pull requests are welcome on [GitHub](https://github.com/dan-alin/package-version-bumper).

## License

[MIT](./LICENSE)
