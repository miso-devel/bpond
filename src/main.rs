use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Shark sprite ───────────────────────────────────────────────────────────

const SHARK: &[&str] = &[
    "         .^^.                    ",
    "        .####.                   ",
    "   .....######...........        ",
    "  .####=====######=====##>.      ",
    "  .##@#::---########---##=>>.    ",
    "  .####=====######=====##>.      ",
    "   .....######...........        ",
    "        .####.                   ",
    "         .^^.                    ",
];

const SHARK_W: usize = 34;

fn shark_color(ch: char) -> (f64, f64, f64) {
    match ch {
        '#' => (75.0, 95.0, 145.0),
        '=' => (95.0, 115.0, 160.0),
        '-' => (170.0, 180.0, 200.0),
        '@' => (240.0, 245.0, 255.0),
        ':' => (50.0, 65.0, 105.0),
        '^' => (60.0, 80.0, 130.0),
        '>' => (70.0, 88.0, 138.0),
        '.' | '`' => (40.0, 55.0, 95.0),
        _ => (75.0, 95.0, 145.0),
    }
}

// ─── Drawing ────────────────────────────────────────────────────────────────
// Centerline-based vertical undulation:
// Each column segment bends up/down based on a traveling sine wave.
// Segments of ~6 columns move together to prevent tearing.

fn draw_shark(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
    speed: f64,
) {
    let sprite = SHARK;
    let h = sprite.len();
    let base_x = (area.width as i32 - SHARK_W as i32) / 2;
    let base_y = (area.height as i32 - h as i32) / 2;

    let k = 2.0 * PI / 30.0; // wavelength in columns
    let omega = 2.0 * PI * 1.5 * speed;

    // Precompute bend per segment (groups of 6 columns)
    // This prevents tearing: all columns in a segment get the same bend
    let seg_size = 6;
    let num_segs = (SHARK_W + seg_size - 1) / seg_size;
    let mut seg_bend = vec![0i32; num_segs];
    for s in 0..num_segs {
        let col_mid = (s * seg_size + seg_size / 2) as f64;
        let r = col_mid / SHARK_W as f64;
        // Amplitude: head barely moves, tail sweeps wide
        let amp = 0.1 + 1.8 * r * r;
        let bend = (k * col_mid - omega * t).sin() * amp;
        seg_bend[s] = bend.round() as i32;
    }

    // Color wave for continuous visual motion
    let breath = 0.93 + 0.07 * (t * 1.3 * speed).sin();

    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let seg = col / seg_size;
            let bend = seg_bend[seg.min(num_segs - 1)];

            let px = base_x + col as i32;
            let py = base_y + row as i32 + bend;

            if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                continue;
            }

            // Color: traveling wave + breath
            let r = col as f64 / SHARK_W as f64;
            let wave = 0.85 + 0.15 * (r * 5.0 - t * 3.0 * speed).sin();
            let intensity = wave * breath;

            let (br, bg, bb) = shark_color(ch);
            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(
                (br * intensity).clamp(0.0, 255.0) as u8,
                (bg * intensity).clamp(0.0, 255.0) as u8,
                (bb * intensity).clamp(0.0, 255.0) as u8,
            ));
            cell.set_style(Style::default());
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

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

            draw_shark(buf, area, elapsed, speed);

            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Shark  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
                    speed
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
