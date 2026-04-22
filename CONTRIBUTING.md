# Contributing to bpond

Thanks for your interest in bpond! Bug reports, feature ideas, and pull requests are all welcome.

## Getting Started

```bash
git clone https://github.com/miso-devel/bpond
cd bpond
cargo run --release
```

Rust toolchain is pinned via `rust-toolchain.toml` — `rustup` will pick it up automatically.

## Branching

bpond follows **GitHub Flow**:

- `main` is always releasable.
- Work happens on short-lived feature branches, merged into `main` via pull request.
- There is no `develop` branch.

Branch naming (kebab-case):

- `feat/add-rain-system`
- `fix/timeline-rendering`
- `refactor/extract-spine`
- `docs/update-readme`
- `chore/bump-deps`

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add ripple effect on food drop
fix: prevent koi from leaving pond bounds
refactor: extract canvas drawing helpers
docs: clarify spine algorithm
chore: bump ratatui to 0.29
test: cover food lifecycle edges
```

Keep each commit to one logical change. Describe the **why**, not the what.

## Pull Requests

Before opening a PR:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

CI runs the same checks, so these must pass.

PR body should include:

- **Summary** — what changed
- **Why** — motivation / context
- **Test plan** — how you verified it

One PR per concern. Mixing a feature, a refactor, and a bug fix makes review hard.

## Code Style

- Follow idiomatic Rust — `rustfmt` + `clippy` are the source of truth.
- Prefer clarity over cleverness.
- Avoid premature abstraction. Three similar lines beat an early helper.
- Comments explain **why**, not what. Well-named identifiers cover the what.
- Keep public APIs minimal. Expose only what callers need.

See [`CLAUDE.md`](./CLAUDE.md) for architectural notes on the spine, braille canvas, and fin biomechanics.

## Reporting Bugs

Open an issue with:

- OS and terminal emulator (e.g. macOS 14 + iTerm2, Linux + Alacritty)
- `rustc --version`
- Minimal steps to reproduce
- Expected vs actual behavior
- Screenshot or terminal recording if possible

## Feature Requests

Open an issue describing the use case and the behavior you'd like. For larger changes, please discuss the approach in an issue before opening a PR so we can align early.

## Releases

Releases are cut by the maintainer. The flow:

1. Bump `version` in `Cargo.toml`
2. Add a new section to `CHANGELOG.md` (Keep a Changelog format)
3. Commit and push to `main`
4. `git tag vX.Y.Z && git push origin vX.Y.Z`
5. `.github/workflows/release.yml` publishes to crates.io and creates a GitHub Release

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** — breaking changes
- **MINOR** — new features, backwards compatible
- **PATCH** — bug fixes, backwards compatible

## License

By contributing, you agree that your contributions will be licensed under the MIT License (see [`LICENSE`](./LICENSE)).
