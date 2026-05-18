<h1 align="center">
  bpond
  <br>
  <sub>Koi, alive in your terminal</sub>
</h1>

<p align="center">
  <a href="https://crates.io/crates/bpond"><img src="https://img.shields.io/crates/v/bpond.svg?logo=rust" alt="crates.io"></a>
  <a href="https://docs.rs/bpond"><img src="https://img.shields.io/docsrs/bpond" alt="docs.rs"></a>
  <a href="https://github.com/miso-devel/bpond/blob/main/LICENSE"><img src="https://img.shields.io/crates/l/bpond.svg" alt="license"></a>
</p>

<p align="center">
  <img src="./assets/demo.gif" alt="demo" width="600">
</p>

---

A living pond in your terminal: koi swim with chain-dynamics physics, lily pads drift on the surface, and frogs sit on the lily pads — breathing, croaking, and leaping. Click to drop food, right-click to scare. No keyframes, no pre-baked frames. Everything is procedural.

## Install

```bash
cargo install bpond
```

Requires Rust 1.80 or later. The installed binary lands in `~/.cargo/bin/`, so make sure that directory is on your `PATH`.

## Run

```bash
bpond                # standard mode
bpond --debug        # show a header with speed / runtime info
```

### From source

```bash
git clone https://github.com/miso-devel/bpond
cd bpond
cargo run --release
```

## Controls

| Input | Action |
|:---:|--------|
| Left click | Drop food |
| Right click | Scare nearby koi and frogs |
| `f` | Drop food at a random spot |
| `+` / `=` | Add a koi |
| `-` | Remove a koi |
| `r` | Toggle rain |
| `↑` / `↓` | Speed up / down |
| `q` / `Esc` | Quit |

## How It Works

**Spine**: 40 points chained at fixed distance. The head moves forward and the body follows; a head-to-tail traveling curvature wave gives each segment a subtle phase-delayed bend, producing the visible tail wagging during cruising and chasing.

**Rendering**: Each terminal cell = 2×4 braille sub-pixels (8× resolution). Body, fins, eyes, and tail are drawn as colored dots, with denser sampling across the body and a 3-band gradient so silhouettes anti-alias through canvas-level color averaging.

**Schooling**: A Boids-style rule (separation, alignment, cohesion) gently pulls idle koi into loose groups. When one koi heads for food, neighbors get curious and trail in — a "follow the leader" reaction familiar from real ponds.

**Feeding**: Koi detect food, steer with proportional navigation on a sharply tuned chase loop (high turn rate, lower forward speed), then decelerate, peck, and nibble with a small orbital wiggle.

**Fins**: Pectoral and pelvic fins each render as a 3-ray fan. While turning, the pectoral fin on the inside extends to brake; the outside fin tucks streamlined. Beat amplitude scales with the current thrust so sprinting and hovering look distinct.

**Frogs**: A handful of pond frogs sit on the surface, breathing visibly (throat pulse), occasionally croaking with an inflating vocal sac, sometimes flicking a tongue forward. Every few seconds a frog crouches and launches a parabolic-arc leap, landing with a splash — or, if a lily pad is in range, gracefully on top of it. Landing in open water triggers a brief breaststroke swim. Each frog rolls one of three colour morphs (green, olive, brown) at spawn, and right-click sends every frog within range scattering away from the threat.

**Lily pads**: Each pad is a clean disc with a single V-shaped wedge cut from the rim — a random size and depth per pad. Pads drift on the water surface under spring-to-home physics plus per-pad ambient currents, a shared global wind, and the wake of koi and jumping frogs passing nearby.

**Effects**: Ripple rings expand from food drops, raindrops, and frog splashes. Bubbles rise from the pond floor. Water color shifts through a day/night cycle.

## Architecture

```
src/
├── main.rs       Event loop + rendering
├── canvas.rs     Braille sub-pixel canvas
├── food.rs       Food pellet lifecycle
├── frog.rs       Pond frogs: sit / crouch / jump / land / croak / tongue / swim
├── koi/          Koi physics + drawing (chain dynamics, body wave, schooling)
├── lily.rs       Floating lily pads (V-wedge notches + drift physics)
├── pond.rs       Pond state + coordinate math
├── ripple.rs     Expanding ring effects
├── bubble.rs     Rising bubble particles
├── rain.rs       Rain system
└── rng.rs        Shared pseudo-RNG
```

## License

MIT
