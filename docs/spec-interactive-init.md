# Spec: Interactive `vup init`

## Problem Statement

`vup init` currently writes a hard-coded default `vup.toml` and tells the user to open the file and edit it manually. Users must understand what each field does without guidance, remember the allowed values, and make a round-trip to their editor before `vup` is usable. Projects that differ from the defaults — a different tag prefix, a non-standard remote name, or an explicit package path — require manual intervention every time they bootstrap.

## Solution

Make `vup init` interactive. Instead of writing defaults silently, the command walks the user through each configuration field, displays the default in brackets, and accepts the user's input (or the default on a blank Enter). After all fields are collected, it writes `vup.toml` with the chosen values and inline comments, then prints a summary so the user can verify the result without opening the file.

A `--yes` flag on `vup init` preserves the current non-interactive behaviour — write defaults and exit — for scripts and CI bootstrapping.

## User Stories

1. As a developer, I want `vup init` to prompt me for each config field so that I do not have to manually edit the generated file.
2. As a developer, I want each prompt to display the default value in brackets so that I can accept it by pressing Enter without memorising the allowed values.
3. As a developer, I want to override a default by typing a custom value so that my project-specific settings are written correctly the first time.
4. As a developer, I want blank input on optional fields to accept the default so that I can step through unfamiliar fields quickly.
5. As a developer, I want the `package` prompt to tell me which file was auto-detected in the current directory so that I know whether to leave it blank or type an explicit path.
6. As a developer, I want invalid input (e.g. an empty push remote name) to be rejected with an explanatory message and the prompt to repeat so that I can correct mistakes without restarting the command.
7. As a developer, I want `vup init` to print a summary of the written values after completion so that I can verify the configuration at a glance.
8. As a developer, I want the generated `vup.toml` to still include inline comments so that future editors understand what each field controls.
9. As a developer, I want `vup init` to skip and warn if `vup.toml` already exists so that I cannot accidentally overwrite an existing configuration.
10. As a CI engineer, I want to run `vup init --yes` so that the default config is written non-interactively and my bootstrap script does not hang waiting for input.
11. As a developer, I want `vup init --yes` to behave identically to the current `vup init` so that existing automation is not broken.
12. As a developer, I want the `tag_prefix` prompt to explain that an empty string produces bare version tags (e.g. `1.2.3` instead of `v1.2.3`) so that I can make an informed choice.
13. As a developer, I want the `push_remote` prompt to explain that this is the git remote name (not a URL) so that I do not confuse it with the remote URL.

## Implementation Decisions

- **`Config::from_prompts(reader: &mut impl BufRead) -> Result<Config>`**: New associated function on `Config`. Iterates through each field in order (`tag_prefix`, `commit_prefix`, `package`, `push_remote`), printing a prompt line to stdout that includes the field's purpose and default (e.g. `Tag prefix [v]: `). Reads a line from `reader`, trims whitespace, and uses the typed value if non-empty or the field default if blank. `push_remote` is the only field with a non-empty constraint — an empty entry prints an error line and re-prompts. The `package` field is optional; blank input produces `None` (auto-detect). Returns a fully populated `Config`.

- **Package auto-detect hint**: Before printing the `package` prompt, `from_prompts` checks whether `Cargo.toml` or `package.json` exists in the current working directory and annotates the prompt accordingly (e.g. `Package file (auto-detects Cargo.toml — press Enter to keep, or type a path): `). The default remains `None` regardless; the user types an explicit path only if they need to override.

- **`Config::write(path: &Path) -> Result<()>`**: New method on `Config` that serialises a `Config` instance to a commented TOML string — same comment structure as the current `write_default` output — and writes it. The comment text is fixed; only the field values are interpolated from `self`. The existing `write_default` call in `main.rs` is replaced by `Config::default().write(path)` (keeping the non-interactive path working); the interactive path calls `config.write(path)` with the user-collected `Config`.

- **`--yes` on `vup init`**: The `init` subcommand gains a `--yes` flag. When set, `Config::default().write(path)` is called directly (current behaviour, no prompts). When absent, `Config::from_prompts(&mut stdin().lock())?` collects the config, then `.write(path)` writes it.

- **Summary output**: After a successful write, `vup init` prints the path of the created file followed by each field and its written value, e.g.:
  ```
  Created vup.toml
    tag_prefix     = "release-"
    commit_prefix  = "chore(version): "
    package        = (auto-detect)
    push_remote    = "upstream"
  ```

- **`write_default` retirement**: `Config::write_default` is replaced by `Config::write`. Callers are updated. The method is removed to avoid two code paths for writing config files.

## Testing Decisions

A good test for this feature asserts on observable outputs — the `Config` returned by `from_prompts` and the TOML string written by `write` — not on which internal branch was taken.

- **`Config::from_prompts`** is the primary test seam. Tests pass a `std::io::Cursor<&[u8]>` with newline-separated answers. This is the same `BufRead`-injection approach implied by the existing stdin reading in `main.rs`, just made explicit in the function signature.
  - All defaults accepted (all-blank lines) returns `Config::default()`.
  - Each field overridden individually produces the overridden value.
  - Empty `push_remote` input followed by a valid entry: the returned config has the valid entry (the re-prompt path is exercised).
  - `package` left blank returns `None`; a typed path returns `Some(path)`.

- **`Config::write`** tests: write a non-default `Config`, then `Config::load_from` the result — the round-trip should return an equal `Config`. This extends the existing `write_default_creates_parseable_file` pattern.

- No integration tests (spawning the binary) are needed; the unit seams above cover all acceptance criteria.

## Out of Scope

- Prompting for `[release_notes]` patterns — too complex and structured for a first-run setup flow.
- Interactive editing of an existing `vup.toml` — the command continues to skip if the file is present.
- Git remote auto-detection to pre-fill the `push_remote` default — the field default remains `"origin"`.
- Coloured or styled terminal prompts.
- Wizard-style navigation (go back, skip ahead).

## Further Notes

- `from_prompts` should write prompts to stdout (not stderr) so that output is predictable in tests and redirectable in scripts.
- The `--yes` flag on `vup init` is scoped to the `init` subcommand, not the global `--yes` flag on the root command. Both can coexist without conflict.
- `deny_unknown_fields` on `Config` is already set; the round-trip test for `Config::write` will catch any serialisation drift automatically.
