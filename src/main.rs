use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Pixel-art style shark ──────────────────────────────────────────────────
// Each character = 1 pixel. Colors encoded by character:
//   # = body (dark blue-gray)
//   = = body mid
//   - = belly (light)
//   @ = eye
//   : = gill
//   ^ = dorsal fin top
//   > = tail tips
//   . = outline/edge
//   (space) = transparent

const SHARK_FRAMES: &[&[&str]] = &[
    // Frame 0: tail center
    &[
        "         .^^.                    ",
        "        .####.                   ",
        "   .....######...........        ",
        "  .####=====######=====##>.      ",
        "  .##@#::---########---##=>>.    ",
        "  .####=====######=====##>.      ",
        "   ....`######...........        ",
        "        `####`                   ",
        "         `..`                    ",
    ],
    // Frame 1: tail up
    &[
        "         .^^.                    ",
        "        .####.                   ",
        "   ....`######`..........        ",
        "  .####=====######=====##.>.     ",
        "  .##@#::---########---###=>>.   ",
        "  .####=====######=====##`       ",
        "   ....`######`..........        ",
        "        `####`                   ",
        "         `..`                    ",
    ],
    // Frame 2: tail down
    &[
        "         .^^.                    ",
        "        .####.                   ",
        "   ....`######`..........        ",
        "  .####=====######=====##`       ",
        "  .##@#::---########---###=>>.   ",
        "  .####=====######=====##.>.     ",
        "   ....`######`..........        ",
        "        `####`                   ",
        "         `..`                    ",
    ],
];

const SHARK_W: usize = 34;
const SHARK_H: usize = 9;

fn shark_color(ch: char) -> Color {
    match ch {
        '#' => Color::Rgb(75, 95, 145),
        '=' => Color::Rgb(95, 115, 160),
        '-' => Color::Rgb(170, 180, 200),
        '@' => Color::Rgb(240, 245, 255),
        ':' => Color::Rgb(50, 65, 105),
        '^' => Color::Rgb(60, 80, 130),
        '>' => Color::Rgb(70, 88, 138),
        '.' => Color::Rgb(40, 55, 95),
        '`' => Color::Rgb(40, 55, 95),
        _ => Color::Rgb(75, 95, 145),
    }
}

// ─── Eel ────────────────────────────────────────────────────────────────────

const EEL_FRAMES: &[&[&str]] = &[
    &[
        " ..................................  ",
        ".##@##::==#==#==#==#==#==#==#==#=>. ",
        " ..................................  ",
    ],
    &[
        "  ..................................  ",
        " .##@##::==#==#==#==#==#==#==#==#=>. ",
        " ...................................  ",
    ],
];

const EEL_W: usize = 38;
const EEL_H: usize = 3;

fn eel_color(ch: char) -> Color {
    match ch {
        '#' => Color::Rgb(40, 80, 48),
        '=' => Color::Rgb(50, 95, 58),
        '@' => Color::Rgb(180, 200, 160),
        ':' => Color::Rgb(30, 55, 35),
        '>' => Color::Rgb(45, 85, 52),
        '.' => Color::Rgb(28, 50, 32),
        _ => Color::Rgb(40, 80, 48),
    }
}

// ─── Creature ───────────────────────────────────────────────────────────────

struct Creature {
    name: &'static str,
    frames: &'static [&'static [&'static str]],
    width: usize,
    height: usize,
    color_fn: fn(char) -> Color,
    // Undulation params
    wave_k: f64,
    wave_speed: f64,
    amp_head: f64,
    amp_tail: f64,
    // Frame animation speed (frames per second for sprite swap)
    frame_fps: f64,
}

const CREATURES: &[Creature] = &[
    Creature {
        name: "Shark",
        frames: SHARK_FRAMES,
        width: SHARK_W,
        height: SHARK_H,
        color_fn: shark_color,
        wave_k: 4.5,
        wave_speed: 2.5,
        amp_head: 0.1,
        amp_tail: 1.2,
        frame_fps: 4.0,
    },
    Creature {
        name: "Eel",
        frames: EEL_FRAMES,
        width: EEL_W,
        height: EEL_H,
        color_fn: eel_color,
        wave_k: 7.0,
        wave_speed: 3.5,
        amp_head: 0.4,
        amp_tail: 2.5,
        frame_fps: 3.0,
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

    // Select sprite frame based on time
    let frame_idx = ((t * c.frame_fps * speed) as usize) % c.frames.len();
    let sprite = c.frames[frame_idx];

    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            // Wave bend: each column shifts vertically
            let r = col as f64 / c.width as f64;
            let amp = c.amp_head + (c.amp_tail - c.amp_head) * r * r;
            let bend = (r * c.wave_k - omega * t).sin() * amp;

            let px = sx + col as i32;
            let py = sy + row as i32 + bend.round() as i32;

            if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                continue;
            }

            let fg = (c.color_fn)(ch);
            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_style(Style::default());
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

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

            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(8, 10, 18));
                    cell.set_fg(Color::Rgb(8, 10, 18));
                }
            }

            draw_creature(buf, area, &CREATURES[current], elapsed, speed);

            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  {} ({}/{})  speed:{:.1}x  \u{2190}\u{2192}:switch  \u{2191}\u{2193}:speed  q:quit",
                    CREATURES[current].name, current + 1, CREATURES.len(), speed
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
