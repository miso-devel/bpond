mod canvas;
mod koi;

use canvas::Canvas;
use color_eyre::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
    MouseEventKind,
};
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

    // Enable mouse capture
    crossterm::execute!(std::io::stdout(), EnableMouseCapture)?;

    // Food pellets: (x, y, age)
    let mut foods: Vec<(f64, f64, f64)> = Vec::new();

    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f64() * speed;
        elapsed += dt;
        last = Instant::now();

        let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));

        // Age food and remove old ones
        for f in &mut foods {
            f.2 += dt;
        }
        foods.retain(|f| f.2 < 15.0);

        for k in &mut fish {
            k.update(dt, elapsed, tw as f64, th as f64, &foods);
        }

        // Remove eaten food
        foods.retain(|&(fx, fy, _)| {
            !fish.iter().any(|k| {
                let dx = k.spine_x[0] - fx;
                let dy = k.spine_y[0] - fy;
                dx * dx + dy * dy < 1.5
            })
        });

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

            // Draw food pellets
            for &(fx, fy, age) in &foods {
                let px = (fx * scale) as i32;
                let py = (fy * scale) as i32;
                let fade = (1.0 - age / 15.0).max(0.0);
                let r = (180.0 * fade) as u8;
                let g = (120.0 * fade) as u8;
                let b = (50.0 * fade) as u8;
                canvas.fat(px, py, r, g, b);
            }

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
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => speed = (speed + 0.2).min(5.0),
                    KeyCode::Down => speed = (speed - 0.2).max(0.2),
                    _ => {}
                },
                Event::Mouse(m) => {
                    if let MouseEventKind::Down(MouseButton::Left) = m.kind {
                        // Drop food at click position (offset y by -1 for header row)
                        let fx = m.column as f64;
                        let fy = (m.row.saturating_sub(1)) as f64;
                        foods.push((fx, fy, 0.0));
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
