use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Braille canvas ─────────────────────────────────────────────────────────

const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_DOT: [[u32; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40],
    [0x08, 0x10, 0x20, 0x80],
];

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<(bool, u8, u8, u8)>,
}

impl Canvas {
    fn new(cw: usize, ch: usize) -> Self {
        Canvas { w: cw * 2, h: ch * 4, px: vec![(false, 0, 0, 0); cw * 2 * ch * 4] }
    }

    fn dot(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }

    fn render(&self, buf: &mut ratatui::buffer::Buffer, ox: u16, oy: u16, area: ratatui::layout::Rect) {
        let cw = self.w / 2;
        let ch = self.h / 4;
        for cy in 0..ch {
            for cx in 0..cw {
                let mut bits = 0u32;
                let (mut tr, mut tg, mut tb, mut n) = (0u32, 0u32, 0u32, 0u32);
                for dy in 0..4usize {
                    for dx in 0..2usize {
                        let (on, r, g, b) = self.px[(cy * 4 + dy) * self.w + cx * 2 + dx];
                        if on {
                            bits |= BRAILLE_DOT[dx][dy];
                            tr += r as u32; tg += g as u32; tb += b as u32; n += 1;
                        }
                    }
                }
                if bits == 0 { continue; }
                let sx = ox as i32 + cx as i32;
                let sy = oy as i32 + cy as i32;
                if sx < 0 || sy < 0 || sx >= area.width as i32 || sy >= area.height as i32 { continue; }
                let cell = &mut buf[(sx as u16, sy as u16)];
                cell.set_char(char::from_u32(BRAILLE_BASE + bits).unwrap_or(' '));
                cell.set_fg(Color::Rgb((tr / n) as u8, (tg / n) as u8, (tb / n) as u8));
                cell.set_style(Style::default());
            }
        }
    }
}

// ─── Koi SDF ────────────────────────────────────────────────────────────────

fn koi_spine(s: f64, t: f64) -> (f64, f64) {
    let wave = (s * 2.5 - t * 2.0).sin() * s * s * 0.35;
    ((1.0 - s) * 6.0, wave) // body length = 6 units
}

fn koi_width(s: f64) -> f64 {
    if s < 0.05 { s / 0.05 * 0.7 }
    else if s < 0.15 { 0.7 + (s - 0.05) / 0.10 * 0.6 }
    else if s < 0.35 { 1.3 }
    else if s < 0.75 { 1.3 - (s - 0.35) / 0.40 * 0.6 }
    else { 0.7 * (1.0 - s) / 0.25 }
}

// Kohaku pattern: 3 distinct red patches (head, middle, near-tail)
fn koi_red(s: f64, perp: f64, id: f64) -> bool {
    let offset = id * 0.7;
    // Patch 1: head cap
    let p1 = s < 0.18 && s > 0.03 && perp.abs() < 0.7;
    // Patch 2: large body patch
    let p2 = s > (0.30 + offset * 0.05) && s < (0.55 + offset * 0.05) && perp.abs() < 0.8;
    // Patch 3: small spot near tail
    let p3 = s > 0.65 && s < 0.78 && perp.abs() < 0.5;
    p1 || p2 || p3
}

fn draw_koi(canvas: &mut Canvas, t: f64, cx: f64, cy: f64, scale: f64, heading: f64, id: f64) {
    let cos_h = heading.cos();
    let sin_h = heading.sin();

    let transform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((cx + wx * scale) as i32, (cy + wy * scale) as i32)
    };

    let tangent_at = |s: f64| -> (f64, f64, f64, f64) {
        let ds = 0.01;
        let (x1, y1) = koi_spine(s, t);
        let (x2, y2) = koi_spine((s + ds).min(1.0), t);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        (-dy / l, dx / l, x1, y1) // (nx, ny, px, py)
    };

    // Body
    for si in 0..60 {
        let s = si as f64 / 60.0;
        let hw = koi_width(s);
        let (nx, ny, px, py) = tangent_at(s);

        let perp_n = (hw * scale * 2.0) as i32 + 1;
        for pi in -perp_n..=perp_n {
            let p = pi as f64 / scale;
            if p.abs() > hw { continue; }
            let norm_p = p / hw; // -1..1

            let lx = px + nx * p;
            let ly = py + ny * p;
            let (sx, sy) = transform(lx, ly);

            let edge = 1.0 - norm_p.abs().powi(2);
            let is_red = koi_red(s, norm_p, id);

            let (r, g, b) = if is_red {
                ((200.0 * edge) as u8, (50.0 * edge) as u8, (35.0 * edge) as u8)
            } else {
                ((230.0 * edge) as u8, (225.0 * edge) as u8, (215.0 * edge) as u8)
            };
            canvas.dot(sx, sy, r, g, b);
        }
    }

    // Pectoral fins
    let fin_flap = (t * 2.5).sin() * 0.25;
    for side in [-1.0, 1.0] {
        let (nx, ny, fpx, fpy) = tangent_at(0.22);
        for fi in 0..12 {
            let ft = fi as f64 / 12.0;
            let spread = side * (1.3 + ft * 1.0 + fin_flap);
            let along = -ft * 1.5;
            let (dx, dy) = koi_spine(0.22, t);
            let (dx2, dy2) = koi_spine(0.23, t);
            let tdx = dx2 - dx;
            let tdy = dy2 - dy;
            let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

            let fx = fpx + nx * spread + tdx / tl * along;
            let fy = fpy + ny * spread + tdy / tl * along;
            let (sx, sy) = transform(fx, fy);
            let a = (1.0 - ft) * 0.5;
            canvas.dot(sx, sy, (190.0 * a) as u8, (185.0 * a) as u8, (175.0 * a) as u8);
        }
    }

    // Tail fin (two lobes)
    let tail_sway = (t * 2.0).sin() * 0.4;
    for lobe in [-1.0, 1.0] {
        for ti in 0..18 {
            let ft = ti as f64 / 18.0;
            let s_pos = 0.88 + ft * 0.12;
            let (tnx, tny, tpx, tpy) = tangent_at(s_pos.min(0.99));
            let spread = lobe * (0.3 + ft * 1.8 + tail_sway * ft);

            let tx = tpx + tnx * spread;
            let ty = tpy + tny * spread;
            let (sx, sy) = transform(tx, ty);
            let a = (1.0 - ft * 0.4) * 0.6;
            canvas.dot(sx, sy, (200.0 * a) as u8, (190.0 * a) as u8, (175.0 * a) as u8);
        }
    }

    // Eyes
    for eye_side in [-0.4, 0.4] {
        let (enx, _eny, epx, epy) = tangent_at(0.06);
        let ex = epx + enx * eye_side;
        let ey = epy + _eny * eye_side;
        let (sx, sy) = transform(ex, ey);
        canvas.dot(sx, sy, 10, 10, 15);
        canvas.dot(sx + 1, sy, 10, 10, 15);
    }
}

// ─── Koi movement ───────────────────────────────────────────────────────────

struct Koi {
    x: f64, y: f64, heading: f64,
    speed: f64, turn_rate: f64, turn_timer: f64, id: f64,
}

fn update_koi(k: &mut Koi, dt: f64, t: f64) {
    k.turn_timer -= dt;
    if k.turn_timer <= 0.0 {
        let s1 = ((k.id * 7.3 + t * 3.1).sin() * 1e4).fract();
        let s2 = ((k.id * 11.7 + t * 2.3).cos() * 1e4).fract();
        // Sometimes go straight, sometimes turn, sometimes sharp turn
        k.turn_rate = if s1 > 0.7 { (s1 - 0.7) * 3.0 }   // sharp right
            else if s1 < 0.3 { -(0.3 - s1) * 3.0 }        // sharp left
            else { (s1 - 0.5) * 0.5 };                     // gentle
        k.turn_timer = 1.5 + s2 * 3.5;
    }

    // Wall: gently push heading toward center
    let margin = 0.25;
    let cx = 0.5;
    let cy = 0.5;
    if k.x < margin || k.x > 1.0 - margin || k.y < margin || k.y > 1.0 - margin {
        let to_center = (cy - k.y).atan2(cx - k.x);
        let diff = (to_center - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.turn_rate += diff * 0.3 * dt;
    }

    k.turn_rate = k.turn_rate.clamp(-1.2, 1.2);
    k.heading += k.turn_rate * dt;

    let spd = k.speed * (0.8 + 0.2 * (t * 0.4 + k.id).sin());
    k.x += k.heading.cos() * spd * dt * 0.01;
    k.y += k.heading.sin() * spd * dt * 0.02;
    k.x = k.x.clamp(0.08, 0.92);
    k.y = k.y.clamp(0.08, 0.92);
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut fish = vec![
        Koi { x: 0.35, y: 0.4, heading: 0.3, speed: 1.8, turn_rate: 0.0, turn_timer: 2.0, id: 1.0 },
        Koi { x: 0.65, y: 0.6, heading: 2.8, speed: 1.5, turn_rate: 0.2, turn_timer: 1.5, id: 3.2 },
        Koi { x: 0.5, y: 0.3, heading: 4.5, speed: 1.3, turn_rate: -0.1, turn_timer: 3.0, id: 5.7 },
    ];

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        for k in &mut fish {
            update_koi(k, dt, elapsed);
        }

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Background: dark water
            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64;
                    let yf = y as f64;
                    let ripple = ((xf * 0.12 + yf * 0.2 + elapsed * 0.4).sin()
                        * (xf * 0.07 - elapsed * 0.25).cos()) * 0.5 + 0.5;
                    let r = (8.0 + ripple * 4.0) as u8;
                    let g = (13.0 + ripple * 7.0) as u8;
                    let b = (22.0 + ripple * 10.0) as u8;

                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(r, g, b));
                    cell.set_fg(Color::Rgb(r, g, b));
                }
            }

            // Braille canvas
            let cw = area.width as usize;
            let ch = (area.height as usize).saturating_sub(1);
            if cw < 4 || ch < 4 { return; }
            let mut canvas = Canvas::new(cw, ch);
            let koi_scale = ch as f64 * 4.0 / 28.0; // koi is ~6 units, fits in ~1/4 of screen height

            for k in &fish {
                let kcx = k.x * canvas.w as f64;
                let kcy = k.y * canvas.h as f64;
                draw_koi(&mut canvas, elapsed, kcx, kcy, koi_scale, k.heading, k.id);
            }

            canvas.render(buf, 0, 1, area);

            // Header
            if area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
                    speed
                );
                for (i, ch) in hdr.chars().enumerate() {
                    if i >= area.width as usize { break; }
                    let cell = &mut buf[(i as u16, 0)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(50, 45, 75));
                    cell.set_bg(Color::Rgb(8, 12, 20));
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
