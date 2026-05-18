# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] — 2026-05-18

A "pond, not just koi" release. The pond now actually looks like one:
green lotus pads float on the surface, drift on the water, and react
to the koi swimming past.

### Added
- **Lotus pads** (`src/lily.rs`): each pad is a clean circular disc
  with a single V-shaped wedge cut from the rim. Per-pad randomness
  in radius, rotation, wedge size, wedge depth, and wedge angle, so
  no two pads look the same.
- **Pad drift physics**: spring-to-home + damping + per-pad ambient
  sinusoid + shared global wind + koi-wake nudges. Pads continuously
  jostle and rotate; a koi swimming past pushes the pads near it.
- **`Koi::velocity()`** public accessor, so the lily-pad layer can
  sample each koi's swim velocity for wake forces.
- **`Pond::lilies`** populated in `Pond::new`, ticked from
  `Pond::update`, and drawn last each frame so swimming koi are
  occluded where they pass under a pad.
- Extensive lily-pad test coverage, including a regression test that
  every spawned pad keeps at least 40% of its disc painted (the
  user-facing "still recognisable as a leaf" invariant) and a fix
  for a long-latent `angle_dist` bug that could let the wedge eat
  most of a pad when its rotation exceeded π.

### Changed
- Pad rendering uses a strict circle silhouette (rim never extends
  past its base radius) and a brighter green palette so leaves are
  clearly distinguishable from the dark blue water.

## [0.4.0] — 2026-05-06

A "lifelike koi" release. Visually the rendering core (braille,
sub-pixel canvas) is unchanged, but every koi now moves, schools, and
feeds in a noticeably more fish-like way.

### Added
- **Head-to-tail body wave** (`animate_body`): a traveling curvature
  wave gives each segment a phase-delayed bend. This is what produces
  the visible tail wagging during cruising and chasing. Modulated by a
  slow "breath" amplitude and the current burst.
- **Sub-step integration** (3 sub-frames per `update`) for smoother
  dynamics at high turn rates and sharp accelerations.
- **Glide-and-pause cruising**: idle koi alternate between gentle drift
  and near-still hovers, instead of cruising at a constant speed.
- **Eyes** (2×2 dark pupil + bright catchlight) on each side of the
  head.
- **Fan-shaped caudal fin** rendered as 3 rays per lobe, base solid /
  tips tapered. Spread scales with current swim effort.
- **Asymmetric pectoral fin steering**: while turning, the inside fin
  extends to brake; the outside fin tucks streamlined.
- **Pectoral / pelvic fins** gain ray detail and burst-scaled beat
  amplitude — sprinting koi row harder, hovering ones barely move.
- **Body color gradient**: 3-band shading (white/red → outline →
  silhouette) with denser sampling so edges anti-alias smoothly.
- **Boids-style schooling** (separation, alignment, cohesion) for
  loose group movement during idle.
- **Food curiosity chain**: a koi gets curious about food its neighbor
  is heading toward, producing "follow the leader" group reactions.
- Behavior test coverage for schooling, curiosity, glide-pause, and
  substep chase convergence.

### Changed
- Chase steering tightened — sharper turns (max 2.5 rad/s), faster
  target tracking, and lower forward speed during the turn so the koi
  takes a direct line to food instead of looping wide.
- Eating "peck" orbit reduced to a small wiggle (±0.15 rad).
- Head yaw oscillation halved (0.10 → 0.05) since the body wave now
  carries most of the side-to-side motion.

### Documentation
- README "How It Works" rewritten to cover body wave, schooling /
  curiosity, asymmetric pectoral brake, and burst-scaled fins.
- CLAUDE.md updated with the new architecture, invariants, and the
  full key bindings list.

### Removed
- `Canvas::thick` (3×3 sub-pixel block) — no longer used after the
  body switched to finer dot-based sampling.

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
