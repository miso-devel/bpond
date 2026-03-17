<h1 align="center">
  terminal-zoo
  <br>
  <sub>Procedural koi pond in your terminal</sub>
</h1>

<p align="center">
  <img src="https://img.shields.io/badge/lang-Rust-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/TUI-ratatui-blue" alt="ratatui">
  <img src="https://img.shields.io/badge/rendering-braille-purple" alt="braille">
  <img src="https://img.shields.io/badge/fps-60-green" alt="60fps">
</p>

<p align="center">
  Chain-dynamics spine physics / braille sub-pixel rendering / biomechanics fin animation
</p>

<p align="center">
  <img src="./assets/demo.gif" alt="demo" width="600">
</p>

---

## Features

| | Feature | Detail |
|---|---|---|
| 🦴 | **Chain-dynamics spine** | 40-segment chain — body bends into C/S shapes on turns, no rigid rotation |
| 🔬 | **Braille sub-pixel** | 8× resolution via Unicode braille (U+2800–U+28FF), 2×4 dots per cell |
| 🐠 | **Biomechanics fins** | Angle-based pectoral/pelvic oscillation, left/right anti-phase alternation |
| 🎨 | **Kohaku patterns** | Unique red/white markings per fish from pseudo-random seeds |
| 🌊 | **Animated water** | Sine-based ripple background |
| 📐 | **Responsive** | Uniform scaling adapts to terminal size in real-time |

## Quick Start

```bash
cargo run
```

<details>
<summary>More options (make)</summary>

```bash
make run          # Debug build + run
make watch        # Auto-rebuild on file change (requires cargo-watch)
make release      # Optimized release build
make run-release  # Run release binary
```

</details>

## Controls

| Key | Action |
|:---:|--------|
| `🖱 Click` | Drop food — koi swim toward it |
| `↑` | Speed up |
| `↓` | Slow down |
| `q` / `Esc` | Quit |

## How It Works

> 4 koi fish swim autonomously with procedural physics — no keyframe animation, no pre-baked frames.

### Spine Chain

Each koi is **40 world-space points** connected at fixed distance. The head moves forward, and each segment follows the one ahead. On turns, the body naturally curves — like a real fish.

```
Straight          Turning

  ●─●─●─●─●─●      ●─●─●
                          ╲
                           ●─●─●
```

### Braille Rendering

Each terminal cell = **2×4 sub-pixel grid** (8 dots). Body, fins, and tail are drawn as colored dots, then encoded as Unicode braille characters.

```
Terminal cell    Braille grid     Result

  ┌──┐           ⡀ ⠄              ⣿
  │  │           ⠂ ⠁
  │  │           ⠐ ⠈
  └──┘           ⢀ ⠠
```

### Fin Animation

Pectoral and pelvic fins oscillate with:

```
angle = rest + amplitude × sin(ωt + phase)
```

Left/right fins alternate in anti-phase for natural paddling motion.

### Pattern Generation

Red kohaku patches are placed pseudo-randomly along the spine based on each fish's unique ID — every fish looks different.

## Architecture

```
src/
├── main.rs      Event loop, water background, header
├── canvas.rs    Braille sub-pixel canvas (2×4 dots per cell)
└── koi.rs       Koi physics (chain spine) + rendering (body/fin/tail)
```

<details>
<summary>Development commands</summary>

```bash
make check     # Compile check
make fmt       # Format code
make lint      # Clippy lint
make test      # Run tests
make ci        # fmt + lint + check + test
make clean     # Clean build artifacts
```

</details>

## Dependencies

| Crate | Role |
|-------|------|
| [ratatui](https://github.com/ratatui/ratatui) | Terminal UI framework |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal manipulation |
| [color-eyre](https://github.com/eyre-rs/color-eyre) | Error reporting |

## License

MIT
