use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Koi fish (top-down view) ───────────────────────────────────────────────
// Small sprite, ~3 rows tall. Each koi has position, direction, speed, color.

// Koi sprites: facing right. 3 frames for tail beat.
// Top-down: oval body, tail fin, patterns
const KOI_R: &[&[&str]] = &[
    &[" .<>=>> ",   // frame 0: tail center
      " `<>=>> "],
    &[" .<>=>>/",   // frame 1: tail up
      " `<>=>  "],
    &[" .<>=>  ",   // frame 2: tail down
      " `<>=>>\\"],
];

// Facing left (mirrored)
const KOI_L: &[&[&str]] = &[
    &[" >>=<>. ",
      " >>=<>` "],
    &["\\>=><>. ",
      "  >=><>` "],
    &["  >=><>. ",
      "/>=><>` "],
];

struct Koi {
    x: f64,
    y: f64,
    vx: f64, // velocity x (positive = right)
    vy: f64,
    color: (u8, u8, u8),    // body color
    accent: (u8, u8, u8),   // pattern color
    tail_speed: f64,         // tail beat speed multiplier
    wave_freq: f64,          // lateral wave frequency
    phase: f64,              // starting phase offset
}

fn make_koi_pond() -> Vec<Koi> {
    vec![
        // Red/white koi (kohaku)
        Koi { x: 0.2, y: 0.3, vx: 0.8, vy: 0.15, color: (220, 60, 40), accent: (240, 230, 220),
              tail_speed: 1.0, wave_freq: 0.8, phase: 0.0 },
        // Black/orange (showa)
        Koi { x: 0.7, y: 0.6, vx: -0.6, vy: -0.1, color: (230, 140, 30), accent: (40, 35, 45),
              tail_speed: 1.2, wave_freq: 0.9, phase: 1.5 },
        // White/red spots (tancho)
        Koi { x: 0.5, y: 0.5, vx: 0.5, vy: -0.2, color: (235, 230, 220), accent: (200, 40, 30),
              tail_speed: 0.9, wave_freq: 0.7, phase: 3.0 },
        // Gold (ogon)
        Koi { x: 0.3, y: 0.7, vx: -0.7, vy: 0.25, color: (220, 180, 50), accent: (240, 210, 100),
              tail_speed: 1.1, wave_freq: 1.0, phase: 4.5 },
        // Blue/white (asagi)
        Koi { x: 0.8, y: 0.4, vx: 0.4, vy: 0.3, color: (70, 110, 170), accent: (200, 210, 230),
              tail_speed: 0.8, wave_freq: 0.75, phase: 2.0 },
    ]
}

fn update_koi(koi: &mut Koi, t: f64) {
    // Smooth swimming path: base velocity + gentle sine curves
    let dx = koi.vx + (t * koi.wave_freq + koi.phase).sin() * 0.3;
    let dy = koi.vy + (t * koi.wave_freq * 0.7 + koi.phase + 1.0).cos() * 0.25;

    koi.x += dx * 0.012;
    koi.y += dy * 0.012;

    // Wrap around pond edges with margin
    let margin = 0.1;
    if koi.x > 1.0 + margin { koi.x = -margin; }
    if koi.x < -margin { koi.x = 1.0 + margin; }
    if koi.y > 1.0 + margin { koi.y = -margin; }
    if koi.y < -margin { koi.y = 1.0 + margin; }

    // Slowly vary direction for natural wandering
    koi.vx += (t * 0.3 + koi.phase).sin() * 0.005;
    koi.vy += (t * 0.2 + koi.phase + 2.0).cos() * 0.005;

    // Clamp speed
    let spd = (koi.vx * koi.vx + koi.vy * koi.vy).sqrt();
    if spd > 1.2 {
        koi.vx *= 1.0 / spd;
        koi.vy *= 1.0 / spd;
    }
    if spd < 0.3 {
        koi.vx *= 1.5;
        koi.vy *= 1.5;
    }
}

fn draw_koi(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    koi: &Koi,
    t: f64,
) {
    // Screen position
    let pond_x = 2i32;
    let pond_y = 2i32;
    let pond_w = (area.width as i32 - 4).max(10);
    let pond_h = (area.height as i32 - 4).max(5);

    let sx = pond_x + (koi.x * pond_w as f64) as i32;
    let sy = pond_y + (koi.y * pond_h as f64) as i32;

    // Direction determines sprite set
    let facing_right = koi.vx >= 0.0;
    let sprites = if facing_right { KOI_R } else { KOI_L };

    // Frame selection based on tail beat
    let frame_idx = ((t * koi.tail_speed * 6.0) as usize) % sprites.len();
    let sprite = sprites[frame_idx];

    // Lateral undulation: slight vertical wave as the koi swims
    let wave = (t * koi.wave_freq * 2.0 * PI + koi.phase).sin() * 0.5;
    let wave_offset = wave.round() as i32;

    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let px = sx + col as i32;
            let py = sy + row as i32 + wave_offset;

            if px < pond_x || py < pond_y
                || px >= pond_x + pond_w
                || py >= pond_y + pond_h
            {
                continue;
            }

            let fg = match ch {
                '<' | '>' | '=' => Color::Rgb(koi.color.0, koi.color.1, koi.color.2),
                '.' | '`' | '/' | '\\' => Color::Rgb(koi.accent.0, koi.accent.1, koi.accent.2),
                _ => Color::Rgb(koi.color.0, koi.color.1, koi.color.2),
            };

            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_style(Style::default());
        }
    }
}

fn draw_pond(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64) {
    let pond_x = 2i32;
    let pond_y = 2i32;
    let pond_w = (area.width as i32 - 4).max(10);
    let pond_h = (area.height as i32 - 4).max(5);

    // Water surface: subtle ripples
    for y in pond_y..(pond_y + pond_h) {
        for x in pond_x..(pond_x + pond_w) {
            let ripple = ((x as f64 * 0.3 + y as f64 * 0.5 + t * 1.5).sin()
                * (x as f64 * 0.15 - t * 0.8).cos())
                * 0.5 + 0.5; // 0..1

            let r = (12.0 + ripple * 10.0) as u8;
            let g = (18.0 + ripple * 15.0) as u8;
            let b = (30.0 + ripple * 20.0) as u8;

            let ch = if ripple > 0.7 { '~' }
                else if ripple > 0.4 { '·' }
                else { ' ' };

            let cell = &mut buf[(x as u16, y as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(r + 15, g + 20, b + 25));
            cell.set_bg(Color::Rgb(r, g, b));
            cell.set_style(Style::default());
        }
    }

    // Pond border
    for x in (pond_x - 1)..=(pond_x + pond_w) {
        if x >= 0 && x < area.width as i32 {
            if pond_y > 0 {
                let cell = &mut buf[(x as u16, (pond_y - 1) as u16)];
                cell.set_char('─');
                cell.set_fg(Color::Rgb(60, 50, 35));
            }
            if pond_y + pond_h < area.height as i32 {
                let cell = &mut buf[(x as u16, (pond_y + pond_h) as u16)];
                cell.set_char('─');
                cell.set_fg(Color::Rgb(60, 50, 35));
            }
        }
    }
    for y in pond_y..(pond_y + pond_h) {
        if pond_x > 0 {
            let cell = &mut buf[((pond_x - 1) as u16, y as u16)];
            cell.set_char('│');
            cell.set_fg(Color::Rgb(60, 50, 35));
        }
        if pond_x + pond_w < area.width as i32 {
            let cell = &mut buf[((pond_x + pond_w) as u16, y as u16)];
            cell.set_char('│');
            cell.set_fg(Color::Rgb(60, 50, 35));
        }
    }
    // Corners
    let corners = [
        (pond_x - 1, pond_y - 1, '╭'),
        (pond_x + pond_w, pond_y - 1, '╮'),
        (pond_x - 1, pond_y + pond_h, '╰'),
        (pond_x + pond_w, pond_y + pond_h, '╯'),
    ];
    for (cx, cy, ch) in corners {
        if cx >= 0 && cy >= 0 && cx < area.width as i32 && cy < area.height as i32 {
            let cell = &mut buf[(cx as u16, cy as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(60, 50, 35));
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut koi = make_koi_pond();
    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64();
        elapsed += dt * speed;
        last = Instant::now();

        // Update koi positions
        for k in &mut koi {
            update_koi(k, elapsed);
        }

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Background
            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(5, 8, 12));
                    cell.set_fg(Color::Rgb(5, 8, 12));
                }
            }

            // Pond water + border
            draw_pond(buf, area, elapsed);

            // Koi fish
            for k in &koi {
                draw_koi(buf, area, k, elapsed);
            }

            // Header
            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi Pond ({} fish)  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
                    koi.len(), speed
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
    }

    ratatui::restore();
    Ok(())
}
