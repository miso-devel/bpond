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
    w: usize, h: usize, cw: usize,
    px: Vec<(bool, u8, u8, u8)>,
}

impl Canvas {
    fn new(cw: usize, ch: usize) -> Self {
        Canvas { w: cw * 2, h: ch * 4, cw, px: vec![(false, 0, 0, 0); cw * 2 * ch * 4] }
    }
    fn dot(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }
    fn fat(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in 0..2 { for dx in 0..2 { self.dot(x + dx, y + dy, r, g, b); } }
    }
    fn thick(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in -1..=1 { for dx in -1..=1 { self.dot(x + dx, y + dy, r, g, b); } }
    }
    fn render(&self, buf: &mut ratatui::buffer::Buffer, ox: u16, oy: u16, area: ratatui::layout::Rect) {
        let ch = self.h / 4;
        for cy in 0..ch {
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
                let bx = ox as i32 + cx as i32;
                let by = oy as i32 + cy as i32;
                if bx < 0 || by < 0 || bx >= area.width as i32 || by >= area.height as i32 { continue; }
                let cell = &mut buf[(bx as u16, by as u16)];
                cell.set_char(char::from_u32(BRAILLE_BASE + bits).unwrap_or(' '));
                cell.set_fg(Color::Rgb((tr / n) as u8, (tg / n) as u8, (tb / n) as u8));
                cell.set_style(Style::default());
            }
        }
    }
}

// ─── Koi spine chain ────────────────────────────────────────────────────────

const N_SPINE: usize = 40;
const SEG_LEN: f64 = 0.55;
const FREQ: f64 = 1.2;
const BODY_TOTAL: f64 = N_SPINE as f64 * SEG_LEN;

fn body_width(s: f64) -> f64 {
    let frac = if s < 0.05 { s / 0.05 * 0.10 }
        else if s < 0.15 { 0.10 + (s - 0.05) / 0.10 * 0.08 }
        else if s < 0.40 { 0.18 }
        else if s < 0.75 { 0.18 - (s - 0.4) / 0.35 * 0.08 }
        else { 0.10 * (1.0 - s) / 0.25 };
    frac * BODY_TOTAL
}

struct Koi {
    spine_x: [f64; N_SPINE],
    spine_y: [f64; N_SPINE],
    heading: f64,
    speed: f64,
    turn_rate: f64,
    target_turn: f64,
    turn_timer: f64,
    id: f64,
    red_mask: [bool; N_SPINE],
}

impl Koi {
    fn new(x: f64, y: f64, heading: f64, speed: f64, id: f64) -> Self {
        let mut koi = Koi {
            spine_x: [0.0; N_SPINE],
            spine_y: [0.0; N_SPINE],
            heading, speed,
            turn_rate: 0.0, target_turn: 0.0,
            turn_timer: 2.0 + id, id,
            red_mask: [false; N_SPINE],
        };
        for i in 0..N_SPINE {
            koi.spine_x[i] = x - (i as f64) * SEG_LEN * heading.cos();
            koi.spine_y[i] = y - (i as f64) * SEG_LEN * heading.sin();
        }
        let n_patches = 2 + ((id * 3.7).sin().abs() * 3.5) as usize;
        for p in 0..n_patches {
            let center = ((id * (p as f64 + 1.0) * 2.3 + 0.7).sin().abs() * 0.7 + 0.08) * N_SPINE as f64;
            let half_w = ((id * (p as f64 + 1.0) * 1.7 + 1.3).cos().abs() * 0.12 + 0.04) * N_SPINE as f64;
            for i in 0..N_SPINE {
                if (i as f64 - center).abs() < half_w {
                    koi.red_mask[i] = true;
                }
            }
        }
        koi
    }
}

fn update_koi(k: &mut Koi, dt: f64, t: f64, w: f64, h: f64) {
    k.turn_timer -= dt;
    if k.turn_timer <= 0.0 {
        let s1 = ((k.id * 7.3 + t * 3.1).sin() * 1e4).fract();
        let s2 = ((k.id * 11.7 + t * 2.3).cos() * 1e4).fract();
        k.target_turn = if s1 > 0.92 { 0.4 }
            else if s1 < 0.08 { -0.4 }
            else if s1 > 0.75 { 0.15 }
            else if s1 < 0.25 { -0.15 }
            else { 0.0 };
        k.turn_timer = 2.0 + s2 * 5.0;
    }

    let approach = 0.6 * dt;
    if (k.target_turn - k.turn_rate).abs() < approach {
        k.turn_rate = k.target_turn;
    } else if k.target_turn > k.turn_rate {
        k.turn_rate += approach;
    } else {
        k.turn_rate -= approach;
    }
    k.turn_rate = k.turn_rate.clamp(-0.45, 0.45);

    let swim_wave = (t * 2.0 * PI * FREQ).sin() * 0.10;
    k.heading += (k.turn_rate + swim_wave) * dt;

    let margin = 5.0;
    let fully_out = k.spine_x[0] < -margin || k.spine_x[0] > w + margin
        || k.spine_y[0] < -margin || k.spine_y[0] > h + margin;
    if fully_out {
        let toward = (h / 2.0 - k.spine_y[0]).atan2(w / 2.0 - k.spine_x[0]);
        let diff = (toward - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.heading += diff * 0.3 * dt;
    }

    let burst = if (t * 0.1 + k.id).sin() > 0.97 { 1.5 } else { 1.0 };
    k.spine_x[0] += k.heading.cos() * k.speed * burst * dt;
    k.spine_y[0] += k.heading.sin() * k.speed * burst * dt;

    for i in 1..N_SPINE {
        let dx = k.spine_x[i - 1] - k.spine_x[i];
        let dy = k.spine_y[i - 1] - k.spine_y[i];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > SEG_LEN {
            let ratio = SEG_LEN / dist;
            k.spine_x[i] = k.spine_x[i - 1] - dx * ratio;
            k.spine_y[i] = k.spine_y[i - 1] - dy * ratio;
        }
    }
}

// ─── Drawing ────────────────────────────────────────────────────────────────

fn draw_koi(canvas: &mut Canvas, t: f64, koi: &Koi, scale: f64, off_x: f64, off_y: f64) {
    let freq = FREQ;

    // World → sub-pixel. Use UNIFORM scale so shape doesn't distort with heading.
    let to_px = |wx: f64, wy: f64| -> (i32, i32) {
        ((wx * scale + off_x) as i32, (wy * scale + off_y) as i32)
    };

    let tangent_at = |i: usize| -> (f64, f64) {
        let i2 = (i + 1).min(N_SPINE - 1);
        let i1 = i.saturating_sub(1);
        let dx = koi.spine_x[i1] - koi.spine_x[i2];
        let dy = koi.spine_y[i1] - koi.spine_y[i2];
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        (dx / l, dy / l)
    };

    let normal_at = |i: usize| -> (f64, f64) {
        let (tx, ty) = tangent_at(i);
        (-ty, tx)
    };

    // Shadow
    for i in (0..N_SPINE).step_by(2) {
        let s = i as f64 / N_SPINE as f64;
        let hw = body_width(s) * 0.7;
        let (nx, ny) = normal_at(i);
        let steps = (hw * scale * 1.2) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let (px, py) = to_px(koi.spine_x[i] + nx * p, koi.spine_y[i] + ny * p);
            canvas.dot(px + 3, py + 5, 3, 6, 12);
        }
    }

    // Tail fin — fixed fan shape, rides on the spine's natural sway.
    // No pitch oscillation in spread (that caused "stretch/shrink").
    // The spine chain already swings the tail left/right via swim_wave.
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..20 {
            let ft = ti as f64 / 20.0;
            let idx = (N_SPINE - 7 + (ft * 6.0) as usize).min(N_SPINE - 1);
            let (nx, ny) = normal_at(idx);
            // Constant fan spread — widens from root to tip
            let spread = lobe * (0.3 + ft * 2.8);
            let (px, py) = to_px(koi.spine_x[idx] + nx * spread, koi.spine_y[idx] + ny * spread);
            let a = (1.0 - ft * 0.3) * 0.55;
            canvas.thick(px, py, (225.0 * a) as u8, (215.0 * a) as u8, (195.0 * a) as u8);
        }
    }

    // Pectoral fins (angle-based: rest angle + oscillation, left/right alternate)
    let pec_idx = (N_SPINE as f64 * 0.2) as usize;
    let pec_rest = 15.0f64.to_radians();
    let pec_amp = 30.0f64.to_radians();
    if pec_idx < N_SPINE {
        let (nx, ny) = normal_at(pec_idx);
        let (tx, ty) = tangent_at(pec_idx);
        for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
            let phase = if is_left { 0.0 } else { PI };
            let angle = pec_rest + pec_amp * (2.0 * PI * freq * t + phase).sin();
            let fin_len = BODY_TOTAL * 0.12;
            for fi in 0..12 {
                let ft = fi as f64 / 12.0;
                let spread = side * (angle.sin() * (1.0 - ft * 0.5)) * 1.5;
                let along = -ft * fin_len;
                let wx = koi.spine_x[pec_idx] + nx * spread + tx * along;
                let wy = koi.spine_y[pec_idx] + ny * spread + ty * along;
                let (px, py) = to_px(wx, wy);
                let a = (1.0 - ft) * 0.5;
                canvas.thick(px, py, (210.0 * a) as u8, (200.0 * a) as u8, (182.0 * a) as u8);
            }
        }
    }

    // Pelvic fins (same angle-based, smaller, at ~45%)
    let pel_idx = (N_SPINE as f64 * 0.45) as usize;
    if pel_idx < N_SPINE {
        let (nx, ny) = normal_at(pel_idx);
        let (tx, ty) = tangent_at(pel_idx);
        for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
            let phase = if is_left { 0.5 } else { PI + 0.5 };
            let angle = 10.0f64.to_radians() + 20.0f64.to_radians() * (2.0 * PI * freq * t + phase).sin();
            let fin_len = BODY_TOTAL * 0.08;
            for fi in 0..8 {
                let ft = fi as f64 / 8.0;
                let spread = side * (angle.sin() * (1.0 - ft * 0.5)) * 1.0;
                let along = -ft * fin_len;
                let wx = koi.spine_x[pel_idx] + nx * spread + tx * along;
                let wy = koi.spine_y[pel_idx] + ny * spread + ty * along;
                let (px, py) = to_px(wx, wy);
                let a = (1.0 - ft) * 0.45;
                canvas.thick(px, py, (210.0 * a) as u8, (200.0 * a) as u8, (182.0 * a) as u8);
            }
        }
    }

    // Body
    for i in 0..N_SPINE {
        let s = i as f64 / N_SPINE as f64;
        let hw = body_width(s);
        let (nx, ny) = normal_at(i);
        let steps = (hw * scale * 2.0) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let np = (p / hw).abs();
            let (px, py) = to_px(koi.spine_x[i] + nx * p, koi.spine_y[i] + ny * p);
            let outline = np > 0.78;
            let is_red = koi.red_mask[i] && np < 0.72;
            let (r, g, b) = if outline { (30, 25, 18) }
                else if is_red { (235, 45, 25) }
                else { (255, 252, 242) };
            canvas.fat(px, py, r, g, b);
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
    let w = tw as f64;
    let h = th as f64;
    let mut fish = vec![
        Koi::new(w * 0.3, h * 0.35, 0.3, 5.5, 1.0),
        Koi::new(w * 0.7, h * 0.6, 3.5, 5.0, 4.3),
        Koi::new(w * 0.5, h * 0.25, 1.8, 4.5, 7.1),
        Koi::new(w * 0.4, h * 0.7, 5.2, 5.2, 11.5),
    ];

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
        for k in &mut fish { update_koi(k, dt, elapsed, tw as f64, th as f64); }

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64; let yf = y as f64;
                    let r = ((xf * 0.08 + yf * 0.14 + elapsed * 0.2).sin()
                        * (xf * 0.05 - elapsed * 0.12).cos()) * 0.5 + 0.5;
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(
                        (10.0 + r * 4.0) as u8, (18.0 + r * 6.0) as u8, (32.0 + r * 9.0) as u8,
                    ));
                    cell.set_fg(Color::Rgb(10, 18, 32));
                }
            }

            let cw = area.width as usize;
            let ch = (area.height as usize).saturating_sub(1);
            if cw < 4 || ch < 4 { return; }
            let mut canvas = Canvas::new(cw, ch);

            // UNIFORM scale: use the same scale for x and y so the fish
            // doesn't change size when it turns. Braille's 2:4 sub-pixel
            // ratio handles the terminal's character aspect ratio.
            let scale = (canvas.h as f64 / (th as f64)).min(canvas.w as f64 / (tw as f64));

            // Offset so world origin (0,0) maps to canvas origin
            // World coords are in terminal cells, canvas is in sub-pixels
            let off_x = 0.0;
            let off_y = 0.0;

            for k in &fish {
                draw_koi(&mut canvas, elapsed, k, scale, off_x, off_y);
            }
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
