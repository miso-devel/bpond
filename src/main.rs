use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

struct App {
    exit: bool,
    elapsed: f64,
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut last = Instant::now();
        while !self.exit {
            terminal.draw(|f| self.draw(f))?;
            let timeout = TICK.saturating_sub(last.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press
                        && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc)
                    {
                        self.exit = true;
                    }
                }
            }
            self.elapsed += last.elapsed().as_secs_f64();
            last = Instant::now();
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let t = self.elapsed;

        // Clear
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(8, 10, 18));
                cell.set_fg(Color::Rgb(8, 10, 18));
            }
        }

        let shark_len: i32 = 40;
        let sx = (area.width as i32 - shark_len) / 2;
        let mid_y = area.height as f64 / 2.0;

        // Helper: compute centerline y at a given column ratio
        let cy_at = |r: f64| -> f64 {
            let amp = 0.3 + 1.7 * r;
            mid_y + (r * 7.0 + t * 3.5).sin() * amp
        };

        // ── Pass 1: Body + fins ──
        for col in 0..shark_len {
            let r = col as f64 / shark_len as f64;
            let cy = cy_at(r);

            let half = if r < 0.12 {
                r / 0.12 * 2.0
            } else if r > 0.82 {
                (1.0 - r) / 0.18 * 2.0
            } else {
                2.0
            };

            let px = sx + col;
            if px < 0 || px >= area.width as i32 {
                continue;
            }

            let top = (cy - half).round() as i32;
            let bot = (cy + half).round() as i32;

            for py in top..=bot {
                let at_edge = py == top || py == bot;
                let (ch, fg) = if r < 0.02 {
                    ('<', Color::Rgb(130, 150, 185))
                } else if at_edge {
                    ('-', Color::Rgb(100, 125, 165))
                } else {
                    (' ', Color::Rgb(8, 10, 18))
                };
                set(buf, px, py, ch, fg, area);
            }

            // Dorsal fin
            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * 2.5;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == 1 { '/' } else { '|' };
                    set(buf, px, top - h, ch, Color::Rgb(90, 112, 150), area);
                }
            }

            // Tail fork
            if r > 0.90 {
                let spread = ((r - 0.90) / 0.10 * 2.5) as i32;
                set(buf, px, top - spread, '/', Color::Rgb(85, 105, 145), area);
                set(buf, px, bot + spread, '\\', Color::Rgb(85, 105, 145), area);
            }
        }

        // ── Pass 2: Eye — exactly one cell, follows centerline ──
        let eye_r = 0.13;
        let eye_cy = cy_at(eye_r);
        let eye_x = sx + (shark_len as f64 * eye_r).round() as i32;
        let eye_y = (eye_cy - 0.4).round() as i32;
        set(buf, eye_x, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        // ── Pass 3: Gill slits — 3 dots, follow centerline ──
        for i in 0..3 {
            let gr = 0.22 + i as f64 * 0.025;
            let gcy = cy_at(gr);
            let gx = sx + (shark_len as f64 * gr).round() as i32;
            let gy = gcy.round() as i32;
            set(buf, gx, gy, ':', Color::Rgb(60, 80, 120), area);
        }

        // Header
        if area.height > 2 && area.width > 20 {
            for (i, ch) in "  terminal-zoo  Shark    q quit"
                .chars()
                .enumerate()
            {
                if i >= area.width as usize {
                    break;
                }
                let cell = &mut buf[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(50, 45, 75));
            }
        }
    }
}

fn set(
    buf: &mut ratatui::buffer::Buffer,
    x: i32,
    y: i32,
    ch: char,
    fg: Color,
    area: ratatui::layout::Rect,
) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_style(Style::default());
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App {
        exit: false,
        elapsed: 0.0,
    }
    .run(terminal);
    ratatui::restore();
    result
}
