# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] — 2026-04-26

### Added
- Pre-built binary archives on GitHub Releases for macOS (arm64 / x86_64),
  Linux (x86_64 / aarch64), and Windows (x86_64). Each archive bundles
  the binary alongside `README.md`, `LICENSE`, and `CHANGELOG.md`.

### Documentation
- README now leads with `cargo install bpond` and shows live
  crates.io / docs.rs / license badges.

## [0.3.0] — 2026-04-23

First release published to crates.io.

### Added
- Rain mode (`r` key) with raindrop ripples on the water surface.
- Bubble particles rising from the pond floor.
- Right-click to scare nearby koi (they dart away).
- Add/remove koi with `+` / `-`.
- `f` to drop food at a random position (no mouse required).
- `--debug` flag shows a header with runtime info; hidden by default.
- MIT `LICENSE` file at the repo root.
- Package metadata in `Cargo.toml` (`license`, `repository`, `homepage`, `readme`,
  `keywords`, `categories`, `authors`, `rust-version`, `exclude`).
- `rust-toolchain.toml` pinning the toolchain to `stable` with `rustfmt` and `clippy`.
- Release workflow (`.github/workflows/release.yml`) that publishes to crates.io
  and creates a GitHub Release on tag push.
- `CONTRIBUTING.md` with branching, commit, and release conventions.

### Changed
- Renamed project from `mini-pond` to `bpond`.
- Replaced `color-eyre` with `anyhow` for simpler error handling.
- Extracted `Food` and `Pond` into their own modules; split `Koi` responsibilities.
- Migrated `src/koi/mod.rs` to `src/koi.rs` (2018 edition module layout).
- Updated tagline to "Koi, alive in your terminal" in the README header and
  crate description.
- Slimmed the README: dropped the MP4 artifact link and the braille rendering badge.

### Removed
- `Makefile` — use `cargo` directly (`cargo run`, `cargo test`,
  `cargo clippy -- -D warnings`, etc.).
- `demo.tape` and tracked `.claude/launch.json` — personal dev artifacts
  that do not belong in the public repo.

## [0.2.0]

### Added
- Mouse click to drop food pellets; koi chase and eat them.

### Changed
- Split rendering into `canvas` and `koi` modules.
- Uniform scale so heading changes do not resize the koi.
- Angle-based fin animation with larger, visible spread.

## [0.1.0]

### Added
- Initial release as `terminal-zoo`: procedural koi with chain-dynamics spine
  and braille sub-pixel rendering.
