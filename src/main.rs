use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Hybrid canvas: braille sub-pixels + direct character overlay ───────────

const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_DOT: [[u32; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40],
    [0x08, 0x10, 0x20, 0x80],
];

struct Canvas {
    w: usize, h: usize, // sub-pixel dimensions
    cw: usize, ch: usize, // character cell dimensions
    px: Vec<(bool, u8, u8, u8)>,
    // Overlay: direct character placement on specific cells (eyes, mouth, etc.)
    overlay: Vec<Option<(char, u8, u8, u8)>>,
}

impl Canvas {
    fn new(cw: usize, ch: usize) -> Self {
        Canvas {
            w: cw * 2, h: ch * 4, cw, ch,
            px: vec![(false, 0, 0, 0); cw * 2 * ch * 4],
            overlay: vec![None; cw * ch],
        }
    }

    fn dot(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }

    fn fat(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in 0..2 { for dx in 0..2 { self.dot(x + dx, y + dy, r, g, b); } }
    }

    // Place a character directly on a cell (overrides braille for that cell)
    fn char_at(&mut self, cell_x: i32, cell_y: i32, ch: char, r: u8, g: u8, b: u8) {
        if cell_x >= 0 && cell_y >= 0 && (cell_x as usize) < self.cw && (cell_y as usize) < self.ch {
            self.overlay[cell_y as usize * self.cw + cell_x as usize] = Some((ch, r, g, b));
        }
    }

    // Convert sub-pixel coords to cell coords
    fn to_cell(&self, sub_x: i32, sub_y: i32) -> (i32, i32) {
        (sub_x / 2, sub_y / 4)
    }

    fn render(&self, buf: &mut ratatui::buffer::Buffer, ox: u16, oy: u16, area: ratatui::layout::Rect) {
        for cy in 0..self.ch {
            for cx in 0..self.cw {
                let sx = ox as i32 + cx as i32;
                let sy = oy as i32 + cy as i32;
                if sx < 0 || sy < 0 || sx >= area.width as i32 || sy >= area.height as i32 { continue; }

                // Check overlay first
                if let Some((ch, r, g, b)) = self.overlay[cy * self.cw + cx] {
                    let cell = &mut buf[(sx as u16, sy as u16)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(r, g, b));
                    cell.set_style(Style::default());
                    continue;
                }

                // Braille
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
                let cell = &mut buf[(sx as u16, sy as u16)];
                cell.set_char(char::from_u32(BRAILLE_BASE + bits).unwrap_or(' '));
                cell.set_fg(Color::Rgb((tr / n) as u8, (tg / n) as u8, (tb / n) as u8));
                cell.set_style(Style::default());
            }
        }
    }
}

// ─── Koi SDF ────────────────────────────────────────────────────────────────

fn koi_spine(s: f64, t: f64, turn: f64) -> (f64, f64) {
    let swim = (s * 2.5 - t * 2.2).sin() * s * s * 0.3;
    let bend = turn * s * s * 2.0;
    ((1.0 - s) * 5.0, swim + bend)
}

fn koi_width(s: f64) -> f64 {
    if s < 0.05 { s / 0.05 * 0.6 }
    else if s < 0.2 { 0.6 + (s - 0.05) / 0.15 * 0.5 }
    else if s < 0.4 { 1.1 }
    else if s < 0.75 { 1.1 - (s - 0.4) / 0.35 * 0.5 }
    else { 0.6 * (1.0 - s) / 0.25 }
}

fn koi_red(s: f64, np: f64, id: f64) -> bool {
    let off = (id * 1.3).sin() * 0.06;
    (s > 0.04 && s < 0.15 && np.abs() < 0.55)
        || (s > (0.28 + off) && s < (0.50 + off) && np.abs() < 0.7)
        || (s > 0.62 && s < 0.74 && np.abs() < 0.4)
}

fn tangent_at(s: f64, t: f64, turn: f64) -> (f64, f64, f64, f64) {
    let ds = 0.01;
    let (x1, y1) = koi_spine(s, t, turn);
    let (x2, y2) = koi_spine((s + ds).min(1.0), t, turn);
    let dx = x2 - x1; let dy = y2 - y1;
    let l = (dx * dx + dy * dy).sqrt().max(0.001);
    (-dy / l, dx / l, x1, y1)
}

fn draw_koi(canvas: &mut Canvas, t: f64, cx: f64, cy: f64, scale: f64, koi: &Koi) {
    let cos_h = koi.heading.cos();
    let sin_h = koi.heading.sin();

    let xform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((cx + wx * scale) as i32, (cy + wy * scale) as i32)
    };

    // ── Shadow ──
    for si in 0..40 {
        let s = si as f64 / 40.0;
        let hw = koi_width(s) * 0.8;
        let (nx, ny, px, py) = tangent_at(s, t, koi.turn_rate);
        let steps = (hw * scale * 1.5) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let (sx, sy) = xform(px + nx * p, py + ny * p);
            canvas.dot(sx + 3, sy + 4, 5, 9, 16);
        }
    }

    // ── Tail fin ──
    let tail_sway = (t * 2.0).sin() * 0.4;
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..24 {
            let ft = ti as f64 / 24.0;
            let s_pos = (0.82 + ft * 0.18).min(0.99);
            let (tnx, tny, tpx, tpy) = tangent_at(s_pos, t, koi.turn_rate);
            let spread = lobe * (0.15 + ft * 2.2 + tail_sway * ft);
            let tx = tpx + tnx * spread;
            let ty = tpy + tny * spread;
            let (sx, sy) = xform(tx, ty);
            let a = (1.0 - ft * 0.3) * 0.6;
            canvas.fat(sx, sy, (205.0 * a) as u8, (195.0 * a) as u8, (180.0 * a) as u8);
        }
    }

    // ── Pectoral fins ──
    let fin_flap = (t * 2.5).sin() * 0.25;
    for side in [-1.0f64, 1.0] {
        let (fnx, fny, fpx, fpy) = tangent_at(0.22, t, koi.turn_rate);
        let (_, _, fpx2, fpy2) = tangent_at(0.24, t, koi.turn_rate);
        let tdx = fpx2 - fpx; let tdy = fpy2 - fpy;
        let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

        for fi in 0..16 {
            let ft = fi as f64 / 16.0;
            let spread = side * (1.1 + ft * 1.2 + fin_flap);
            let along = -ft * 1.8;
            let fx = fpx + fnx * spread + tdx / tl * along;
            let fy = fpy + fny * spread + tdy / tl * along;
            let (sx, sy) = xform(fx, fy);
            let a = (1.0 - ft) * 0.55;
            canvas.fat(sx, sy, (198.0 * a) as u8, (190.0 * a) as u8, (175.0 * a) as u8);
        }
    }

    // ── Body ──
    for si in 0..60 {
        let s = si as f64 / 60.0;
        let hw = koi_width(s);
        let (nx, ny, px, py) = tangent_at(s, t, koi.turn_rate);

        let steps = (hw * scale * 2.0) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let np = (p / hw).abs();

            let lx = px + nx * p;
            let ly = py + ny * p;
            let (sx, sy) = xform(lx, ly);

            let is_outline = np > 0.82;
            let is_red = koi_red(s, p / hw, koi.id);

            let (r, g, b) = if is_outline {
                (50, 45, 38)
            } else if is_red {
                (218, 56, 36)
            } else {
                (240, 236, 225)
            };
            canvas.fat(sx, sy, r, g, b);
        }
    }

    // ── Eyes (ASCII overlay for sharpness) ──
    for eye_side in [-0.32f64, 0.32] {
        let (enx, eny, epx, epy) = tangent_at(0.06, t, koi.turn_rate);
        let ex = epx + enx * eye_side;
        let ey = epy + eny * eye_side;
        let (sx, sy) = xform(ex, ey);
        let (ccx, ccy) = canvas.to_cell(sx, sy);
        canvas.char_at(ccx, ccy, '◉', 10, 10, 18);
    }

    // ── Mouth (subtle line at head tip) ──
    let (_, _, mpx, mpy) = tangent_at(0.01, t, koi.turn_rate);
    let (mx, my) = xform(mpx, mpy);
    let (mcx, mcy) = canvas.to_cell(mx, my);
    canvas.char_at(mcx, mcy, '·', 160, 140, 120);
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
        // More variety: sometimes straight, sometimes curve, sometimes U-turn
        k.turn_rate = if s1 > 0.8 { 0.7 + s1 * 0.3 }       // sharp right
            else if s1 < 0.2 { -(0.7 + (1.0 - s1) * 0.3) }  // sharp left
            else if s1 > 0.6 { (s1 - 0.5) * 0.6 }            // gentle right
            else if s1 < 0.4 { (s1 - 0.5) * 0.6 }            // gentle left
            else { 0.0 };                                      // straight
        k.turn_timer = 1.0 + s2 * 3.5;
    }

    // Keep within a defined swimming area (center 60% of screen)
    let bound = 0.20;
    let soft = 0.30;
    let dx = k.x - 0.5;
    let dy = k.y - 0.5;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist > bound {
        let overshoot = ((dist - bound) / (soft - bound)).clamp(0.0, 1.0);
        let to_center = (0.5 - k.y).atan2(0.5 - k.x);
        let diff = (to_center - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.turn_rate += diff * overshoot * 1.5 * dt;
    }

    k.turn_rate = k.turn_rate.clamp(-0.9, 0.9);
    k.heading += k.turn_rate * dt;

    let spd = k.speed * (0.85 + 0.15 * (t * 0.3 + k.id).sin());
    k.x += k.heading.cos() * spd * dt * 0.006;
    k.y += k.heading.sin() * spd * dt * 0.012;
    k.x = k.x.clamp(0.12, 0.88);
    k.y = k.y.clamp(0.12, 0.88);
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut koi = Koi {
        x: 0.5, y: 0.5, heading: 0.5,
        speed: 1.8, turn_rate: 0.0, turn_timer: 2.0, id: 1.0,
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

            // Water
            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64; let yf = y as f64;
                    let r = ((xf * 0.1 + yf * 0.17 + elapsed * 0.3).sin()
                        * (xf * 0.06 - elapsed * 0.18).cos()) * 0.5 + 0.5;
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(
                        (10.0 + r * 5.0) as u8,
                        (18.0 + r * 8.0) as u8,
                        (32.0 + r * 12.0) as u8,
                    ));
                    cell.set_fg(Color::Rgb(10, 18, 32));
                }
            }

            let cw = area.width as usize;
            let ch = (area.height as usize).saturating_sub(1);
            if cw < 4 || ch < 4 { return; }
            let mut canvas = Canvas::new(cw, ch);

            let koi_scale = (ch as f64 * 4.0 / 22.0).min(cw as f64 * 2.0 / 22.0);
            let kcx = koi.x * canvas.w as f64;
            let kcy = koi.y * canvas.h as f64;
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
