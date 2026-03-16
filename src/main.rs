use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Koi sprite: top-down view, facing right ────────────────────────────────
// 11 rows × 24 cols. Detailed with fins, tail, patterns.
//
// Character encoding:
//   # = body (white/base)
//   ~ = red/orange pattern
//   @ = eye
//   . ` ' = outline (dark)
//   { } = pectoral fins (spread out)
//   ( ) = tail fin lobes
//   - = tail fin rays
//   : = dorsal/anal fin edge
//   , = fin detail

const KOI_R: &[&str] = &[
    r"                  (--)   ",
    r"          .####. (----)  ",
    r"     .:##~~####~#(----)  ",
    r"   {.#~~#@##~~####. (-.) ",
    r"  {.####~~########~#.   >",
    r"  {`####~~########~#`   >",
    r"   {`#~~#.##~~####` (-`) ",
    r"     `:##~~####~#(----)  ",
    r"          `####` (----)  ",
    r"                  (--)   ",
];

const KOI_L: &[&str] = &[
    r"   (--)                  ",
    r"  (----)  .####.         ",
    r"  (----)#~####~~##:.     ",
    r" (.-) .####~~##@#~~#.}   ",
    r">   .#~########~~####.}  ",
    r">   `#~########~~####`}  ",
    r" (`-) `####~~##.#~~#`}   ",
    r"  (----)#~####~~##:`     ",
    r"  (----)  `####`         ",
    r"   (--)                  ",
];

const KOI_W: usize = 26;
const KOI_H: usize = 10;

// ─── Colors ─────────────────────────────────────────────────────────────────

fn koi_color(ch: char) -> Color {
    match ch {
        '#' => Color::Rgb(240, 238, 230),   // white body
        '~' => Color::Rgb(215, 55, 40),     // red/orange pattern
        '@' => Color::Rgb(15, 15, 20),      // eye
        '.' | '`' | '\'' => Color::Rgb(180, 170, 155), // body outline
        ':' => Color::Rgb(160, 150, 135),   // dorsal edge
        '{' | '}' => Color::Rgb(210, 200, 180), // pectoral fin base
        '(' | ')' => Color::Rgb(200, 190, 170), // tail lobe outline
        '-' => Color::Rgb(220, 210, 195),   // tail fin rays
        ',' => Color::Rgb(195, 185, 165),   // fin detail
        '>' | '<' => Color::Rgb(210, 200, 185), // tail center
        _ => Color::Rgb(230, 225, 215),
    }
}

// ─── Koi state ──────────────────────────────────────────────────────────────

struct Koi {
    x: f64,
    y: f64,
    heading: f64,
    speed: f64,
    turn_rate: f64,
    turn_timer: f64,
    id: f64,
}

fn update_koi(koi: &mut Koi, dt: f64, t: f64) {
    koi.turn_timer -= dt;
    if koi.turn_timer <= 0.0 {
        let seed = ((koi.id * 7.3 + t * 3.1).sin() * 10000.0).fract();
        koi.turn_rate = (seed - 0.5) * 1.2;
        let dur_seed = ((koi.id * 11.7 + t * 2.3).cos() * 10000.0).fract();
        koi.turn_timer = 2.0 + dur_seed * 4.0;
    }

    // Wall avoidance
    let margin = 0.2;
    let wall_str = 2.5;
    for &(edge, target) in &[
        (koi.x, 0.0f64), (1.0 - koi.x, PI),
        (koi.y, PI * 0.5), (1.0 - koi.y, -PI * 0.5),
    ] {
        if edge < margin {
            let push = ((margin - edge) / margin).powi(2);
            koi.turn_rate += (target - koi.heading).sin() * push * wall_str * dt;
        }
    }

    koi.turn_rate = koi.turn_rate.clamp(-1.2, 1.2);
    koi.heading += koi.turn_rate * dt;

    let spd = koi.speed * (0.85 + 0.15 * (t * 0.4 + koi.id).sin());
    koi.x += koi.heading.cos() * spd * dt * 0.015;
    koi.y += koi.heading.sin() * spd * dt * 0.03;
    koi.x = koi.x.clamp(0.05, 0.95);
    koi.y = koi.y.clamp(0.05, 0.95);
}

fn draw_koi(
    buf: &mut ratatui::buffer::Buffer,
    koi: &Koi,
    pond_x: i32, pond_y: i32, pond_w: i32, pond_h: i32,
    t: f64,
) {
    let cx = pond_x + (koi.x * pond_w as f64) as i32;
    let cy = pond_y + (koi.y * pond_h as f64) as i32;
    let sx = cx - KOI_W as i32 / 2;
    let sy = cy - KOI_H as i32 / 2;

    let facing_right = koi.heading.cos() >= 0.0;
    let sprite = if facing_right { KOI_R } else { KOI_L };

    // Subtle color breathing
    let breath = 0.95 + 0.05 * (t * 1.0 + koi.id).sin();

    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let px = sx + col as i32;
            let py = sy + row as i32;

            if px < pond_x || py < pond_y || px >= pond_x + pond_w || py >= pond_y + pond_h {
                continue;
            }

            let base = koi_color(ch);
            let fg = if let Color::Rgb(r, g, b) = base {
                Color::Rgb(
                    (r as f64 * breath).min(255.0) as u8,
                    (g as f64 * breath).min(255.0) as u8,
                    (b as f64 * breath).min(255.0) as u8,
                )
            } else {
                base
            };

            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_style(Style::default());
        }
    }
}

// ─── Pond ───────────────────────────────────────────────────────────────────

fn draw_pond(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
) -> (i32, i32, i32, i32) {
    let px = 2i32;
    let py = 2i32;
    let pw = (area.width as i32 - 4).max(20);
    let ph = (area.height as i32 - 4).max(10);

    for y in py..(py + ph) {
        for x in px..(px + pw) {
            let xf = x as f64;
            let yf = y as f64;
            let r1 = ((xf * 0.15 + yf * 0.25 + t * 0.6).sin()
                * (xf * 0.08 - t * 0.4).cos()) * 0.5 + 0.5;

            let r = (8.0 + r1 * 6.0) as u8;
            let g = (14.0 + r1 * 10.0) as u8;
            let b = (25.0 + r1 * 14.0) as u8;

            let ch = if r1 > 0.78 { '~' }
                else if r1 > 0.55 { '·' }
                else { ' ' };

            let cell = &mut buf[(x as u16, y as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(r + 10, g + 14, b + 18));
            cell.set_bg(Color::Rgb(r, g, b));
            cell.set_style(Style::default());
        }
    }

    // Border
    let bc = Color::Rgb(50, 42, 28);
    for x in (px - 1)..=(px + pw) {
        set(buf, x, py - 1, '─', bc, area);
        set(buf, x, py + ph, '─', bc, area);
    }
    for y in py..(py + ph) {
        set(buf, px - 1, y, '│', bc, area);
        set(buf, px + pw, y, '│', bc, area);
    }
    set(buf, px - 1, py - 1, '╭', bc, area);
    set(buf, px + pw, py - 1, '╮', bc, area);
    set(buf, px - 1, py + ph, '╰', bc, area);
    set(buf, px + pw, py + ph, '╯', bc, area);

    (px, py, pw, ph)
}

fn set(buf: &mut ratatui::buffer::Buffer, x: i32, y: i32, ch: char, fg: Color, area: ratatui::layout::Rect) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_style(Style::default());
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut koi = Koi {
        x: 0.5, y: 0.5, heading: 0.3,
        speed: 2.0, turn_rate: 0.0, turn_timer: 2.0, id: 1.0,
    };
    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        update_koi(&mut koi, dt, elapsed);

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(5, 8, 12));
                    cell.set_fg(Color::Rgb(5, 8, 12));
                }
            }

            let (px, py, pw, ph) = draw_pond(buf, area, elapsed);
            draw_koi(buf, &koi, px, py, pw, ph, elapsed);

            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
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
    }

    ratatui::restore();
    Ok(())
}
