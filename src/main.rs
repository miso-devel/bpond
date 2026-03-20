mod canvas;
mod food;
mod koi;
mod pond;

use anyhow::Result;
use canvas::Canvas;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
    MouseEventKind,
};
use ratatui::style::Color;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16); // ~60 fps

fn main() -> Result<()> {
    let debug = std::env::args().any(|a| a == "--debug");
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

            draw_water(buf, area, elapsed);

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

            draw_food(&pond, &mut canvas, scale);
            for k in &pond.fish {
                k.draw(&mut canvas, elapsed, scale);
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
                    _ => {}
                },
                Event::Mouse(m) => {
                    if let MouseEventKind::Down(MouseButton::Left) = m.kind {
                        let scale = pond::compute_scale(tw, th);
                        let (fx, fy) = pond::screen_to_world(m.column, m.row, scale);
                        pond.drop_food(fx, fy);
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

fn draw_water(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, elapsed: f64) {
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
                (10.0 + ripple * 4.0) as u8,
                (18.0 + ripple * 6.0) as u8,
                (32.0 + ripple * 9.0) as u8,
            ));
            cell.set_fg(Color::Rgb(10, 18, 32));
        }
    }
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

fn draw_header(buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, speed: f64) {
    if area.width <= 20 {
        return;
    }
    let hdr = format!(
        "  mini-pond  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
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
