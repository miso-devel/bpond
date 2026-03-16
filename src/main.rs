mod canvas;
mod koi;

use canvas::Canvas;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use koi::Koi;
use ratatui::style::Color;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16); // ~60 fps

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
    let (w, h) = (tw as f64, th as f64);
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
        for k in &mut fish {
            k.update(dt, elapsed, tw as f64, th as f64);
        }

        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            // Water background with subtle animated ripples
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

            // Braille canvas for koi rendering
            let cw = area.width as usize;
            let ch = (area.height as usize).saturating_sub(1);
            if cw < 4 || ch < 4 {
                return;
            }
            let mut canvas = Canvas::new(cw, ch);

            // Uniform scale so fish size doesn't change with heading
            let scale =
                (canvas.h as f64 / th as f64).min(canvas.w as f64 / tw as f64);

            for k in &fish {
                k.draw(&mut canvas, elapsed, scale);
            }
            canvas.render(buf, 0, 1, area);

            // Header
            if area.width > 20 {
                let hdr = format!(
                    "  terminal-zoo  Koi Pond  speed:{:.1}x  \u{2191}\u{2193}:speed  q:quit",
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
