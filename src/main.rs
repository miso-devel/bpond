use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Shark sprite ───────────────────────────────────────────────────────────
// Static shape. Animation is done by bending rows horizontally.

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
// Each ROW is shifted horizontally as a whole unit (no tearing).
// The shift amount comes from a sine wave that treats each row as
// a vertical slice of the fish body — top/bottom rows = tail edges,
// middle row = spine. The wave travels top→bottom creating a
// horizontal S-curve undulation.

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

    let omega = 2.0 * PI * 1.8 * speed;

    // Color wave: traveling brightness ripple
    let color_wave = |col: usize| -> f64 {
        let r = col as f64 / SHARK_W as f64;
        0.82 + 0.18 * (r * 5.0 - t * 3.0 * speed).sin()
    };

    // Breathing
    let breath = 0.94 + 0.06 * (t * 1.2 * speed).sin();

    for (row, line) in sprite.iter().enumerate() {
        // Row-based horizontal shift:
        // Map row index to a "body ratio" — row 0 and last row are tail extremes,
        // middle row is the head/nose (least movement).
        let row_ratio = row as f64 / (h - 1) as f64; // 0..1
        let from_center = (row_ratio - 0.5).abs() * 2.0; // 0 at center, 1 at edges

        // The undulation: columns shift LEFT/RIGHT based on how far the row is
        // from the body centerline. Tail rows (top/bottom) shift most.
        // Wave travels continuously for smooth motion.
        let shift = (from_center * 3.0 + t * omega).sin()
            * from_center  // amplitude grows toward tail
            * 2.5;         // max shift in columns

        let shift_int = shift.round() as i32;

        let py = base_y + row as i32;
        if py < 0 || py >= area.height as i32 { continue; }

        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let px = base_x + col as i32 + shift_int;
            if px < 0 || px >= area.width as i32 { continue; }

            let (br, bg, bb) = shark_color(ch);
            let intensity = color_wave(col) * breath;

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
