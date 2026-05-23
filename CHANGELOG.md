# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] — 2026-05-24

A "colour-out-of-the-box" release. The default pond now reads as
a vivid pond at a glance — saturated blue water, punchier red koi,
and a brighter green on the most common frog morph — and there's
a `--bg <hex>` flag for pinning the water to a fixed colour when
the day/night cycle's drift would get in the way (screenshots,
recordings, demo embeds).

### Added
- **`--bg <hex>` CLI flag** (`src/main.rs`): pin the water's base
  RGB to a fixed value, skipping the slow day/night colour drift.
  Accepts `RRGGBB` or `#RRGGBB`, with either `--bg <hex>` or
  `--bg=<hex>` argv form. Invalid hex silently falls back to the
  default cycle so a typo doesn't kill a TUI session. The per-cell
  ripple modulation still rides on top, so the surface keeps its
  shimmer.
- `parse_hex_rgb` + `parse_bg_arg` helpers, with parser tests for
  short input, non-hex chars, space- vs equals-separated argv, and
  the missing-flag case.

### Changed
- **Default water cycle** biased strongly toward blue
  (`(8..18, 26..44, 52..80)`) so the koi red and frog green land
  as vivid complementary / triadic contrast instead of merging
  into a muddy grey-blue. The new cycle still stays well above
  the koi's silhouette / outline range (≈ 8-32) on every channel
  so dark koi edges read as dark *against* the water rather than
  disappearing into it.
- **Koi `RED`** bumped to `(245, 35, 15)` for a punchier red.
- **Frog `Morph::Green`** light / mid bands lifted to
  `(95, 200, 70)` / `(55, 145, 45)` — vibrant lime rather than
  muddy olive. `Morph::Olive` picks up a modest saturation bump;
  `Morph::Brown` stays muted (it's the camo morph).

### Documentation
- README `Run` and `From source` sections document `--bg`, the
  `cargo run --release -- --bg <hex>` separator (so cargo
  doesn't eat the flag), and the recording workflow using
  `--bg 17243a` as a stable preset for the demo gif.
- CLAUDE.md key-bindings list gains the `--bg` entry.
- Demo gif refreshed against the saturated default palette.

## [0.6.0] — 2026-05-21

A "frogs in the pond" release. The pond gains a third inhabitant
alongside koi and lily pads: pond frogs with a full lifecycle of
sit, croak, tongue, jump, swim, scare — and they actually look
and behave differently depending on whether they're perched on a
pad or floating in the water.

### Added
- **Pond frogs** (`src/frog.rs` + `src/frog/draw.rs`): three frogs
  spawn on lily pads at startup. Each frog runs a state machine:
  * On a pad — `Sit` (folded Z hind legs, visible front legs,
    throat-pulse breath) with `Crouch → Jump → Land`, plus
    occasional `Croak` (vocal-sac inflation) and `TongueFlick`
    branches.
  * In water — `Float` (submerged tint, hind legs trailing back)
    cycling with `SwimKick` (one propulsive breaststroke kick).
    Frogs in water swim actively (~8 wu/s peak), and every kick
    starts with a random heading turn so the path visibly curves.
    Frogs in water never jump randomly — they only leap when a
    reachable lily pad is in range.
- **`on_pad: Option<usize>`** tracking on every frog. While set,
  the frog's position is synced to that pad's centre each tick,
  so a drifting pad carries the frog with it — perch detection
  cannot go stale. Pond uses this index directly to mark which
  pads to tint.
- **`FrogEvent`** API (`Splash`, `Wake`). Landing emits three
  concentric splash ripples; each swim kick emits two small wake
  ripples behind the frog; taking off from water emits a takeoff
  wake.
- **Food-drop scare**: `Pond::drop_food` propagates a scare to
  every frog within range. Koi still chase the pellet; frogs
  scatter away from the splash.
- **Perched-pad tint**: a lily pad with a frog on it lerps its
  green palette 70% toward a water-blue tint so the green frog
  reads clearly on top.
- Three colour morphs at spawn (`Green / Olive / Brown`),
  occasional eye blink (nictitating-membrane slit), right-click
  scare for frogs as well as koi.

### Changed
- **`Pond::drop_food` and `Pond::scare`** now take `(x, y, w, h)`
  so they can route to `Frog::scare`. This is the only
  backwards-incompatible signature change in 0.6.
- **`LilyPad`** gains `snapshot()` (pad position + radius for
  frog targeting) and `set_occupied(bool)` (set by Pond each
  frame from the perched-frog index).
- Pad radius range tuned to 5.0–8.0 wu so even the smallest pad
  comfortably contains the largest frog (max half-len ≈ 3.15).

### Fixed
- CI: release workflow now cross-compiles `x86_64-apple-darwin`
  on the `macos-latest` (Apple Silicon) runner. Every release
  since v0.3.1 had its Intel macOS binary job hang on the
  `macos-13` runner for the full 24-hour workflow limit and get
  cancelled. Added `timeout-minutes: 30` so a future runner hang
  fails fast.

### Documentation
- README updated to describe the on-pad vs in-water rest states
  and the new behaviours; architecture diagram now matches the
  actual `koi/` + `frog/` module layout with one-line file
  descriptions.
- CLAUDE.md updated with the frog state machine, pad-occupancy
  logic, and the `FrogEvent` flow.

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
