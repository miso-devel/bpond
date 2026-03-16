# terminal-zoo 🐟

Procedural koi fish swimming in your terminal — built with Rust, ratatui, and Unicode braille characters.

![Koi Pond](https://img.shields.io/badge/Rust-ratatui-blue)

## Features

- **Chain-dynamics spine**: Each koi has a 40-segment chain that naturally bends when turning — no rigid rotation
- **Braille sub-pixel rendering**: 8× resolution using Unicode braille characters (U+2800–U+28FF)
- **Biomechanics-accurate fins**: Angle-based pectoral/pelvic fin animation with left/right alternation
- **Procedural kohaku patterns**: Red/white patches generated per-fish from pseudo-random seeds
- **Animated water**: Subtle sine-based ripple pattern on the background
- **Responsive**: Koi scale adapts to terminal size in real-time
- **60fps**: Smooth animation at ~60 frames per second

## Quick Start

```bash
cargo run
```

Or with make:

```bash
make run          # Debug build + run
make watch        # Auto-rebuild on file change (requires cargo-watch)
make release      # Optimized release build
make run-release  # Run release binary
```

## Key Bindings

| Key | Action |
|-----|--------|
| `↑` | Speed up |
| `↓` | Slow down |
| `q` / `Esc` | Quit |

## Architecture

```
src/
├── main.rs      # Event loop, water rendering, header
├── canvas.rs    # Braille sub-pixel canvas (2×4 dots per cell)
└── koi.rs       # Koi fish: spine chain physics, body/fin/tail drawing
```

### How It Works

**Spine Chain**: The koi body is 40 world-space points. Each frame, the head moves forward along its heading, and each subsequent point follows the one ahead at a fixed distance. When the head turns, the body naturally curves into a C/S shape — like a real fish.

**Braille Rendering**: Each terminal cell is treated as a 2×4 grid of sub-pixels. The koi body, fins, and tail are drawn as colored dots into this grid, then encoded as Unicode braille characters with averaged foreground colors.

**Fin Animation**: Pectoral and pelvic fins use angle-based oscillation (`rest + amplitude × sin(ωt + phase)`). Left and right fins alternate in anti-phase for a natural paddling motion.

**Pattern Generation**: Red patches are placed pseudo-randomly along the spine based on each fish's unique ID, creating distinct kohaku-style markings per individual.

## Development

```bash
make check     # Compile check
make fmt       # Format code
make lint      # Clippy lint
make test      # Run tests
make ci        # fmt + lint + check + test
make clean     # Clean build artifacts
```

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) — Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — Terminal manipulation
- [color-eyre](https://github.com/eyre-rs/color-eyre) — Error reporting
