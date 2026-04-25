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

Koi fish swim with chain-dynamics physics. Click to drop food — they'll chase it. No keyframes, no pre-baked frames. Everything is procedural.

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
| Right click | Scare nearby koi |
| `f` | Drop food at a random spot |
| `+` / `=` | Add a koi |
| `-` | Remove a koi |
| `r` | Toggle rain |
| `↑` / `↓` | Speed up / down |
| `q` / `Esc` | Quit |

## How It Works

**Spine**: 40 points chained at fixed distance. Head moves forward, body follows — turns create natural C/S curves.

**Rendering**: Each terminal cell = 2×4 braille sub-pixels (8× resolution). Body, fins, and tail are drawn as colored dots.

**Feeding**: Koi detect food, steer with proportional navigation, decelerate on approach, then orbit and nibble.

**Effects**: Ripple rings expand from food drops and raindrops. Bubbles rise from the pond floor. Water color shifts through a day/night cycle.

## Architecture

```
src/
├── main.rs       Event loop + rendering
├── canvas.rs     Braille sub-pixel canvas
├── food.rs       Food pellet lifecycle
├── koi/          Koi physics + drawing
├── pond.rs       Pond state + coordinate math
├── ripple.rs     Expanding ring effects
├── bubble.rs     Rising bubble particles
├── rain.rs       Rain system
└── rng.rs        Shared pseudo-RNG
```

## License

MIT
