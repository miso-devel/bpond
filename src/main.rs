mod bubble;
mod canvas;
mod food;
mod frog;
mod koi;
mod lily;
mod pond;
mod rain;
mod ripple;
mod rng;

use canvas::Canvas;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
    MouseEventKind,
};
use ratatui::style::Color;
use std::io;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16); // ~60 fps

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let debug = args.iter().any(|a| a == "--debug");
    let bg_override = parse_bg_arg(&args);
    let mut terminal = ratatui::init();

    let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
    let (w, h) = (tw as f64, pond::world_height(th));
    let mut pond = pond::Pond::new(w, h);

    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
        let world_h = pond::world_height(th);

        pond.update(dt, elapsed, tw as f64, world_h);

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            draw_water(buf, area, elapsed, bg_override);

            let cw = area.width as usize;
            let ch = if debug {
                (area.height as usize).saturating_sub(1)
            } else {
                area.height as usize
            };
            if cw < 4 || ch < 4 {
                return;
            }
            let mut canvas = Canvas::new(cw, ch);
            let scale = pond::compute_scale(tw, th);

            draw_ripples(&pond, &mut canvas, scale);
            draw_bubbles(&pond, &mut canvas, scale);
            draw_rain(&pond, &mut canvas, scale);
            draw_food(&pond, &mut canvas, scale);
            for k in &pond.fish {
                k.draw(&mut canvas, elapsed, scale);
            }
            // Lily pads float on the water surface, so they're painted
            // last — koi underneath are partially occluded where pads
            // overlap.
            for pad in &pond.lilies {
                pad.draw(&mut canvas, scale, elapsed);
            }
            // Frogs sit on top of the pads / water.
            for fr in &pond.frogs {
                fr.draw(&mut canvas, scale);
            }
            if debug {
                canvas.render(buf, 0, 1, area);
                draw_header(buf, area, speed);
            } else {
                canvas.render(buf, 0, 0, area);
            }
        })?;

        let timeout = TICK.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => speed = (speed + 0.2).min(5.0),
                    KeyCode::Down => speed = (speed - 0.2).max(0.2),
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        pond.add_fish(tw as f64, world_h, elapsed);
                    }
                    KeyCode::Char('-') => pond.remove_fish(),
                    KeyCode::Char('r') | KeyCode::Char('R') => pond.toggle_rain(),
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        let fx = (0.1 + rng::pseudo_rand(elapsed) * 0.8) * tw as f64;
                        let fy = (0.2 + rng::pseudo_rand(elapsed + 7.3) * 0.7) * world_h;
                        pond.drop_food(fx, fy, tw as f64, world_h);
                    }
                    _ => {}
                },
                Event::Mouse(m) => {
                    if let MouseEventKind::Down(MouseButton::Left) = m.kind {
                        let scale = pond::compute_scale(tw, th);
                        let (fx, fy) = pond::screen_to_world(m.column, m.row, scale);
                        pond.drop_food(fx, fy, tw as f64, world_h);
                    }
                    if let MouseEventKind::Down(MouseButton::Right) = m.kind {
                        let scale = pond::compute_scale(tw, th);
                        let (fx, fy) = pond::screen_to_world(m.column, m.row, scale);
                        pond.scare(fx, fy, tw as f64, world_h);
                    }
                }
                _ => {}
            }
        }
    }

    crossterm::execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}

fn draw_water(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    elapsed: f64,
    bg_override: Option<(u8, u8, u8)>,
) {
    // Base water colour. The default cycles slowly across a
    // day/night band tuned to stay distinctly brighter than the
    // koi's outline/silhouette (≈ 8-32 on each channel) so dark
    // koi edges read as dark *against* the water rather than
    // disappearing into it. `--bg <hex>` overrides the cycle with a
    // pinned RGB so screenshots and recordings don't depend on
    // where in the cycle the simulation happens to sit. Either way
    // the per-cell ripple modulation still rides on top so the
    // surface looks alive instead of a flat painted block.
    let (base_r, base_g, base_b) = match bg_override {
        Some((r, g, b)) => (f64::from(r), f64::from(g), f64::from(b)),
        None => {
            let day = (elapsed * 0.03).sin() * 0.5 + 0.5;
            (12.0 + day * 14.0, 22.0 + day * 16.0, 40.0 + day * 18.0)
        }
    };

    for y in 0..area.height {
        for x in 0..area.width {
            let (xf, yf) = (x as f64, y as f64);
            let ripple = ((xf * 0.08 + yf * 0.14 + elapsed * 0.2).sin()
                * (xf * 0.05 - elapsed * 0.12).cos())
                * 0.5
                + 0.5;
            let cell = &mut buf[(x, y)];
            cell.set_char(' ');
            cell.set_bg(Color::Rgb(
                (base_r + ripple * 4.0) as u8,
                (base_g + ripple * 6.0) as u8,
                (base_b + ripple * 9.0) as u8,
            ));
            cell.set_fg(Color::Rgb(10, 18, 32));
        }
    }
}

fn draw_ripples(pond: &pond::Pond, canvas: &mut Canvas, scale: f64) {
    for r in &pond.ripples {
        r.draw(canvas, scale);
    }
}

fn draw_bubbles(pond: &pond::Pond, canvas: &mut Canvas, scale: f64) {
    for b in &pond.bubbles {
        b.draw(canvas, scale);
    }
}

fn draw_rain(pond: &pond::Pond, canvas: &mut Canvas, scale: f64) {
    pond.rain.draw(canvas, scale);
}

fn draw_food(pond: &pond::Pond, canvas: &mut Canvas, scale: f64) {
    for food in &pond.foods {
        let px = (food.x * scale) as i32;
        let py = (food.y * scale) as i32;
        let fade = food.fade();
        canvas.fat(
            px,
            py,
            (180.0 * fade) as u8,
            (120.0 * fade) as u8,
            (50.0 * fade) as u8,
        );
    }
}

/// Parse `--bg <hex>` (or `--bg=<hex>`) out of the argument list.
/// Accepts `RRGGBB` with or without a leading `#`. Anything else
/// returns `None` and we fall back to the day/night cycle base.
fn parse_bg_arg(args: &[String]) -> Option<(u8, u8, u8)> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let value = if let Some(v) = arg.strip_prefix("--bg=") {
            v
        } else if arg == "--bg" {
            iter.next().map(String::as_str)?
        } else {
            continue;
        };
        return parse_hex_rgb(value);
    }
    None
}

fn parse_hex_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

fn draw_header(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, speed: f64) {
    if area.width <= 20 {
        return;
    }
    let hdr = format!(
        "  bpond  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  f:feed  q:quit",
        speed
    );
    for (i, ch) in hdr.chars().enumerate() {
        if i >= area.width as usize {
            break;
        }
        let cell = &mut buf[(i as u16, 0)];
        cell.set_char(ch);
        cell.set_fg(Color::Rgb(60, 55, 85));
        cell.set_bg(Color::Rgb(10, 16, 28));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parse_hex_accepts_six_digit_lowercase() {
        assert_eq!(parse_hex_rgb("1a2b3c"), Some((0x1a, 0x2b, 0x3c)));
    }

    #[test]
    fn parse_hex_accepts_leading_hash_and_uppercase() {
        assert_eq!(parse_hex_rgb("#FFAA00"), Some((0xff, 0xaa, 0x00)));
    }

    #[test]
    fn parse_hex_rejects_short_strings() {
        assert_eq!(parse_hex_rgb("12345"), None);
        assert_eq!(parse_hex_rgb("#1a2"), None);
    }

    #[test]
    fn parse_hex_rejects_non_hex() {
        assert_eq!(parse_hex_rgb("1a2zzz"), None);
    }

    #[test]
    fn parse_bg_finds_space_separated_value() {
        let argv = args(&["bpond", "--bg", "1a2434", "--debug"]);
        assert_eq!(parse_bg_arg(&argv), Some((0x1a, 0x24, 0x34)));
    }

    #[test]
    fn parse_bg_finds_equals_separated_value() {
        let argv = args(&["bpond", "--bg=#1a2434"]);
        assert_eq!(parse_bg_arg(&argv), Some((0x1a, 0x24, 0x34)));
    }

    #[test]
    fn parse_bg_ignores_invalid_hex() {
        let argv = args(&["bpond", "--bg", "notacolor"]);
        assert_eq!(parse_bg_arg(&argv), None);
    }

    #[test]
    fn parse_bg_returns_none_when_flag_missing() {
        let argv = args(&["bpond", "--debug"]);
        assert_eq!(parse_bg_arg(&argv), None);
    }
}
