use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Koi sprites (top-down, 5 rows × ~14 cols) ─────────────────────────────
// Facing RIGHT (head on right side)
const KOI_R: &[&str] = &[
    r"  >.###.  ",
    r" >##~~#@. ",
    r">####~~##>",
    r" >##~~#`. ",
    r"  >`###`  ",
];

// Facing LEFT (head on left side)
const KOI_L: &[&str] = &[
    r"  .###.<  ",
    r" .@#~~##< ",
    r"<##~~####<",
    r" .`#~~##< ",
    r"  `###`<  ",
];

const KOI_W: usize = 10;
const KOI_H: usize = 5;

// ─── Koi state ──────────────────────────────────────────────────────────────

struct Koi {
    x: f64,          // 0..1 normalized pond position
    y: f64,
    heading: f64,    // radians: 0=right, PI/2=down, PI=left
    speed: f64,      // cells per second (base)
    turn_rate: f64,  // current turning speed (rad/sec)
    turn_timer: f64, // seconds until next turn decision
    body: (u8, u8, u8),
    pattern: (u8, u8, u8),
    accent: (u8, u8, u8),
    id: f64,         // unique seed for pseudo-random behavior
}

fn make_pond() -> Vec<Koi> {
    vec![
        Koi { x: 0.3, y: 0.4, heading: 0.3, speed: 3.0, turn_rate: 0.0, turn_timer: 2.0,
              body: (230, 230, 220), pattern: (210, 50, 40), accent: (240, 200, 60), id: 1.0 },
        Koi { x: 0.7, y: 0.6, heading: 2.5, speed: 2.5, turn_rate: 0.2, turn_timer: 1.5,
              body: (240, 140, 30), pattern: (250, 250, 240), accent: (220, 100, 20), id: 2.7 },
        Koi { x: 0.5, y: 0.3, heading: 4.0, speed: 2.0, turn_rate: -0.1, turn_timer: 3.0,
              body: (240, 240, 230), pattern: (200, 40, 35), accent: (180, 30, 25), id: 4.2 },
        Koi { x: 0.4, y: 0.7, heading: 1.2, speed: 2.8, turn_rate: 0.3, turn_timer: 1.0,
              body: (220, 175, 45), pattern: (245, 210, 90), accent: (200, 160, 30), id: 6.1 },
        Koi { x: 0.6, y: 0.5, heading: 5.5, speed: 2.2, turn_rate: -0.15, turn_timer: 2.5,
              body: (60, 100, 160), pattern: (190, 200, 220), accent: (40, 75, 130), id: 8.5 },
    ]
}

fn update_koi(koi: &mut Koi, dt: f64, t: f64) {
    // Turn decision timer
    koi.turn_timer -= dt;
    if koi.turn_timer <= 0.0 {
        // Pseudo-random new turn rate based on unique id + time
        let seed = ((koi.id * 7.3 + t * 3.1).sin() * 10000.0).fract();
        koi.turn_rate = (seed - 0.5) * 1.5; // -0.75 to 0.75 rad/sec

        // Pseudo-random duration
        let dur_seed = ((koi.id * 11.7 + t * 2.3).cos() * 10000.0).fract();
        koi.turn_timer = 1.5 + dur_seed * 3.0; // 1.5 to 4.5 seconds
    }

    // Wall avoidance: smoothly steer away from edges
    let margin = 0.15;
    let wall_force = 3.0;
    if koi.x < margin {
        let push = (margin - koi.x) / margin;
        koi.turn_rate += push * wall_force * dt * if koi.heading.sin() > 0.0 { -1.0 } else { 1.0 };
        // Steer toward center (heading → 0 = right)
        let target = 0.0;
        koi.turn_rate += (target - koi.heading).sin() * push * wall_force * dt;
    }
    if koi.x > 1.0 - margin {
        let push = (koi.x - (1.0 - margin)) / margin;
        let target = PI;
        koi.turn_rate += (target - koi.heading).sin() * push * wall_force * dt;
    }
    if koi.y < margin {
        let push = (margin - koi.y) / margin;
        let target = PI * 0.5;
        koi.turn_rate += (target - koi.heading).sin() * push * wall_force * dt;
    }
    if koi.y > 1.0 - margin {
        let push = (koi.y - (1.0 - margin)) / margin;
        let target = -PI * 0.5;
        koi.turn_rate += (target - koi.heading).sin() * push * wall_force * dt;
    }

    // Clamp turn rate
    koi.turn_rate = koi.turn_rate.clamp(-1.5, 1.5);

    // Update heading
    koi.heading += koi.turn_rate * dt;

    // Speed varies slightly with a sine wave (not constant)
    let spd = koi.speed * (0.85 + 0.15 * (t * 0.5 + koi.id).sin());

    // Move forward along heading
    // Terminal aspect ratio: chars are ~2x taller than wide
    koi.x += koi.heading.cos() * spd * dt * 0.02;
    koi.y += koi.heading.sin() * spd * dt * 0.04;

    // Hard clamp (shouldn't normally hit due to wall avoidance)
    koi.x = koi.x.clamp(0.02, 0.98);
    koi.y = koi.y.clamp(0.02, 0.98);
}

fn koi_color(ch: char, koi: &Koi) -> Color {
    match ch {
        '#' => Color::Rgb(koi.body.0, koi.body.1, koi.body.2),
        '~' => Color::Rgb(koi.pattern.0, koi.pattern.1, koi.pattern.2),
        '@' => Color::Rgb(20, 20, 25), // eye (dark)
        '.' | '`' | '\'' => Color::Rgb(koi.accent.0, koi.accent.1, koi.accent.2),
        '>' | '<' => Color::Rgb(
            koi.body.0.saturating_sub(20),
            koi.body.1.saturating_sub(20),
            koi.body.2.saturating_sub(10),
        ),
        _ => Color::Rgb(koi.body.0, koi.body.1, koi.body.2),
    }
}

fn draw_koi(
    buf: &mut ratatui::buffer::Buffer,
    _area: ratatui::layout::Rect,
    koi: &Koi,
    pond_x: i32, pond_y: i32, pond_w: i32, pond_h: i32,
) {
    let sx = pond_x + (koi.x * pond_w as f64) as i32 - KOI_W as i32 / 2;
    let sy = pond_y + (koi.y * pond_h as f64) as i32 - KOI_H as i32 / 2;

    // Pick sprite based on heading direction
    let facing_right = koi.heading.cos() >= 0.0;
    let sprite = if facing_right { KOI_R } else { KOI_L };

    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let px = sx + col as i32;
            let py = sy + row as i32;

            if px < pond_x || py < pond_y || px >= pond_x + pond_w || py >= pond_y + pond_h {
                continue;
            }

            let fg = koi_color(ch, koi);
            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_style(Style::default());
        }
    }
}

// ─── Pond rendering ─────────────────────────────────────────────────────────

fn draw_pond(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
) -> (i32, i32, i32, i32) {
    let pond_x = 2i32;
    let pond_y = 2i32;
    let pond_w = (area.width as i32 - 4).max(10);
    let pond_h = (area.height as i32 - 4).max(5);

    // Water surface
    for y in pond_y..(pond_y + pond_h) {
        for x in pond_x..(pond_x + pond_w) {
            let xf = x as f64;
            let yf = y as f64;
            let ripple = ((xf * 0.2 + yf * 0.3 + t * 0.8).sin()
                * (xf * 0.1 - t * 0.5).cos())
                * 0.5 + 0.5;

            let r = (10.0 + ripple * 8.0) as u8;
            let g = (16.0 + ripple * 12.0) as u8;
            let b = (28.0 + ripple * 16.0) as u8;

            let ch = if ripple > 0.75 { '~' }
                else if ripple > 0.5 { '·' }
                else { ' ' };

            let cell = &mut buf[(x as u16, y as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(r + 12, g + 16, b + 20));
            cell.set_bg(Color::Rgb(r, g, b));
            cell.set_style(Style::default());
        }
    }

    // Border
    for x in (pond_x - 1)..=(pond_x + pond_w) {
        if x >= 0 && x < area.width as i32 {
            if pond_y > 0 {
                set(buf, x, pond_y - 1, '─', Color::Rgb(55, 45, 30), area);
            }
            if pond_y + pond_h < area.height as i32 {
                set(buf, x, pond_y + pond_h, '─', Color::Rgb(55, 45, 30), area);
            }
        }
    }
    for y in pond_y..(pond_y + pond_h) {
        if pond_x > 0 {
            set(buf, pond_x - 1, y, '│', Color::Rgb(55, 45, 30), area);
        }
        if pond_x + pond_w < area.width as i32 {
            set(buf, pond_x + pond_w, y, '│', Color::Rgb(55, 45, 30), area);
        }
    }
    let c = Color::Rgb(55, 45, 30);
    set(buf, pond_x - 1, pond_y - 1, '╭', c, area);
    set(buf, pond_x + pond_w, pond_y - 1, '╮', c, area);
    set(buf, pond_x - 1, pond_y + pond_h, '╰', c, area);
    set(buf, pond_x + pond_w, pond_y + pond_h, '╯', c, area);

    (pond_x, pond_y, pond_w, pond_h)
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

    let mut koi = make_pond();
    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        for k in &mut koi {
            update_koi(k, dt, elapsed);
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

            let (px, py, pw, ph) = draw_pond(buf, area, elapsed);

            for k in &koi {
                draw_koi(buf, area, k, px, py, pw, ph);
            }

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
