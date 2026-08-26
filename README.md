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

## Package detection

`vup` looks for package files in the current directory in this order:

1. `Cargo.toml` — if found, updates the `[package].version` field
2. `package.json` — if found, updates the `version` field

If both files exist, `Cargo.toml` takes priority.

## Known limitations

- **Both `Cargo.toml` and `package.json` present:** `Cargo.toml` is always used. Bumping both files in a single run is not yet supported.
- **`--tag` requires GPG signing:** The tag is created with `git tag -s`, which requires a GPG key configured in your git environment. The command will fail if signing is not set up.

## Contributing

```shell
cargo build
```

The first build sets up pre-commit hooks via [`rusty-hook`](https://github.com/swellaby/rusty-hook). After that, the hooks run automatically on each commit.

Bug reports and pull requests are welcome on [GitHub](https://github.com/dan-alin/package-version-bumper).

## License

[MIT](./LICENSE)
