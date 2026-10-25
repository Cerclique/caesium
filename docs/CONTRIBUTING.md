# Contributing

Thanks for taking the time to contribute to `caesium`. Bug reports, feature
requests and pull requests are all welcome. Everyone taking part is expected
to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting issues

Open an issue using one of the [templates][new-issue]. For a security
vulnerability, do **not** open a public issue; follow [SECURITY.md](SECURITY.md)
instead.

## Development

You need a stable Rust toolchain. The minimum supported Rust version (MSRV) is
the `rust-version` in [`Cargo.toml`](../Cargo.toml); don't use language or library
features newer than it.

### Dev container

The repository has a [dev container](../.devcontainer/devcontainer.json) with
everything below preinstalled: the stable toolchain, rust-analyzer, prek,
cargo-deny, cargo-semver-checks and git-cliff, with the Git hooks enabled.

- **GitHub Codespaces**: [open caesium in a codespace](https://codespaces.new/Cerclique/caesium),
  nothing to install.
- **VS Code**: with Docker and the Dev Containers extension, run
  *Dev Containers: Reopen in Container*.

The tool versions are pinned in
[`.devcontainer/post-create.sh`](../.devcontainer/post-create.sh).

Working on your own machine instead? Install the tools you need as described
below.

### Git hooks

The repository ships Git hooks in [`.pre-commit-config.yaml`](../.pre-commit-config.yaml),
run by [prek](https://github.com/j178/prek). Install prek (e.g.
`cargo install --locked prek`; see its README for other methods), then enable the
hooks once per clone:

```sh
prek install
```

- **pre-commit**: trailing whitespace, final newlines, line endings, YAML/TOML
  syntax, merge-conflict markers, `cargo fmt` and `cargo clippy`. Fixers rewrite
  the files and fail the commit; review the changes, `git add` them and commit
  again.
- **pre-push**: `cargo test`.

Run every hook on the whole tree with `prek run --all-files`. Skip them for one
commit with `git commit --no-verify`; CI runs the same checks anyway.

### Checks

Before opening a pull request, run the same checks as CI:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo run --locked --example main
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features
cargo deny check   # needs cargo-deny: cargo install --locked cargo-deny
```

Lints are configured in the `[lints]` table of `Cargo.toml` (`clippy::pedantic`
included). Every public item needs a doc comment.

The README is also the crate-level documentation, and its Rust examples run as
doctests, so keep them compiling.

### CI

The workflows live in [`.github/workflows/`](../.github/workflows/):

| Workflow | Runs on | What it does |
| --- | --- | --- |
| `rust.yml` | PRs to `main`, pushes to `main` | The checks above as separate jobs: `fmt`, `clippy`, `test` (Linux, macOS, Windows), `docs`, `msrv`, `deny`, `semver` |
| `pr-title.yml` | PRs | `conventional`: the PR title is a conventional commit |
| `devcontainer.yml` | PRs touching `.devcontainer/`, weekly, manual | Builds the dev container and runs the checks inside it |
| `scheduled.yml` | Weekly, manual | New advisories against the locked dependencies, tests and clippy on beta and nightly |
| `release.yml` | `v*` tag push | Creates the GitHub Release with git-cliff notes |

Every `rust.yml` job except `semver`, plus `conventional`, is required to
merge, and the branch must be up to date with `main`: if GitHub says it is
behind, update it and let the checks run again.

Each job is cancelled after 15 minutes of running (30 for the dev container
build), far above their usual time of under a minute. The limit doesn't cover
time spent queued: if jobs stay pending at 0s with no runner, force-cancel the
run and start a new one (push a commit, or update the branch) rather than
re-running it.

### Domain language and decisions

[`GLOSSARY.md`](GLOSSARY.md) defines the terms used in code and docs (global
logger, initialization, line, elapsed time, ...); please stick to them.
Significant design decisions are recorded as ADRs in [`docs/adr/`](adr/).

## Pull requests

- Branch from `main` and open a pull request against `main`. Direct pushes to
  `main` are blocked, and CI must pass before merging.
- PRs are **squash-merged**. The PR title becomes the commit title and the
  branch's commit messages become its body; the PR description stays on GitHub
  and is not copied into the history. The PR title must follow
  [Conventional Commits](https://www.conventionalcommits.org/), which CI checks:

  ```text
  <type>[(scope)][!]: <description>
  ```

  Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `build` (shown in the
  changelog), and `ci`, `test`, `chore`, `style` (left out of it). Mark a
  breaking change with `!` in the PR title (e.g. `feat!: ...`), or with a
  `BREAKING CHANGE:` footer in a branch commit (one in the PR description is
  not carried into the history).
- Don't edit `CHANGELOG.md` by hand: it is generated from the commit history by
  [git-cliff](https://git-cliff.org) when a release is cut.
- The `semver` CI job compares the public API with the latest release on
  crates.io. A red `semver` job is not a blocker; it signals that the next
  release must be a minor (breaking) bump while the version is `0.x`.

## Releases

Releases are cut by the maintainer, also through a pull request:

1. On a `release/vX.Y.Z` branch, bump `version` in `Cargo.toml` (and
   `Cargo.lock`).
2. Regenerate the changelog: `git cliff --tag vX.Y.Z -o CHANGELOG.md`.
3. Commit as `chore: release vX.Y.Z` and open a PR.
4. Once it is merged, tag the merge commit on `main` and push the tag:

   ```sh
   git tag -a vX.Y.Z -m "chore: Release caesium-log version X.Y.Z"
   git push origin vX.Y.Z
   ```

   Pushing the tag creates the GitHub Release, with notes generated by
   git-cliff. docs.rs builds the API docs once the crate is published.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](../LICENSE).

[new-issue]: https://github.com/Cerclique/caesium/issues/new/choose
