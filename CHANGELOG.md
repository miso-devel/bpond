# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- MIT `LICENSE` file at the repo root.
- Package metadata in `Cargo.toml` (`license`, `repository`, `homepage`, `readme`, `keywords`, `categories`).
- `rust-toolchain.toml` pinning the toolchain to `stable` with `rustfmt` and `clippy`.

### Changed
- Migrated `src/koi/mod.rs` to `src/koi.rs` (2018 edition module layout).

### Removed
- `Makefile` — use `cargo` directly (`cargo run`, `cargo test`, `cargo clippy -- -D warnings`, etc.).

## [0.3.0] — 2026-04-15

### Added
- Rain mode (`r` key) with raindrop ripples on the water surface.
- Bubble particles rising from the pond floor.
- Right-click to scare nearby koi (they dart away).
- Add/remove koi with `+` / `-`.
- `--debug` flag shows a header with runtime info; hidden by default.

### Changed
- Renamed project from `mini-pond` to `bpond`.
- Replaced `color-eyre` with `anyhow` for simpler error handling.
- Extracted `Food` and `Pond` into their own modules; split `Koi` responsibilities.

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
