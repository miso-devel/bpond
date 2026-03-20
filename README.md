<h1 align="center">
  mini-pond
  <br>
  <sub>Procedural koi pond in your terminal</sub>
</h1>

<p align="center">
  <img src="https://img.shields.io/badge/lang-Rust-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/TUI-ratatui-blue" alt="ratatui">
  <img src="https://img.shields.io/badge/rendering-braille-purple" alt="braille">
</p>

<p align="center">
  <img src="./assets/demo.gif" alt="demo" width="600">
</p>

---

4 koi fish swim with chain-dynamics physics. Click to drop food — they'll chase it. No keyframes, no pre-baked frames. Everything is procedural.

## Install & Run

```bash
cargo run --release
```

## Controls

| Input | Action |
|:---:|--------|
| Click | Drop food |
| `↑` / `↓` | Speed up / down |
| `q` | Quit |

## How It Works

**Spine**: 40 points chained at fixed distance. Head moves forward, body follows — turns create natural C/S curves.

**Rendering**: Each terminal cell = 2×4 braille sub-pixels (8× resolution). Body, fins, and tail are drawn as colored dots.

**Feeding**: Koi detect food, steer with proportional navigation, decelerate on approach, then orbit and nibble.

## Architecture

```
src/
├── main.rs      Event loop + rendering
├── canvas.rs    Braille sub-pixel canvas
├── food.rs      Food pellet lifecycle
├── koi.rs       Koi physics + drawing
└── pond.rs      Pond state + coordinate math
```

## License

MIT
