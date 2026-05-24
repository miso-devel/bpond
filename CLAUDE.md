# bpond

Procedural pond animation in the terminal — koi, frogs, and lily pads. Braille sub-pixel rendering + chain-dynamics spine.

## Build & Run

```bash
cargo run                       # debug build → run
cargo run --release             # release build → run
cargo run --release -- --debug  # show header (speed / runtime info)
cargo run --release -- --bg 17243a  # pin water to a fixed RGB (for recordings)
cargo watch -x run              # rebuild and rerun on file changes
RUST_BACKTRACE=1 cargo run      # run with a backtrace on panic
```

## Development

```bash
cargo check                # compile check
cargo fmt                  # format the code
cargo fmt --check          # format verification only (matches CI)
cargo clippy --all-targets -- -D warnings  # clippy with warnings as errors
cargo test                 # run tests
cargo clean                # remove build artifacts
```

Run the same checks CI runs, locally:
```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

## Architecture

```
src/
├── main.rs           # event loop + drawing (water / food / header)
├── canvas.rs         # braille sub-pixel canvas (1 cell = 2×4 dots)
├── food.rs           # food pellet lifecycle
├── frog.rs           # pond frogs: state machine + breath + croak + tongue + jump + swim
├── koi.rs            # koi: struct, constants, public API
├── koi/physics.rs    # steering, body wave (animate_body), Boids, sub-step update
├── koi/draw.rs       # body / tail / fins / eyes / burst-scaled rendering
├── lily.rs           # floating lily pads (V-wedge notches + drift physics)
├── pond.rs           # pond: koi+frog+lily+food state, coordinate helpers
├── ripple.rs         # expanding ripple rings
├── bubble.rs         # rising bubbles
├── rain.rs           # rain system
└── rng.rs            # shared pseudo-random number generator
```

### Technical notes

- **Chain dynamics**: a 40-segment world-coordinate chain. The head moves
  forward and each segment follows its predecessor; turns curve the body
  into natural C / S shapes.
- **Traveling wave (`animate_body`)**: a phase-delayed head-to-tail
  curvature applied per segment. This is what produces the visible
  tail wagging. Amplitude is modulated by `breath` and scaled by `burst`.
- **Sub-step integration**: each frame is split into `SUBSTEPS = 3`
  sub-frames so the dynamics stay smooth under sharp turns and bursts.
- **Boids schooling**: separation / alignment / cohesion forces summed
  against neighbors within `NEIGHBOR_RADIUS`. Only modulates idle
  steering — food chase and scare flight override. Weights are
  conservative so schooling shapes the path without dominating it.
- **Curiosity chain**: when a neighbor is heading toward food, this
  koi nudges its `target_turn` toward the same food. One koi leads,
  the rest trail in.
- **Braille rendering**: Unicode braille (U+2800) at 2×4 sub-pixels
  per cell — 8× the resolution of normal character rendering.
- **Uniform scale**: `sx == sy` keeps the koi from changing size when
  it changes heading.
- **Biomechanical fins**: angle-based open/close (`rest + amp × sin(ωt
  + phase)`), alternating left/right. While turning, the pectoral fin
  on the inside extends (asymmetric brake).
- **Burst-scaled drawing**: the draw layer reads `self.burst` and
  scales fin beat amplitude and tail spread accordingly.
- **Frog state machine** (`src/frog.rs`): two rest states
  depending on whether the frog is on a pad or in water.
  * `Sit` (on a pad) → `Crouch → Jump → Land → (Sit | Float)`
    with `Croak` and `TongueFlick` branches.
  * `Float` (in water) ↔ `SwimKick` (one propulsive breaststroke
    kick); Float occasionally rolls a `Crouch → Jump` to leave
    the water (preferring pads). The same `Crouch → Jump → Land`
    pipeline serves both rest states.
  Each `SwimKick` emits a `FrogEvent::Wake` ripple behind the
  frog; each `Jump` landing emits `FrogEvent::Splash`. Jump
  velocity also feeds the lily-pad wake force list.
- **Frog rendering**: a wide shoulder oval + tapered rear oval
  (broadest near the head). Two large yellow eyes poke sideways
  past the silhouette. Hind-leg posture is state-driven:
  * Sit/Crouch/Land: folded Z (femur out + tibia forward).
  * Jump: extended straight back.
  * Float: trailing back relaxed (mid-extended).
  * SwimKick: animated breaststroke sweep.
  When the frog is in water the whole body lerps toward a
  submerged blue tint; only the eyes stay bright above the
  surface, and front legs / throat / tongue overlays are hidden.
  Body size grows at the jump apex (`(27/4) t (1-t)^2` lift
  curve) to read as vertical lift. Each frog rolls one of three
  colour morphs at spawn: `Green / Olive / Brown`.
- **Pad occupancy** (`LilyPad::set_occupied`): each frame Pond
  marks any pad that has a resting frog inside its footprint; the
  pad's pixel shader lerps the green palette toward a water tint
  so the frog reads clearly on top.

### When making changes

- Branch off, work in the branch, and throw it away if it isn't approved.
- Fin parameters are in world coordinates (cell units). If you change
  scale, retune them.
- Changing `SEG_LEN` changes body length and, through `BODY_TOTAL`,
  body width and fin sizes too.
- `Koi::update` signature is `(dt, t, w, h, foods, others, my_idx)`.
  `others` is the neighbor snapshot list `(x, y, heading)`; `my_idx`
  is the index used to skip self.
- `Pond::update` collects a `Vec<(f64, f64, f64)>` of snapshots each
  frame and threads it through `Koi::update` (required to satisfy the
  borrow checker — can't read other koi while holding a mutable iter).

## Key Bindings

- Left click — drop food (koi swim over and eat it)
- Right click — scare nearby koi (they dart, then return) and frogs (which immediately leap away)
- `f` — drop food at a random position (no mouse needed)
- `+` / `=` — add one koi
- `-` — remove one koi
- `r` — toggle rain mode
- `↑` / `↓` — adjust simulation speed
- `q` / `Esc` — quit
- `--debug` flag — show header (speed / runtime info)
- `--bg <hex>` flag — pin the water's base RGB (skips the day/night cycle); ripple modulation still applies. Useful for screenshots and demo recordings.

## Releasing

Publishing to crates.io is automated via tag push.

1. Bump `version` in `Cargo.toml`.
2. Add a new version entry to `CHANGELOG.md`.
3. Commit and push.
4. `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. `.github/workflows/release.yml` runs `cargo publish` and creates a
   GitHub Release.

Prerequisite: `CARGO_REGISTRY_TOKEN` is set in repo secrets (issue one
from crates.io → Account Settings → API Tokens).
