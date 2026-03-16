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

const BODY_LEN: f64 = 5.0;
const FREQ: f64 = 1.2;

fn amplitude(x: f64) -> f64 {
    BODY_LEN * (0.02 - 0.08 * x + 0.16 * x * x)
}

fn midline(s: f64, t: f64, turn: f64, freq: f64) -> f64 {
    let a = amplitude(s);
    let k = 2.0 * PI / BODY_LEN;
    let omega = 2.0 * PI * freq;
    let wave = a * (k * s * BODY_LEN - omega * t).sin();

    // Turn: HEAD leads, tail follows. Curvature concentrated at FRONT.
    // (1 - s)^1.5 means head (s=0) bends most, tail (s=1) barely bends
    let turn_curve = turn * 0.25 * BODY_LEN * (1.0 - s).powf(1.5);

    let asym = if (wave > 0.0 && turn > 0.0) || (wave < 0.0 && turn < 0.0) {
        1.0 + 0.2 * turn.abs().min(1.0)
    } else {
        1.0 - 0.2 * turn.abs().min(1.0)
    };

    turn_curve + wave * asym
}

fn body_width(s: f64) -> f64 {
    if s < 0.05 { s / 0.05 * 0.5 }
    else if s < 0.2 { 0.5 + (s - 0.05) / 0.15 * 0.5 }
    else if s < 0.4 { 1.0 }
    else if s < 0.75 { 1.0 - (s - 0.4) / 0.35 * 0.45 }
    else { 0.55 * (1.0 - s) / 0.25 }
}

// Each koi has a unique pattern based on its id
// Returns: 0 = base color, 1 = accent color, 2 = second accent (for sanke/showa)
fn pattern(s: f64, np: f64, id: f64) -> u8 {
    let seed = (id * 2.71).sin().abs();
    let off1 = (id * 1.3).sin() * 0.08;
    let off2 = (id * 3.7).cos() * 0.06;

    let variant = (id * 1.9).sin().abs(); // 0..1 determines pattern style

    if variant < 0.25 {
        // Kohaku: 2 large patches + head cap
        let p1 = s > 0.03 && s < (0.16 + off1) && np.abs() < 0.5;
        let p2 = s > (0.32 + off1) && s < (0.58 + off2) && np.abs() < 0.75;
        if p1 || p2 { 1 } else { 0 }
    } else if variant < 0.5 {
        // Tancho: single round spot on head only
        let spot = (s - 0.08).powi(2) + (np * 0.5).powi(2) < 0.008 + seed * 0.003;
        if spot { 1 } else { 0 }
    } else if variant < 0.75 {
        // Sanke: red patches + small black (sumi) dots
        let p1 = s > (0.05 + off1) && s < (0.22 + off1) && np.abs() < 0.6;
        let p2 = s > (0.40 + off2) && s < (0.62 + off2) && np.abs() < 0.65;
        let sumi = ((s * 17.0 + seed * 10.0).sin() > 0.85) && (np.abs() > 0.3);
        if sumi { 2 } else if p1 || p2 { 1 } else { 0 }
    } else {
        // Showa: mostly dark with red and white patches
        let white = (s > (0.15 + off1) && s < (0.30 + off1) && np.abs() < 0.55)
            || (s > (0.55 + off2) && s < (0.72 + off2) && np.abs() < 0.5);
        let red = s > (0.35 + off1) && s < (0.52 + off2) && np.abs() < 0.7;
        if red { 1 } else if white { 0 } else { 2 }
    }
}

fn spine_at(s: f64, t: f64, turn: f64, freq: f64) -> (f64, f64) {
    ((1.0 - s) * BODY_LEN, midline(s, t, turn, freq))
}

fn tgt(s: f64, t: f64, turn: f64, freq: f64) -> (f64, f64, f64, f64) {
    let ds = 0.005;
    let (x1, y1) = spine_at(s, t, turn, freq);
    let (x2, y2) = spine_at((s + ds).min(1.0), t, turn, freq);
    let dx = x2 - x1; let dy = y2 - y1;
    let l = (dx * dx + dy * dy).sqrt().max(0.001);
    (-dy / l, dx / l, x1, y1)
}

fn draw_fin(canvas: &mut Canvas, s_pos: f64, t: f64, turn: f64, freq: f64,
    heading: f64, cx: f64, cy: f64, sx: f64, sy: f64,
    side: f64, fin_len: f64, fin_width: f64, phase: f64, amp: f64,
    color: (u8, u8, u8))
{
    let cos_h = heading.cos(); let sin_h = heading.sin();
    let angle = amp * (2.0 * PI * freq * t + phase).sin();
    let (fnx, fny, fpx, fpy) = tgt(s_pos, t, turn, freq);
    let (_, _, fpx2, fpy2) = tgt(s_pos + 0.02, t, turn, freq);
    let tdx = fpx2 - fpx; let tdy = fpy2 - fpy;
    let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

    for fi in 0..12 {
        let ft = fi as f64 / 12.0;
        let spread = side * (angle.sin().abs() * 0.3 + 0.6) * (1.0 - ft * 0.4) * fin_width;
        let along = -ft * fin_len;
        let fx = fpx + fnx * spread + tdx / tl * along;
        let fy = fpy + fny * spread + tdy / tl * along;
        let wx = fx * cos_h - fy * sin_h;
        let wy = fx * sin_h + fy * cos_h;
        let px = (cx + wx * sx) as i32;
        let py = (cy + wy * sy) as i32;
        let a = 1.0 - ft * 0.5;
        canvas.thick(px, py,
            (color.0 as f64 * a) as u8,
            (color.1 as f64 * a) as u8,
            (color.2 as f64 * a) as u8);
    }
}

fn draw_koi(canvas: &mut Canvas, t: f64, cx: f64, cy: f64, sx: f64, sy: f64, koi: &Koi) {
    let cos_h = koi.heading.cos();
    let sin_h = koi.heading.sin();
    let freq = FREQ;

    let xform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((cx + wx * sx) as i32, (cy + wy * sy) as i32)
    };

    // Shadow
    for si in 0..40 {
        let s = si as f64 / 40.0;
        let hw = body_width(s) * 0.75;
        let (nx, ny, px, py) = tgt(s, t, koi.turn_rate, freq);
        let steps = (hw * sx.max(sy) * 1.5) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let (dx, dy) = xform(px + nx * p, py + ny * p);
            canvas.dot(dx + 3, dy + 5, 3, 6, 12);
        }
    }

    // Tail fin
    let tail_pitch = (2.0 * PI * freq * t).cos() * 0.25;
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..14 {
            let ft = ti as f64 / 14.0;
            let s_pos = (0.86 + ft * 0.14).min(0.99);
            let (tnx, tny, tpx, tpy) = tgt(s_pos, t, koi.turn_rate, freq);
            let spread = lobe * (0.08 + ft * 1.0 + tail_pitch * ft);
            let (dx, dy) = xform(tpx + tnx * spread, tpy + tny * spread);
            let a = (1.0 - ft * 0.3) * 0.55;
            canvas.thick(dx, dy, (225.0 * a) as u8, (215.0 * a) as u8, (195.0 * a) as u8);
        }
    }

    // Pectoral fins (s=0.22)
    let fin_c = (210, 200, 180);
    for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
        let ph = if is_left { 0.0 } else { PI };
        draw_fin(canvas, 0.22, t, koi.turn_rate, freq, koi.heading, cx, cy, sx, sy,
            side, BODY_LEN * 0.10, 1.3, ph, 0.4, fin_c);
    }

    // Pelvic (ventral) fins (s=0.45)
    for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
        let ph = if is_left { 0.5 } else { PI + 0.5 };
        draw_fin(canvas, 0.45, t, koi.turn_rate, freq, koi.heading, cx, cy, sx, sy,
            side, BODY_LEN * 0.07, 1.0, ph, 0.3, fin_c);
    }

    // Body (drawn LAST so it's on top of fins)
    for si in 0..60 {
        let s = si as f64 / 60.0;
        let hw = body_width(s);
        let (nx, ny, px, py) = tgt(s, t, koi.turn_rate, freq);
        let steps = (hw * sx.max(sy) * 2.0) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let np = (p / hw).abs();
            let (dx, dy) = xform(px + nx * p, py + ny * p);
            let outline = np > 0.80;
            let pat = pattern(s, p / hw, koi.id);
            let (r, g, b) = if outline { (30, 25, 18) }
                else if pat == 1 { koi.accent1 }
                else if pat == 2 { koi.accent2 }
                else { koi.base_color };
            canvas.fat(dx, dy, r, g, b);
        }
    }
}

// ─── Koi state ──────────────────────────────────────────────────────────────

struct Koi {
    x: f64, y: f64, heading: f64,
    speed: f64, turn_rate: f64, target_turn: f64,
    turn_timer: f64, id: f64,
    base_color: (u8, u8, u8),   // body white/gold
    accent1: (u8, u8, u8),      // red/orange patches
    accent2: (u8, u8, u8),      // black/dark sumi marks
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
    k.heading += k.turn_rate * dt;

    // Only steer back when FULLY off-screen (no bias while visible)
    let margin = 5.0; // cells past the edge before steering kicks in
    let fully_out = k.x < -margin || k.x > w + margin
        || k.y < -margin || k.y > h + margin;
    if fully_out {
        let cx = w / 2.0;
        let cy = h / 2.0;
        let toward = (cy - k.y).atan2(cx - k.x);
        let diff = (toward - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.heading += diff * 0.3 * dt;
    }

    let burst = if (t * 0.1 + k.id).sin() > 0.97 { 1.5 } else { 1.0 };
    k.x += k.heading.cos() * k.speed * burst * dt;
    k.y += k.heading.sin() * k.speed * burst * dt;
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
    let mut fish = vec![
        // Kohaku (red on white)
        Koi { x: tw as f64 * 0.3, y: th as f64 * 0.35, heading: 0.3,
              speed: 3.5, turn_rate: 0.0, target_turn: 0.0, turn_timer: 3.0, id: 1.0,
              base_color: (255, 252, 242), accent1: (235, 45, 25), accent2: (30, 25, 18) },
        // Ogon (gold body, orange accent)
        Koi { x: tw as f64 * 0.7, y: th as f64 * 0.6, heading: 3.5,
              speed: 3.0, turn_rate: 0.0, target_turn: 0.0, turn_timer: 2.0, id: 4.3,
              base_color: (245, 210, 80), accent1: (230, 130, 30), accent2: (100, 70, 20) },
        // Sanke (white, red, black spots)
        Koi { x: tw as f64 * 0.5, y: th as f64 * 0.25, heading: 1.8,
              speed: 2.8, turn_rate: 0.0, target_turn: 0.0, turn_timer: 4.0, id: 7.1,
              base_color: (255, 250, 240), accent1: (220, 50, 35), accent2: (25, 22, 20) },
        // Showa (black base, red/white patches)
        Koi { x: tw as f64 * 0.4, y: th as f64 * 0.7, heading: 5.2,
              speed: 3.2, turn_rate: 0.0, target_turn: 0.0, turn_timer: 1.5, id: 9.8,
              base_color: (250, 248, 238), accent1: (225, 40, 30), accent2: (20, 18, 15) },
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

            // Separate x/y scales for correct aspect ratio at any terminal size
            // Terminal chars are ~2x taller than wide, braille sub-pixels compensate:
            //   horizontal: 2 sub-px per char
            //   vertical: 4 sub-px per char
            // So 1 unit should be: scale_x sub-px horizontally, scale_y sub-px vertically
            let scale_x = canvas.w as f64 / 14.0;  // koi body ~5 units, screen ~14 units wide
            let scale_y = canvas.h as f64 / 14.0;

            for k in &fish {
                let kcx = k.x / area.width as f64 * canvas.w as f64;
                let kcy = k.y / area.height as f64 * canvas.h as f64;
                draw_koi(&mut canvas, elapsed, kcx, kcy, scale_x, scale_y, k);
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
