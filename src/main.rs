use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Shark body profile from empirical data ─────────────────────────────────
// Half-height as fraction of body length (great white / lamnid shark)
const PROFILE: &[(f64, f64)] = &[
    (0.00, 0.000),
    (0.03, 0.025),
    (0.06, 0.045),
    (0.10, 0.070),
    (0.15, 0.090),
    (0.20, 0.100),
    (0.25, 0.108),
    (0.30, 0.112), // max girth
    (0.35, 0.110),
    (0.40, 0.105),
    (0.45, 0.098),
    (0.50, 0.090),
    (0.55, 0.080),
    (0.60, 0.068),
    (0.65, 0.055),
    (0.70, 0.042),
    (0.75, 0.030),
    (0.80, 0.022), // peduncle starts
    (0.85, 0.015),
    (0.88, 0.012),
    (0.91, 0.025), // caudal fin starts
    (0.95, 0.035),
    (0.98, 0.020),
    (1.00, 0.000),
];

fn profile_half(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    for i in 1..PROFILE.len() {
        if x <= PROFILE[i].0 {
            let (x0, y0) = PROFILE[i - 1];
            let (x1, y1) = PROFILE[i];
            let t = (x - x0) / (x1 - x0);
            return y0 + (y1 - y0) * t;
        }
    }
    0.0
}

// Dorsal profile is taller (hump), ventral is flatter
fn dorsal_half(x: f64) -> f64 {
    let base = profile_half(x);
    if x < 0.50 {
        base * 1.3 // dorsal hump
    } else {
        base * 1.1
    }
}

fn ventral_half(x: f64) -> f64 {
    profile_half(x) * 0.85
}

// ─── Creature trait ─────────────────────────────────────────────────────────
trait Creature {
    fn name(&self) -> &str;
    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64, speed: f64);
}

// ─── Shark ──────────────────────────────────────────────────────────────────
struct SharkCfg;

impl Creature for SharkCfg {
    fn name(&self) -> &str { "Shark" }

    #[allow(clippy::too_many_lines)]
    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64, speed: f64) {
        let len: i32 = 52;
        let height_scale = 22.0; // maps profile fraction → rows
        let sx = (area.width as i32 - len) / 2;
        let cy = area.height as f64 / 2.0;

        // Wave params (thunniform: lambda ~1.14 body lengths)
        let lambda = len as f64 * 1.14;
        let k = 2.0 * PI / lambda;
        let freq = 1.3 * speed;
        let omega = 2.0 * PI * freq;

        // Amplitude envelope: thunniform (movement concentrated at tail)
        let amp_env = |x: f64| -> f64 {
            let a = if x < 0.6 { 0.02 * x } else { 0.02 * 0.6 + 0.15 * ((x - 0.6) / 0.4).powi(2) };
            a * len as f64
        };

        for col in 0..len {
            let r = col as f64 / len as f64;
            let px = sx + col;
            if px < 0 || px >= area.width as i32 { continue; }

            let col_f = col as f64;

            // Lateral displacement z(x,t)
            let z = amp_env(r) * (k * col_f - omega * t).sin();

            // Derivative dz/dx — determines apparent thickening
            let dz_dx = amp_env(r) * k * (k * col_f - omega * t).cos();
            // (ignoring dA/dx term for simplicity — it's small)

            // Apparent thickness: W = D / cos(theta), theta = atan(dz/dx)
            // Simplified: W ≈ D * (1 + 0.5 * (dz/dx)^2)
            let thickness_factor = 1.0 + 0.5 * dz_dx * dz_dx;

            // Asymmetric profile (dorsal hump)
            let ht = dorsal_half(r) * height_scale * thickness_factor;
            let hb = ventral_half(r) * height_scale * thickness_factor;

            // Tiny vertical shift from recoil (anti-phase to tail, very small)
            let recoil = -z * 0.03 * (1.0 - r);
            let center = cy + recoil;

            let top_exact = center - ht;
            let bot_exact = center + hb;
            let top = top_exact.round() as i32;
            let bot = bot_exact.round() as i32;

            // Edge slope (for directional chars)
            let r_next = ((col + 1) as f64 / len as f64).min(1.0);
            let z_next = amp_env(r_next) * (k * (col_f + 1.0) - omega * t).sin();
            let recoil_next = -z_next * 0.03 * (1.0 - r_next);
            let ht_next = dorsal_half(r_next) * height_scale * {
                let d = amp_env(r_next) * k * (k * (col_f + 1.0) - omega * t).cos();
                1.0 + 0.5 * d * d
            };
            let top_next = ((cy + recoil_next) - ht_next).round() as i32;
            let slope = (top_next - top) as f64;
            let (top_ch, bot_ch) = edge_chars(slope);

            // AA hints
            let top_frac = (top_exact - top_exact.floor()).abs();
            let bot_frac = (bot_exact - bot_exact.floor()).abs();
            if top_frac > 0.25 && top_frac < 0.75 {
                let hy = if top_exact < top as f64 { top - 1 } else { top + 1 };
                set_c(buf, px, hy, '·', Color::Rgb(22, 32, 55), area);
            }
            if bot_frac > 0.25 && bot_frac < 0.75 {
                let hy = if bot_exact > bot as f64 { bot + 1 } else { bot - 1 };
                set_c(buf, px, hy, '·', Color::Rgb(60, 68, 76), area);
            }

            // Brightness from viewing angle (thicker parts = more surface facing us)
            let bright = (0.7 + 0.3 * thickness_factor.min(1.5) / 1.5).min(1.0);

            for py in top..=bot {
                let vert = if top == bot { 0.5 }
                    else { (py - top) as f64 / (bot - top) as f64 };
                let depth = (0.5 - (vert - 0.5).abs()) * 2.0;

                // Counter-shading: dark dorsal → light belly
                let cr = 45.0 + 120.0 * vert;
                let cg = 65.0 + 110.0 * vert;
                let cb = 115.0 + 80.0 * vert;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb((cr * bright) as u8, (cg * bright) as u8, (cb * bright) as u8))
                } else if py == top {
                    (top_ch, Color::Rgb((45.0 * bright) as u8, (65.0 * bright) as u8, (115.0 * bright) as u8))
                } else if py == bot {
                    (bot_ch, Color::Rgb((150.0 * bright) as u8, (165.0 * bright) as u8, (185.0 * bright) as u8))
                } else if depth > 0.6 {
                    ('=', Color::Rgb((cr * bright) as u8, (cg * bright) as u8, (cb * bright) as u8))
                } else if depth > 0.25 {
                    ('·', Color::Rgb((cr * bright * 0.7) as u8, (cg * bright * 0.7) as u8, (cb * bright * 0.7) as u8))
                } else {
                    (' ', Color::Rgb(8, 10, 18))
                };
                if ch != ' ' { set_c(buf, px, py, ch, fg, area); }
            }

            // Dorsal fin (at profile peak region)
            if r > 0.22 && r < 0.42 {
                let fh = (1.0 - ((r - 0.32) / 0.10).powi(2)).max(0.0) * 3.0 * thickness_factor.min(1.3);
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '/' } else { '|' };
                    set_c(buf, px, top - h, ch, Color::Rgb((45.0 * bright) as u8, (65.0 * bright) as u8, (115.0 * bright) as u8), area);
                }
            }

            // Pectoral fin
            if r > 0.28 && r < 0.40 {
                let fh = (1.0 - ((r - 0.34) / 0.06).powi(2)).max(0.0) * 1.8 * thickness_factor.min(1.3);
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '\\' } else { '|' };
                    set_c(buf, px, bot + h, ch, Color::Rgb((150.0 * bright) as u8, (165.0 * bright) as u8, (185.0 * bright) as u8), area);
                }
            }

            // Tail fork (only in caudal fin region r > 0.91)
            if r > 0.91 && r < 0.99 {
                // Already handled by profile widening
            }
        }

        // Eye
        let eye_r = 0.10;
        let eye_col = (len as f64 * eye_r) as i32;
        let z_eye = amp_env(eye_r) * (k * eye_col as f64 - omega * t).sin();
        let recoil_eye = -z_eye * 0.03 * (1.0 - eye_r);
        let eye_y = (cy + recoil_eye - dorsal_half(eye_r) * height_scale * 0.3).round() as i32;
        set_c(buf, sx + eye_col, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        // Gills
        for i in 0..3 {
            let gr = 0.18 + i as f64 * 0.02;
            let gc = (len as f64 * gr) as i32;
            let z_g = amp_env(gr) * (k * gc as f64 - omega * t).sin();
            let recoil_g = -z_g * 0.03 * (1.0 - gr);
            let gy = (cy + recoil_g).round() as i32;
            set_c(buf, sx + gc, gy, ':', Color::Rgb(40, 55, 95), area);
        }
    }
}

// ─── Eel ────────────────────────────────────────────────────────────────────
struct EelCfg;

impl Creature for EelCfg {
    fn name(&self) -> &str { "Eel" }

    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64, speed: f64) {
        let len: i32 = 55;
        let max_half: f64 = 1.2;
        let sx = (area.width as i32 - len) / 2;
        let mid_y = area.height as f64 / 2.0;

        let k = 2.0 * PI / 28.0;
        let omega = 2.0 * PI * 1.8 * speed;
        let envelope = |x: f64| -> f64 { 0.3 + 0.7 * x };

        let cy_at = |col: i32| -> f64 {
            let r = col as f64 / len as f64;
            mid_y + 3.0 * envelope(r) * (k * col as f64 - omega * t).sin()
        };

        // Trail
        for echo in 1..=2 {
            let t_past = t - echo as f64 * 0.05;
            let fade = 0.2 / echo as f64;
            for col in 0..len {
                let r = col as f64 / len as f64;
                let cy_p = mid_y + 3.0 * envelope(r) * (k * col as f64 - omega * t_past).sin();
                let half = if r < 0.08 { r / 0.08 * max_half }
                    else if r > 0.90 { (1.0 - r) / 0.10 * max_half }
                    else { max_half };
                let top = (cy_p - half).round() as i32;
                let bot = (cy_p + half).round() as i32;
                let px = sx + col;
                for py in top..=bot {
                    set_c(buf, px, py, '·',
                        Color::Rgb((40.0 * fade) as u8, (90.0 * fade) as u8, (50.0 * fade) as u8), area);
                }
            }
        }

        let mut prev_top: Option<i32> = None;
        for col in 0..len {
            let r = col as f64 / len as f64;
            let cy = cy_at(col);
            let half = if r < 0.08 { r / 0.08 * max_half }
                else if r > 0.90 { (1.0 - r) / 0.10 * max_half }
                else { max_half };

            let px = sx + col;
            if px < 0 || px >= area.width as i32 { continue; }

            let top = (cy - half).round() as i32;
            let bot = (cy + half).round() as i32;
            let slope = cy_at((col + 1).min(len - 1)) - cy;
            let (top_ch, bot_ch) = edge_chars(slope);

            // Gap fill
            if let Some(pt) = prev_top {
                if top < pt - 1 {
                    for gy in (top + 1)..pt { set_c(buf, px - 1, gy, '/', Color::Rgb(30, 70, 40), area); }
                } else if top > pt + 1 {
                    for gy in (pt + 1)..top { set_c(buf, px, gy, '\\', Color::Rgb(30, 70, 40), area); }
                }
            }

            for py in top..=bot {
                let vert = if top == bot { 0.5 } else { (py - top) as f64 / (bot - top) as f64 };
                let cr = 25.0 + 40.0 * vert;
                let cg = 60.0 + 50.0 * vert;
                let cb = 30.0 + 30.0 * vert;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb(cr as u8, cg as u8, cb as u8))
                } else if py == top {
                    (top_ch, Color::Rgb(25, 60, 30))
                } else if py == bot {
                    (bot_ch, Color::Rgb(55, 100, 55))
                } else {
                    ('~', Color::Rgb(cr as u8, cg as u8, cb as u8))
                };
                if ch != ' ' { set_c(buf, px, py, ch, fg, area); }
            }
            prev_top = Some(top);
        }

        let eye_x = sx + (len as f64 * 0.06).round() as i32;
        let eye_y = cy_at((len as f64 * 0.06) as i32).round() as i32;
        set_c(buf, eye_x, eye_y, 'o', Color::Rgb(180, 200, 160), area);
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn edge_chars(slope: f64) -> (char, char) {
    if slope < -0.5 { ('/', '\\') }
    else if slope < -0.15 { ('\'', '.') }
    else if slope > 0.5 { ('\\', '/') }
    else if slope > 0.15 { ('.', '\'') }
    else { ('\u{2500}', '\u{2500}') }
}

fn set_c(buf: &mut ratatui::buffer::Buffer, x: i32, y: i32, ch: char, fg: Color, area: ratatui::layout::Rect) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_style(Style::default());
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

const CREATURES: &[&dyn Creature] = &[&SharkCfg, &EelCfg];

struct App {
    current: usize,
    exit: bool,
    elapsed: f64,
    speed: f64,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut app = App { current: 0, exit: false, elapsed: 0.0, speed: 1.0 };
    let mut last = Instant::now();

    while !app.exit {
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

            CREATURES[app.current].draw(buf, area, app.elapsed, app.speed);

            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  {} ({}/{})  speed:{:.1}x  \u{2190}\u{2192}:switch  \u{2191}\u{2193}:speed  q:quit",
                    CREATURES[app.current].name(), app.current + 1, CREATURES.len(), app.speed
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
                        KeyCode::Char('q') | KeyCode::Esc => app.exit = true,
                        KeyCode::Right | KeyCode::Char('n') => {
                            app.current = (app.current + 1) % CREATURES.len();
                        }
                        KeyCode::Left | KeyCode::Char('p') => {
                            app.current = if app.current == 0 { CREATURES.len() - 1 } else { app.current - 1 };
                        }
                        KeyCode::Up => { app.speed = (app.speed + 0.2).min(5.0); }
                        KeyCode::Down => { app.speed = (app.speed - 0.2).max(0.2); }
                        _ => {}
                    }
                }
            }
        }
        app.elapsed += last.elapsed().as_secs_f64();
        last = Instant::now();
    }

    ratatui::restore();
    Ok(())
}
