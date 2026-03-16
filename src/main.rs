use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

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
    let off = (id * 1.3).sin() * 0.08;
    (s > 0.03 && s < 0.18 && np.abs() < 0.7)
        || (s > (0.30 + off) && s < (0.55 + off) && np.abs() < 0.8)
        || (s > 0.65 && s < 0.78 && np.abs() < 0.5)
}

fn tangent_at(s: f64, t: f64, turn: f64) -> (f64, f64, f64, f64) {
    let ds = 0.01;
    let (x1, y1) = koi_spine(s, t, turn);
    let (x2, y2) = koi_spine((s + ds).min(1.0), t, turn);
    let dx = x2 - x1;
    let dy = y2 - y1;
    let l = (dx * dx + dy * dy).sqrt().max(0.001);
    (-dy / l, dx / l, x1, y1)
}

// ─── Koi rendering (uses bg color for solid fill) ───────────────────────────

fn draw_koi(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
    koi: &Koi,
) {
    let cos_h = koi.heading.cos();
    let sin_h = koi.heading.sin();
    let sx = 2.5f64; // scale x (chars per unit)
    let sy = 1.2f64; // scale y

    let cx = koi.x * area.width as f64;
    let cy = koi.y * area.height as f64;

    let transform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((cx + wx * sx) as i32, (cy + wy * sy) as i32)
    };

    // Shadow (slightly offset, darker)
    for si in 0..40 {
        let s = si as f64 / 40.0;
        let hw = koi_width(s) * 0.9;
        let (nx, ny, px, py) = tangent_at(s, t, koi.turn_rate);
        let steps = (hw * sx.max(sy) * 2.0) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let lx = px + nx * p;
            let ly = py + ny * p;
            let (scx, scy) = transform(lx, ly);
            let scx = scx + 1;
            let scy = scy + 1;
            if scx >= 0 && scy >= 1 && scx < area.width as i32 && scy < area.height as i32 {
                let cell = &mut buf[(scx as u16, scy as u16)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(5, 9, 16));
            }
        }
    }

    // Body (filled with bg color — solid colored cells)
    for si in 0..50 {
        let s = si as f64 / 50.0;
        let hw = koi_width(s);
        let (nx, ny, px, py) = tangent_at(s, t, koi.turn_rate);

        let steps = (hw * sx.max(sy) * 2.5) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let norm_p = p / hw;

            let lx = px + nx * p;
            let ly = py + ny * p;
            let (scx, scy) = transform(lx, ly);

            if scx < 0 || scy < 1 || scx >= area.width as i32 || scy >= area.height as i32 { continue; }

            let edge_dist = 1.0 - norm_p.abs();
            let is_outline = edge_dist < 0.12;
            let is_red = koi_red(s, norm_p, koi.id);

            let (ch, fg, bg) = if is_outline {
                // Dark outline ring
                (' ', Color::Rgb(0, 0, 0), Color::Rgb(60, 55, 45))
            } else if is_red {
                // Red patch — use █ fg for solid red
                ('█', Color::Rgb(210, 55, 35), Color::Rgb(210, 55, 35))
            } else {
                // White body
                ('█', Color::Rgb(235, 230, 218), Color::Rgb(235, 230, 218))
            };

            let cell = &mut buf[(scx as u16, scy as u16)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_bg(bg);
            cell.set_style(Style::default());
        }
    }

    // Pectoral fins (semi-transparent — use lighter bg)
    let fin_flap = (t * 2.5).sin() * 0.2;
    for side in [-1.0f64, 1.0] {
        let (fnx, fny, fpx, fpy) = tangent_at(0.22, t, koi.turn_rate);
        let (_, _, fpx2, fpy2) = tangent_at(0.23, t, koi.turn_rate);
        let tdx = fpx2 - fpx;
        let tdy = fpy2 - fpy;
        let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

        for fi in 0..10 {
            let ft = fi as f64 / 10.0;
            let spread = side * (1.0 + ft * 0.8 + fin_flap);
            let along = -ft * 1.2;
            let fx = fpx + fnx * spread + tdx / tl * along;
            let fy = fpy + fny * spread + tdy / tl * along;
            let (scx, scy) = transform(fx, fy);
            if scx >= 0 && scy >= 1 && scx < area.width as i32 && scy < area.height as i32 {
                let a = 1.0 - ft;
                let cell = &mut buf[(scx as u16, scy as u16)];
                // Only draw fin if cell is still water (don't overwrite body)
                if cell.bg == Color::Rgb(5, 9, 16) || cell.symbol() == " " {
                    cell.set_char('░');
                    cell.set_fg(Color::Rgb((180.0 * a) as u8, (170.0 * a) as u8, (155.0 * a) as u8));
                }
            }
        }
    }

    // Tail fin
    let tail_sway = (t * 2.0).sin() * 0.3;
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..14 {
            let ft = ti as f64 / 14.0;
            let s_pos = (0.88 + ft * 0.12).min(0.99);
            let (tnx, tny, tpx, tpy) = tangent_at(s_pos, t, koi.turn_rate);
            let spread = lobe * (0.2 + ft * 1.5 + tail_sway * ft);
            let tx = tpx + tnx * spread;
            let ty = tpy + tny * spread;
            let (scx, scy) = transform(tx, ty);
            if scx >= 0 && scy >= 1 && scx < area.width as i32 && scy < area.height as i32 {
                let a = 1.0 - ft * 0.4;
                let cell = &mut buf[(scx as u16, scy as u16)];
                cell.set_char('░');
                cell.set_fg(Color::Rgb((190.0 * a) as u8, (180.0 * a) as u8, (165.0 * a) as u8));
            }
        }
    }

    // Eyes
    for eye_side in [-0.35f64, 0.35] {
        let (enx, eny, epx, epy) = tangent_at(0.06, t, koi.turn_rate);
        let ex = epx + enx * eye_side;
        let ey = epy + eny * eye_side;
        let (scx, scy) = transform(ex, ey);
        if scx >= 0 && scy >= 1 && scx < area.width as i32 && scy < area.height as i32 {
            let cell = &mut buf[(scx as u16, scy as u16)];
            cell.set_char('●');
            cell.set_fg(Color::Rgb(10, 10, 15));
            cell.set_bg(Color::Rgb(235, 230, 218));
            cell.set_style(Style::default());
        }
    }
}

// ─── Koi state ──────────────────────────────────────────────────────────────

struct Koi {
    x: f64, y: f64, heading: f64,
    speed: f64, turn_rate: f64, turn_timer: f64, id: f64,
}

fn update_koi(k: &mut Koi, dt: f64, t: f64) {
    k.turn_timer -= dt;
    if k.turn_timer <= 0.0 {
        let s1 = ((k.id * 7.3 + t * 3.1).sin() * 1e4).fract();
        let s2 = ((k.id * 11.7 + t * 2.3).cos() * 1e4).fract();
        k.turn_rate = if s1 > 0.75 { (s1 - 0.5) * 2.0 }
            else if s1 < 0.25 { (s1 - 0.5) * 2.0 }
            else { (s1 - 0.5) * 0.3 };
        k.turn_timer = 1.5 + s2 * 4.0;
    }

    let margin = 0.22;
    if k.x < margin || k.x > 1.0 - margin || k.y < margin || k.y > 1.0 - margin {
        let to_center = (0.5 - k.y).atan2(0.5 - k.x);
        let diff = (to_center - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.turn_rate += diff * 0.5 * dt;
    }

    k.turn_rate = k.turn_rate.clamp(-0.8, 0.8);
    k.heading += k.turn_rate * dt;

    let spd = k.speed * (0.8 + 0.2 * (t * 0.3 + k.id).sin());
    k.x += k.heading.cos() * spd * dt * 0.008;
    k.y += k.heading.sin() * spd * dt * 0.015;
    k.x = k.x.clamp(0.1, 0.9);
    k.y = k.y.clamp(0.1, 0.9);
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut fish = vec![
        Koi { x: 0.35, y: 0.4, heading: 0.3, speed: 2.0, turn_rate: 0.0, turn_timer: 2.0, id: 1.0 },
        Koi { x: 0.65, y: 0.6, heading: 2.8, speed: 1.6, turn_rate: 0.2, turn_timer: 1.5, id: 3.2 },
        Koi { x: 0.5, y: 0.35, heading: 4.5, speed: 1.4, turn_rate: -0.1, turn_timer: 3.0, id: 5.7 },
    ];

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        for k in &mut fish { update_koi(k, dt, elapsed); }

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Water
            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64;
                    let yf = y as f64;
                    let r = ((xf * 0.1 + yf * 0.17 + elapsed * 0.35).sin()
                        * (xf * 0.06 - elapsed * 0.2).cos()) * 0.5 + 0.5;
                    let br = (10.0 + r * 5.0) as u8;
                    let bg = (18.0 + r * 8.0) as u8;
                    let bb = (32.0 + r * 12.0) as u8;

                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(br, bg, bb));
                    cell.set_fg(Color::Rgb(br, bg, bb));
                }
            }

            for k in &fish { draw_koi(buf, area, elapsed, k); }

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
