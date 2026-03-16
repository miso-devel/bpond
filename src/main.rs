use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Koi SDF (no braille — render directly to character cells) ──────────────

// Spine: S-curve that incorporates the current turn rate for dynamic bending
fn koi_spine(s: f64, t: f64, turn: f64) -> (f64, f64) {
    // Swimming undulation: travels head→tail
    let swim_wave = (s * 2.5 - t * 2.2).sin() * s * s * 0.3;
    // Turn-induced body bend: whole body curves when turning
    // The body arcs proportionally to the turn rate
    let turn_bend = turn * s * s * 2.0;

    let x = (1.0 - s) * 5.0;
    let y = swim_wave + turn_bend;
    (x, y)
}

fn koi_width(s: f64) -> f64 {
    if s < 0.05 { s / 0.05 * 0.6 }
    else if s < 0.2 { 0.6 + (s - 0.05) / 0.15 * 0.5 }
    else if s < 0.4 { 1.1 }
    else if s < 0.75 { 1.1 - (s - 0.4) / 0.35 * 0.5 }
    else { 0.6 * (1.0 - s) / 0.25 }
}

fn koi_red(s: f64, norm_p: f64, id: f64) -> bool {
    let off = (id * 1.3).sin() * 0.08;
    let p1 = s > 0.03 && s < 0.18 && norm_p.abs() < 0.7;
    let p2 = s > (0.30 + off) && s < (0.55 + off) && norm_p.abs() < 0.8;
    let p3 = s > 0.65 && s < 0.78 && norm_p.abs() < 0.5;
    p1 || p2 || p3
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

fn draw_koi(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
    koi: &Koi,
) {
    let cos_h = koi.heading.cos();
    let sin_h = koi.heading.sin();
    let scale_x = 2.5; // chars per unit (horizontal — chars are narrow)
    let scale_y = 1.2; // chars per unit (vertical — chars are tall)

    let screen_cx = koi.x * area.width as f64;
    let screen_cy = koi.y * area.height as f64;

    let transform = |lx: f64, ly: f64| -> (i32, i32) {
        let wx = lx * cos_h - ly * sin_h;
        let wy = lx * sin_h + ly * cos_h;
        ((screen_cx + wx * scale_x) as i32, (screen_cy + wy * scale_y) as i32)
    };

    // Body
    for si in 0..50 {
        let s = si as f64 / 50.0;
        let hw = koi_width(s);
        let (nx, ny, px, py) = tangent_at(s, t, koi.turn_rate);

        let steps = (hw * scale_x.max(scale_y) * 2.5) as i32 + 1;
        for pi in -steps..=steps {
            let p = pi as f64 / (steps as f64 / hw);
            if p.abs() > hw { continue; }
            let norm_p = p / hw;

            let lx = px + nx * p;
            let ly = py + ny * p;
            let (sx, sy) = transform(lx, ly);

            if sx < 0 || sy < 1 || sx >= area.width as i32 || sy >= area.height as i32 { continue; }

            let edge = 1.0 - norm_p.abs().powi(2);
            let is_red = koi_red(s, norm_p, koi.id);

            let (ch, r, g, b) = if edge < 0.15 {
                // Outline
                ('.', 120.0 * edge * 3.0, 115.0 * edge * 3.0, 105.0 * edge * 3.0)
            } else if is_red {
                let ch = if edge > 0.7 { '#' } else { '=' };
                (ch, 210.0 * edge, 55.0 * edge, 40.0 * edge)
            } else {
                let ch = if edge > 0.7 { '#' } else if edge > 0.4 { '=' } else { '-' };
                (ch, 235.0 * edge, 230.0 * edge, 220.0 * edge)
            };

            let cell = &mut buf[(sx as u16, sy as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(r as u8, g as u8, b as u8));
            cell.set_style(Style::default());
        }
    }

    // Pectoral fins
    let fin_flap = (t * 2.5).sin() * 0.2;
    for side in [-1.0f64, 1.0] {
        let (fnx, fny, fpx, fpy) = tangent_at(0.22, t, koi.turn_rate);
        let (_, _, fpx2, fpy2) = tangent_at(0.23, t, koi.turn_rate);
        let tdx = fpx2 - fpx;
        let tdy = fpy2 - fpy;
        let tl = (tdx * tdx + tdy * tdy).sqrt().max(0.001);

        for fi in 0..8 {
            let ft = fi as f64 / 8.0;
            let spread = side * (1.0 + ft * 0.8 + fin_flap);
            let along = -ft * 1.2;
            let fx = fpx + fnx * spread + tdx / tl * along;
            let fy = fpy + fny * spread + tdy / tl * along;
            let (sx, sy) = transform(fx, fy);
            if sx >= 0 && sy >= 1 && sx < area.width as i32 && sy < area.height as i32 {
                let a = (1.0 - ft) * 0.5;
                let cell = &mut buf[(sx as u16, sy as u16)];
                cell.set_char(',');
                cell.set_fg(Color::Rgb((180.0 * a) as u8, (175.0 * a) as u8, (165.0 * a) as u8));
                cell.set_style(Style::default());
            }
        }
    }

    // Tail fin
    let tail_sway = (t * 2.0).sin() * 0.3;
    for lobe in [-1.0f64, 1.0] {
        for ti in 0..12 {
            let ft = ti as f64 / 12.0;
            let s_pos = (0.88 + ft * 0.12).min(0.99);
            let (tnx, tny, tpx, tpy) = tangent_at(s_pos, t, koi.turn_rate);
            let spread = lobe * (0.2 + ft * 1.4 + tail_sway * ft);
            let tx = tpx + tnx * spread;
            let ty = tpy + tny * spread;
            let (sx, sy) = transform(tx, ty);
            if sx >= 0 && sy >= 1 && sx < area.width as i32 && sy < area.height as i32 {
                let a = (1.0 - ft * 0.3) * 0.5;
                let cell = &mut buf[(sx as u16, sy as u16)];
                cell.set_char(if ft < 0.5 { '~' } else { '.' });
                cell.set_fg(Color::Rgb((195.0 * a) as u8, (185.0 * a) as u8, (170.0 * a) as u8));
                cell.set_style(Style::default());
            }
        }
    }

    // Eyes
    for eye_side in [-0.35f64, 0.35] {
        let (enx, eny, epx, epy) = tangent_at(0.06, t, koi.turn_rate);
        let ex = epx + enx * eye_side;
        let ey = epy + eny * eye_side;
        let (sx, sy) = transform(ex, ey);
        if sx >= 0 && sy >= 1 && sx < area.width as i32 && sy < area.height as i32 {
            let cell = &mut buf[(sx as u16, sy as u16)];
            cell.set_char('@');
            cell.set_fg(Color::Rgb(15, 15, 20));
            cell.set_style(Style::default());
        }
    }
}

// ─── Koi state + movement ───────────────────────────────────────────────────

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

    // Wall: steer toward center
    let margin = 0.25;
    let in_margin = k.x < margin || k.x > 1.0 - margin || k.y < margin || k.y > 1.0 - margin;
    if in_margin {
        let to_center = (0.5 - k.y).atan2(0.5 - k.x);
        let diff = (to_center - k.heading + PI).rem_euclid(2.0 * PI) - PI;
        k.turn_rate += diff * 0.5 * dt;
    }

    // Smoothly approach target turn rate (don't snap)
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

            // Water background with ripples
            for y in 0..area.height {
                for x in 0..area.width {
                    let xf = x as f64;
                    let yf = y as f64;
                    let r = ((xf * 0.1 + yf * 0.17 + elapsed * 0.35).sin()
                        * (xf * 0.06 - elapsed * 0.2).cos()) * 0.5 + 0.5;
                    let bg_r = (8.0 + r * 4.0) as u8;
                    let bg_g = (13.0 + r * 6.0) as u8;
                    let bg_b = (22.0 + r * 8.0) as u8;

                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(bg_r, bg_g, bg_b));
                    cell.set_fg(Color::Rgb(bg_r, bg_g, bg_b));
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
