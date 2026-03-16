use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Braille rendering ─────────────────────────────────────────────────────
// Each terminal cell encodes a 2×4 pixel grid using Unicode braille (U+2800).
// This gives 2x horizontal and 4x vertical sub-cell resolution.
//
// Braille dot positions:
//   (0,0) (1,0)    dots 1, 4
//   (0,1) (1,1)    dots 2, 5
//   (0,2) (1,2)    dots 3, 6
//   (0,3) (1,3)    dots 7, 8

const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_MAP: [[u32; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // left column
    [0x08, 0x10, 0x20, 0x80], // right column
];

struct Canvas {
    // Pixel buffer: (char_w * 2) × (char_h * 4) sub-pixels
    // Each pixel has: filled(bool), color(r,g,b)
    w: usize,
    h: usize,
    pixels: Vec<(bool, u8, u8, u8)>,
}

impl Canvas {
    fn new(char_w: usize, char_h: usize) -> Self {
        let w = char_w * 2;
        let h = char_h * 4;
        Canvas {
            w, h,
            pixels: vec![(false, 0, 0, 0); w * h],
        }
    }

    fn set(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.pixels[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }

    fn render(&self, buf: &mut ratatui::buffer::Buffer, ox: u16, oy: u16, area: ratatui::layout::Rect) {
        let char_w = self.w / 2;
        let char_h = self.h / 4;

        for cy in 0..char_h {
            for cx in 0..char_w {
                let mut bits: u32 = 0;
                let mut tr: u32 = 0;
                let mut tg: u32 = 0;
                let mut tb: u32 = 0;
                let mut count: u32 = 0;

                for dy in 0..4 {
                    for dx in 0..2 {
                        let px = cx * 2 + dx;
                        let py = cy * 4 + dy;
                        let (filled, r, g, b) = self.pixels[py * self.w + px];
                        if filled {
                            bits |= BRAILLE_MAP[dx][dy];
                            tr += r as u32;
                            tg += g as u32;
                            tb += b as u32;
                            count += 1;
                        }
                    }
                }

                if bits == 0 { continue; }

                let sx = ox as i32 + cx as i32;
                let sy = oy as i32 + cy as i32;
                if sx < 0 || sy < 0 || sx >= area.width as i32 || sy >= area.height as i32 {
                    continue;
                }

                let ch = char::from_u32(BRAILLE_BASE + bits).unwrap_or(' ');
                let fg = Color::Rgb(
                    (tr / count) as u8,
                    (tg / count) as u8,
                    (tb / count) as u8,
                );

                let cell = &mut buf[(sx as u16, sy as u16)];
                cell.set_char(ch);
                cell.set_fg(fg);
                cell.set_style(Style::default());
            }
        }
    }
}

// ─── SDF-based koi fish (top-down, S-curve body) ────────────────────────────
// The koi body is defined as a curve (spine), with thickness around it.
// Fins and tail are separate shapes that animate.

struct KoiParams {
    heading: f64,
    speed: f64,
    turn_rate: f64,
    turn_timer: f64,
    x: f64,
    y: f64,
    id: f64,
}

fn koi_spine(s: f64, t: f64) -> (f64, f64) {
    // s = 0..1 along the body (0=head, 1=tail)
    // Returns (x, y) offset from center
    // S-curve that undulates with time
    let wave = (s * 2.5 - t * 2.0).sin() * s * s * 0.4;
    let x = (1.0 - s) * 10.0; // head at x=10, tail at x=0
    let y = wave;
    (x, y)
}

fn koi_body_half_width(s: f64) -> f64 {
    // Body cross-section: widest at s=0.3, tapers at head and tail
    if s < 0.05 { s / 0.05 * 1.2 }         // head tip
    else if s < 0.15 { 1.2 + (s - 0.05) / 0.10 * 0.8 } // head
    else if s < 0.35 { 2.0 }                // widest
    else if s < 0.75 { 2.0 - (s - 0.35) / 0.40 * 0.8 } // body taper
    else { 1.2 * (1.0 - s) / 0.25 }         // tail peduncle
}

fn koi_pattern(s: f64, perp: f64, _t: f64) -> bool {
    // Red patches: based on position along body
    let p1 = (s * 6.0 + 0.5).sin() > 0.2;
    let p2 = (perp * 3.0).cos() > -0.3;
    p1 && p2
}

fn draw_koi_sdf(canvas: &mut Canvas, t: f64, cx: f64, cy: f64, scale: f64, heading: f64) {
    let cos_h = heading.cos();
    let sin_h = heading.sin();

    // Sample many points along the spine
    let steps = 80;
    for si in 0..steps {
        let s = si as f64 / steps as f64;
        let (sx, sy) = koi_spine(s, t);
        let half_w = koi_body_half_width(s);

        // Sample perpendicular to spine
        // Compute spine tangent for perpendicular direction
        let ds = 0.01;
        let (sx2, sy2) = koi_spine((s + ds).min(1.0), t);
        let dx = sx2 - sx;
        let dy = sy2 - sy;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let nx = -dy / len; // perpendicular
        let ny = dx / len;

        let perp_steps = (half_w * scale * 2.0) as i32 + 2;
        for pi in -perp_steps..=perp_steps {
            let perp = pi as f64 / scale;
            if perp.abs() > half_w { continue; }

            // Position in local space
            let lx = sx + nx * perp;
            let ly = sy + ny * perp;

            // Rotate by heading
            let wx = lx * cos_h - ly * sin_h;
            let wy = lx * sin_h + ly * cos_h;

            // To canvas pixel coords
            let px = (cx + wx * scale) as i32;
            let py = (cy + wy * scale) as i32;

            // Color: white body with red patches
            let is_red = koi_pattern(s, perp / half_w, t);
            let edge_fade = 1.0 - (perp.abs() / half_w).powi(2);

            let (r, g, b) = if is_red {
                ((210.0 * edge_fade) as u8, (55.0 * edge_fade) as u8, (40.0 * edge_fade) as u8)
            } else {
                ((235.0 * edge_fade) as u8, (230.0 * edge_fade) as u8, (220.0 * edge_fade) as u8)
            };

            canvas.set(px, py, r, g, b);
        }
    }

    // ── Pectoral fins (at s ≈ 0.2, spread to sides) ──
    let fin_phase = (t * 2.5).sin() * 0.3; // fins flap
    for fin_side in [-1.0, 1.0] {
        let fin_s = 0.22;
        let (fsx, fsy) = koi_spine(fin_s, t);
        let ds = 0.01;
        let (fsx2, fsy2) = koi_spine(fin_s + ds, t);
        let fdx = fsx2 - fsx;
        let fdy = fsy2 - fsy;
        let flen = (fdx * fdx + fdy * fdy).sqrt().max(0.001);
        let fnx = -fdy / flen;
        let fny = fdx / flen;

        for fi in 0..20 {
            let ft = fi as f64 / 20.0;
            let _fin_len = 2.5 * (1.0 - ft * 0.7);
            let fin_spread = fin_side * (2.0 + ft * 1.5 + fin_phase);

            let fx = fsx + fnx * fin_spread - fdx / flen * ft * 3.0;
            let fy = fsy + fny * fin_spread - fdy / flen * ft * 3.0;

            let wx = fx * cos_h - fy * sin_h;
            let wy = fx * sin_h + fy * cos_h;

            let px = (cx + wx * scale) as i32;
            let py = (cy + wy * scale) as i32;

            let alpha = (1.0 - ft) * 0.6;
            canvas.set(px, py,
                (200.0 * alpha) as u8,
                (195.0 * alpha) as u8,
                (185.0 * alpha) as u8);
        }
    }

    // ── Tail fin (at s ≈ 0.9-1.0, two lobes) ──
    let tail_wave = (t * 2.0).sin() * 0.5;
    for lobe in [-1.0, 1.0] {
        for ti in 0..25 {
            let ft = ti as f64 / 25.0;
            let (tsx, tsy) = koi_spine(0.92 + ft * 0.08, t);
            let spread = lobe * (0.5 + ft * 2.5 + tail_wave * ft);

            let ds = 0.01;
            let (tsx2, tsy2) = koi_spine((0.92 + ft * 0.08 + ds).min(1.0), t);
            let tdx = tsx2 - tsx;
            let tdy = tsy2 - tsy;
            let tlen = (tdx * tdx + tdy * tdy).sqrt().max(0.001);
            let tnx = -tdy / tlen;
            let tny = tdx / tlen;

            let tx = tsx + tnx * spread;
            let ty = tsy + tny * spread;

            let wx = tx * cos_h - ty * sin_h;
            let wy = tx * sin_h + ty * cos_h;

            let px = (cx + wx * scale) as i32;
            let py = (cy + wy * scale) as i32;

            let alpha = (1.0 - ft * 0.5) * 0.7;
            canvas.set(px, py,
                (210.0 * alpha) as u8,
                (200.0 * alpha) as u8,
                (185.0 * alpha) as u8);
        }
    }

    // ── Eye (small dot near head) ──
    let (ex, ey) = koi_spine(0.08, t);
    let ds = 0.01;
    let (ex2, ey2) = koi_spine(0.08 + ds, t);
    let edx = ex2 - ex;
    let edy = ey2 - ey;
    let elen = (edx * edx + edy * edy).sqrt().max(0.001);
    let enx = -edy / elen;

    for eye_side in [-0.6, 0.6] {
        let eye_x = ex + enx * eye_side;
        let eye_y = ey + edx / elen * eye_side;
        let wx = eye_x * cos_h - eye_y * sin_h;
        let wy = eye_x * sin_h + eye_y * cos_h;
        let px = (cx + wx * scale) as i32;
        let py = (cy + wy * scale) as i32;
        canvas.set(px, py, 15, 15, 20);
    }
}

// ─── Koi movement ───────────────────────────────────────────────────────────

fn update_koi(koi: &mut KoiParams, dt: f64, t: f64) {
    koi.turn_timer -= dt;
    if koi.turn_timer <= 0.0 {
        let seed = ((koi.id * 7.3 + t * 3.1).sin() * 10000.0).fract();
        koi.turn_rate = (seed - 0.5) * 1.0;
        let dur = ((koi.id * 11.7 + t * 2.3).cos() * 10000.0).fract();
        koi.turn_timer = 2.0 + dur * 4.0;
    }

    // Wall avoidance
    let margin = 0.2;
    for &(edge, target) in &[
        (koi.x, 0.0f64), (1.0 - koi.x, PI),
        (koi.y, PI * 0.5), (1.0 - koi.y, -PI * 0.5),
    ] {
        if edge < margin {
            let push = ((margin - edge) / margin).powi(2);
            koi.turn_rate += (target - koi.heading).sin() * push * 2.0 * dt;
        }
    }

    koi.turn_rate = koi.turn_rate.clamp(-1.0, 1.0);
    koi.heading += koi.turn_rate * dt;

    let spd = koi.speed * (0.85 + 0.15 * (t * 0.4 + koi.id).sin());
    koi.x += koi.heading.cos() * spd * dt * 0.012;
    koi.y += koi.heading.sin() * spd * dt * 0.024;
    koi.x = koi.x.clamp(0.05, 0.95);
    koi.y = koi.y.clamp(0.05, 0.95);
}

// ─── Pond ───────────────────────────────────────────────────────────────────

fn draw_pond_bg(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64,
) -> (i32, i32, i32, i32) {
    let px = 1i32;
    let py = 2i32;
    let pw = (area.width as i32 - 2).max(20);
    let ph = (area.height as i32 - 3).max(10);

    for y in py..(py + ph) {
        for x in px..(px + pw) {
            let xf = x as f64;
            let yf = y as f64;
            let r1 = ((xf * 0.15 + yf * 0.25 + t * 0.5).sin()
                * (xf * 0.08 - t * 0.3).cos()) * 0.5 + 0.5;

            let r = (8.0 + r1 * 5.0) as u8;
            let g = (14.0 + r1 * 8.0) as u8;
            let b = (24.0 + r1 * 12.0) as u8;

            let cell = &mut buf[(x as u16, y as u16)];
            cell.set_char(' ');
            cell.set_bg(Color::Rgb(r, g, b));
            cell.set_fg(Color::Rgb(r + 8, g + 10, b + 14));
            cell.set_style(Style::default());
        }
    }

    (px, py, pw, ph)
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut koi = KoiParams {
        x: 0.5, y: 0.5, heading: -0.5,
        speed: 1.5, turn_rate: 0.0, turn_timer: 2.0, id: 1.0,
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

            // Background
            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(5, 8, 12));
                    cell.set_fg(Color::Rgb(5, 8, 12));
                }
            }

            let (px, py, pw, ph) = draw_pond_bg(buf, area, elapsed);

            // Draw koi using braille canvas
            let canvas_cw = pw as usize;
            let canvas_ch = ph as usize;
            let mut canvas = Canvas::new(canvas_cw, canvas_ch);

            // Koi position in canvas sub-pixel coords
            let koi_cx = koi.x * (canvas.w as f64);
            let koi_cy = koi.y * (canvas.h as f64);
            let koi_scale = canvas.h as f64 / 10.0; // body is ~10 units

            draw_koi_sdf(&mut canvas, elapsed, koi_cx, koi_cy, koi_scale, koi.heading);

            canvas.render(buf, px as u16, py as u16, area);

            // Header
            if area.height > 2 && area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi (braille)  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
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
