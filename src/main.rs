use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Hand-drawn shark sprite ────────────────────────────────────────────────
// Each character has a role: outline, fill, feature, or space
// The sprite is designed to look good as ASCII art FIRST, then animated.

const SHARK_SPRITE: &[&str] = &[
    r"                /\                       ",
    r"        _______/  \________              ",
    r"      /       '    \       \____         ",
    r"     / O    |  |  | \          \===      ",
    r"    <       |  |  |  \          >===     ",
    r"     \      |  |  |  /     ____/===      ",
    r"      \_____,___,___/ ____/              ",
    r"            \  /  \__/                   ",
    r"             \/                          ",
];

const SHARK_W: usize = 42;
const SHARK_H: usize = 9;

// Color for each character type
fn shark_color(ch: char, row: usize) -> Color {
    let vert = row as f64 / SHARK_H as f64;
    // Counter-shading: dark top, light bottom
    let r = (50.0 + 115.0 * vert) as u8;
    let g = (70.0 + 105.0 * vert) as u8;
    let b = (120.0 + 75.0 * vert) as u8;
    match ch {
        'O' => Color::Rgb(230, 235, 245),              // eye
        '|' => Color::Rgb(55, 70, 110),                 // gill slits
        '=' | '>' => Color::Rgb(r / 2 + 40, g / 2 + 45, b / 2 + 55), // tail
        '/' | '\\' | '_' | ',' | '\'' => Color::Rgb(r, g, b), // outline
        '<' => Color::Rgb(r, g, b),                      // nose
        _ => Color::Rgb(r, g, b),
    }
}

// ─── Eel sprite ─────────────────────────────────────────────────────────────

const EEL_SPRITE: &[&str] = &[
    r"  _____________________________________  ",
    r" / o  :  :  :  ~~~~~~~~~~~~~~~~~~~~~~~~\ ",
    r" \_________________________________~~~~/ ",
];

const EEL_W: usize = 42;
const EEL_H: usize = 3;

fn eel_color(ch: char, _row: usize) -> Color {
    match ch {
        'o' => Color::Rgb(180, 200, 160),
        ':' => Color::Rgb(35, 60, 40),
        '~' => Color::Rgb(40, 85, 50),
        _ => Color::Rgb(35, 75, 42),
    }
}

// ─── Creature ───────────────────────────────────────────────────────────────

struct Creature {
    name: &'static str,
    sprite: &'static [&'static str],
    width: usize,
    height: usize,
    color_fn: fn(char, usize) -> Color,
    wave_k: f64,     // spatial frequency
    wave_speed: f64,  // temporal frequency
    amp_head: f64,    // amplitude at head (cells)
    amp_tail: f64,    // amplitude at tail (cells)
}

const CREATURES: &[Creature] = &[
    Creature {
        name: "Shark",
        sprite: SHARK_SPRITE,
        width: SHARK_W,
        height: SHARK_H,
        color_fn: shark_color,
        wave_k: 5.0,
        wave_speed: 2.8,
        amp_head: 0.15,
        amp_tail: 1.5,
    },
    Creature {
        name: "Eel",
        sprite: EEL_SPRITE,
        width: EEL_W,
        height: EEL_H,
        color_fn: eel_color,
        wave_k: 8.0,
        wave_speed: 3.5,
        amp_head: 0.5,
        amp_tail: 3.0,
    },
];

// ─── Drawing ────────────────────────────────────────────────────────────────

fn draw_creature(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    c: &Creature,
    t: f64,
    speed: f64,
) {
    let sx = (area.width as i32 - c.width as i32) / 2;
    let sy = (area.height as i32 - c.height as i32) / 2;

    let omega = 2.0 * PI * c.wave_speed * speed;

    for (row, line) in c.sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' {
                continue;
            }

            // Wave: each column bends vertically, amplitude grows toward tail
            let r = col as f64 / c.width as f64; // 0=head, 1=tail
            let amp = c.amp_head + (c.amp_tail - c.amp_head) * r * r;
            let bend = (r * c.wave_k - omega * t).sin() * amp;

            let px = sx + col as i32;
            let py = sy + row as i32 + bend.round() as i32;

            if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                continue;
            }

            let fg = (c.color_fn)(ch, row);
            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_style(Style::default());
        }
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut current = 0usize;
    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Clear
            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(8, 10, 18));
                    cell.set_fg(Color::Rgb(8, 10, 18));
                }
            }

            let c = &CREATURES[current];
            draw_creature(buf, area, c, elapsed, speed);

            // Header
            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  {} ({}/{})  speed:{:.1}x  \u{2190}\u{2192}:switch  \u{2191}\u{2193}:speed  q:quit",
                    c.name, current + 1, CREATURES.len(), speed
                );
                for (i, ch) in hdr.chars().enumerate() {
                    if i >= area.width as usize { break; }
                    let cell = &mut buf[(i as u16, 0)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(50, 45, 75));
                }
            }
        })?;

        let timeout = TICK.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Char('n') => {
                            current = (current + 1) % CREATURES.len();
                        }
                        KeyCode::Left | KeyCode::Char('p') => {
                            current = if current == 0 { CREATURES.len() - 1 } else { current - 1 };
                        }
                        KeyCode::Up => speed = (speed + 0.2).min(5.0),
                        KeyCode::Down => speed = (speed - 0.2).max(0.2),
                        _ => {}
                    }
                }
            }
        }
        elapsed += last.elapsed().as_secs_f64();
        last = Instant::now();
    }

    ratatui::restore();
    Ok(())
}
