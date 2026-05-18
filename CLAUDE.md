# bpond

Procedural koi pond animation in the terminal. Braille sub-pixel rendering + chain-dynamics spine.

## Build & Run

```bash
cargo run                       # debug build → run
cargo run --release             # release build → run
cargo run --release -- --debug  # show header (speed / runtime info)
cargo watch -x run              # rebuild and rerun on file changes
RUST_BACKTRACE=1 cargo run      # run with a backtrace on panic
```

## Development

```bash
cargo check                # compile check
cargo fmt                  # format the code
cargo fmt --check          # format verification only (matches CI)
cargo clippy -- -D warnings  # clippy with warnings as errors
cargo test                 # run tests
cargo clean                # remove build artifacts
```

Run the same checks CI runs, locally:
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
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
- **Frog state machine** (`src/frog.rs`): `Sit → Crouch → Jump →
  Land → (Sit | Swim → Sit)`, with occasional `Croak` and
  `TongueFlick` branches off Sit. Crouch / Jump targets are biased
  toward nearby lily pads (`PAD_PREFERENCE = 0.65`); landing in
  open water diverts through `Swim` before the frog settles. Jump
  velocity also feeds the lily-pad wake force so leaping near a
  pad nudges it.
- **Frog rendering**: body oval + head bulge + two yellow eyes
  with pupils + folded-Z or extended hind legs depending on
  state. Body grows visibly at the jump apex to read as vertical
  lift. A vocal sac inflates under the chin during Croak; a pink
  tongue snaps forward during TongueFlick; an occasional blink
  replaces the eye with a dark slit (nictitating membrane). Each
  frog rolls one of three colour morphs at spawn: `Green / Olive
  / Brown`.

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
