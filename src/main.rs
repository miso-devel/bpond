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
    w: usize, h: usize, cw: usize, ch: usize,
    px: Vec<(bool, u8, u8, u8)>,
}

impl Canvas {
    fn new(cw: usize, ch: usize) -> Self {
        Canvas { w: cw * 2, h: ch * 4, cw, ch, px: vec![(false, 0, 0, 0); cw * 2 * ch * 4] }
    }
    fn dot(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }
    fn fat(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in 0..2 { for dx in 0..2 { self.dot(x + dx, y + dy, r, g, b); } }
    }
    fn render(&self, buf: &mut ratatui::buffer::Buffer, ox: u16, oy: u16, area: ratatui::layout::Rect) {
        for cy in 0..self.ch {
            for cx in 0..self.cw {
                let mut bits = 0u32;
                let (mut tr, mut tg, mut tb, mut n) = (0u32, 0u32, 0u32, 0u32);
                for dy in 0..4usize {
                    for dx in 0..2usize {
                        let (on, r, g, b) = self.px[(cy * 4 + dy) * self.w + cx * 2 + dx];
                        if on { bits |= BRAILLE_DOT[dx][dy]; tr += r as u32; tg += g as u32; tb += b as u32; n += 1; }
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

// ─── Koi biomechanics (Videler 1993, subcarangiform) ────────────────────────

const BODY_LEN: f64 = 5.0; // arbitrary units
const FREQ: f64 = 1.2;     // tail beat Hz

// Amplitude envelope: A(x) = L * (0.02 - 0.08x + 0.16x²)
// Head moves slightly, node at x≈0.2, tail tip ≈ 10% of body length
fn amplitude(x: f64) -> f64 {
    BODY_LEN * (0.02 - 0.08 * x + 0.16 * x * x)
}

// Body midline: traveling wave + turn curvature + asymmetric beat
fn midline(x: f64, t: f64, turn: f64, freq: f64) -> f64 {
    let a = amplitude(x);
    let k = 2.0 * PI / BODY_LEN; // wavelength ≈ 1 body length
    let omega = 2.0 * PI * freq;
    let wave = a * (k * x * BODY_LEN - omega * t).sin();

    // Turn curvature: concentrated posteriorly (x^1.8)
    let turn_curve = turn * 0.20 * BODY_LEN * x.powf(1.8);

    // Asymmetric amplitude during turn
    let asym = if (wave > 0.0 && turn > 0.0) || (wave < 0.0 && turn < 0.0) {
        1.0 + 0.25 * turn.abs().min(1.0)
    } else {
        1.0 - 0.25 * turn.abs().min(1.0)
    };

    turn_curve + wave * asym
}

// Body half-width
fn body_width(s: f64) -> f64 {
    if s < 0.05 { s / 0.05 * 0.5 }
    else if s < 0.2 { 0.5 + (s - 0.05) / 0.15 * 0.5 }
    else if s < 0.4 { 1.0 }
    else if s < 0.75 { 1.0 - (s - 0.4) / 0.35 * 0.45 }
    else { 0.55 * (1.0 - s) / 0.25 }
}

// Red/white pattern
fn is_red(s: f64, np: f64, id: f64) -> bool {
    let off = (id * 1.3).sin() * 0.06;
    (s > 0.04 && s < 0.14 && np.abs() < 0.55)
        || (s > (0.28 + off) && s < (0.48 + off) && np.abs() < 0.7)
        || (s > 0.60 && s < 0.72 && np.abs() < 0.4)
}

fn spine_at(s: f64, t: f64, turn: f64, freq: f64) -> (f64, f64) {
    let x = (1.0 - s) * BODY_LEN;
    let y = midline(s, t, turn, freq);
    (x, y)
}

fn tangent(s: f64, t: f64, turn: f64, freq: f64) -> (f64, f64, f64, f64) {
    let ds = 0.005;
    let (x1, y1) = spine_at(s, t, turn, freq);
    let (x2, y2) = spine_at((s + ds).min(1.0), t, turn, freq);
    let dx = x2 - x1; let dy = y2 - y1;
    let l = (dx * dx + dy * dy).sqrt().max(0.001);
    (-dy / l, dx / l, x1, y1)
}

fn draw_koi(canvas: &mut Canvas, t: f64, cx: f64, cy: f64, scale: f64, koi: &Koi) {
    let cos_h = koi.heading.cos();
    let sin_h = koi.heading.sin();
    let freq = FREQ * (0.85 + 0.15 * (t * 0.3 + koi.id).sin());

    let xform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((cx + wx * scale) as i32, (cy + wy * scale) as i32)
    };

    // Shadow
    for si in 0..40 {
        let s = si as f64 / 40.0;
        let hw = body_width(s) * 0.75;
        let (nx, ny, px, py) = tangent(s, t, koi.turn_rate, freq);
        let steps = (hw * scale * 1.5) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let (sx, sy) = xform(px + nx * p, py + ny * p);
            canvas.dot(sx + 3, sy + 4, 5, 9, 16);
        }
    }

    // Tail fin (0.15L, two lobes, pitch leads by ~90°)
    let tail_pitch = (2.0 * PI * freq * t).cos() * 0.3; // cos = 90° phase lead
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..16 {
            let ft = ti as f64 / 16.0;
            let s_pos = (0.85 + ft * 0.15).min(0.99);
            let (tnx, tny, tpx, tpy) = tangent(s_pos, t, koi.turn_rate, freq);
            let spread = lobe * (0.1 + ft * 1.2 + tail_pitch * ft);
            let tx = tpx + tnx * spread;
            let ty = tpy + tny * spread;
            let (sx, sy) = xform(tx, ty);
            let a = (1.0 - ft * 0.3) * 0.55;
            canvas.fat(sx, sy, (200.0 * a) as u8, (190.0 * a) as u8, (175.0 * a) as u8);
        }
    }

    // Pectoral fins (at x≈0.22, short, left/right alternate)
    let pec_rest = 12.0f64.to_radians();
    let pec_amp = 25.0f64.to_radians();
    for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
        let phase = if is_left { 0.0 } else { PI };
        let angle = pec_rest + pec_amp * (2.0 * PI * freq * t + phase).sin();

        let (fnx, fny, fpx, fpy) = tangent(0.22, t, koi.turn_rate, freq);
        let (_, _, fpx2, fpy2) = tangent(0.24, t, koi.turn_rate, freq);
        let tdx = fpx2 - fpx; let tdy = fpy2 - fpy;
        let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

        let fin_len = BODY_LEN * 0.12; // short fins
        for fi in 0..8 {
            let ft = fi as f64 / 8.0;
            let spread = side * (angle.sin() * (1.0 - ft * 0.5)) * 1.2;
            let along = -ft * fin_len;
            let fx = fpx + fnx * spread + tdx / tl * along;
            let fy = fpy + fny * spread + tdy / tl * along;
            let (sx, sy) = xform(fx, fy);
            let a = (1.0 - ft) * 0.5;
            canvas.dot(sx, sy, (190.0 * a) as u8, (182.0 * a) as u8, (168.0 * a) as u8);
            canvas.dot(sx + 1, sy, (185.0 * a) as u8, (178.0 * a) as u8, (164.0 * a) as u8);
        }
    }

    // Body
    for si in 0..60 {
        let s = si as f64 / 60.0;
        let hw = body_width(s);
        let (nx, ny, px, py) = tangent(s, t, koi.turn_rate, freq);

        let steps = (hw * scale * 2.0) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let np = (p / hw).abs();

            let (sx, sy) = xform(px + nx * p, py + ny * p);
            let outline = np > 0.82;
            let red = is_red(s, p / hw, koi.id);

            let (r, g, b) = if outline { (48, 44, 36) }
                else if red { (218, 56, 36) }
                else { (240, 236, 225) };
            canvas.fat(sx, sy, r, g, b);
        }
    }
}

// ─── Koi state ──────────────────────────────────────────────────────────────

struct Koi {
    x: f64, y: f64, heading: f64,
    speed: f64, turn_rate: f64, target_turn: f64,
    turn_timer: f64, id: f64,
}

fn update_koi(k: &mut Koi, dt: f64, t: f64, w: f64, h: f64) {
    k.turn_timer -= dt;
    if k.turn_timer <= 0.0 {
        let s1 = ((k.id * 7.3 + t * 3.1).sin() * 1e4).fract();
        let s2 = ((k.id * 11.7 + t * 2.3).cos() * 1e4).fract();
        k.target_turn = if s1 > 0.8 { 0.6 }
            else if s1 < 0.2 { -0.6 }
            else if s1 > 0.6 { 0.2 }
            else if s1 < 0.4 { -0.2 }
            else { 0.0 };
        k.turn_timer = 1.5 + s2 * 4.0;
    }

    // Wall avoidance: gentle continuous steering toward center of allowed area
    let margin_x = w * 0.15;
    let margin_y = h * 0.15;
    let center_x = w / 2.0;
    let center_y = h / 2.0;

    let dx = k.x - center_x;
    let dy = k.y - center_y;
    let safe_rx = w / 2.0 - margin_x;
    let safe_ry = h / 2.0 - margin_y;

    // How far outside the safe ellipse (0 = inside, >0 = outside)
    let ellipse_dist = (dx / safe_rx).powi(2) + (dy / safe_ry).powi(2);

    if ellipse_dist > 0.7 {
        let overshoot = ((ellipse_dist - 0.7) / 0.3).clamp(0.0, 1.0);
        let to_center = (center_y - k.y).atan2(center_x - k.x);
        let diff = (to_center - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.target_turn += diff * overshoot * 0.8;
    }

    // Smoothly approach target turn rate (no sudden jumps)
    let turn_speed = 1.5 * dt;
    if (k.target_turn - k.turn_rate).abs() < turn_speed {
        k.turn_rate = k.target_turn;
    } else if k.target_turn > k.turn_rate {
        k.turn_rate += turn_speed;
    } else {
        k.turn_rate -= turn_speed;
    }
    k.turn_rate = k.turn_rate.clamp(-0.7, 0.7);

    k.heading += k.turn_rate * dt;

    let spd = k.speed * (0.85 + 0.15 * (t * 0.3 + k.id).sin());
    k.x += k.heading.cos() * spd * dt;
    k.y += k.heading.sin() * spd * dt;

    // Hard clamp (shouldn't normally hit)
    k.x = k.x.clamp(margin_x * 0.5, w - margin_x * 0.5);
    k.y = k.y.clamp(margin_y * 0.5, h - margin_y * 0.5);
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
    let mut koi = Koi {
        x: tw as f64 / 2.0, y: th as f64 / 2.0, heading: 0.5,
        speed: 4.0, turn_rate: 0.0, target_turn: 0.0, turn_timer: 2.0, id: 1.0,
    };

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
        update_koi(&mut koi, dt, elapsed, tw as f64, th as f64);

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Water
            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64; let yf = y as f64;
                    let r = ((xf * 0.08 + yf * 0.14 + elapsed * 0.25).sin()
                        * (xf * 0.05 - elapsed * 0.15).cos()) * 0.5 + 0.5;
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(
                        (10.0 + r * 4.0) as u8,
                        (18.0 + r * 7.0) as u8,
                        (32.0 + r * 10.0) as u8,
                    ));
                    cell.set_fg(Color::Rgb(10, 18, 32));
                }
            }

            let cw = area.width as usize;
            let ch = (area.height as usize).saturating_sub(1);
            if cw < 4 || ch < 4 { return; }
            let mut canvas = Canvas::new(cw, ch);

            // Scale: koi body = ~5 units, should fill about 1/3 of screen height
            let koi_scale = (ch as f64 * 4.0 / 6.0).min(cw as f64 * 2.0 / 6.0);

            // Convert koi position (in terminal cell coords) to canvas sub-pixel coords
            let kcx = koi.x / area.width as f64 * canvas.w as f64;
            let kcy = koi.y / area.height as f64 * canvas.h as f64;

            draw_koi(&mut canvas, elapsed, kcx, kcy, koi_scale, &koi);
            canvas.render(buf, 0, 1, area);

            if area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit", speed
                );
                for (i, ch) in hdr.chars().enumerate() {
                    if i >= area.width as usize { break; }
                    let cell = &mut buf[(i as u16, 0)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(60, 55, 85));
                    cell.set_bg(Color::Rgb(10, 16, 28));
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
