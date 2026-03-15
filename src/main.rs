mod sdf;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16); // ~60fps

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
        let w = area.width as f64;
        let h = area.height as f64;
        let aspect = 0.5; // terminal cells are ~2x taller than wide

        for y in 0..area.height {
            for x in 0..area.width {
                let nx = (x as f64 - w / 2.0) * aspect / (h / 2.0);
                let ny = (y as f64 - h / 2.0) / (h / 2.0);

                // Features layer (eyes, gills) — drawn on top
                if let Some((fch, fcolor)) = sdf::shark_features(nx, ny, t) {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(fch);
                    cell.set_fg(fcolor);
                    cell.set_bg(Color::Rgb(10, 8, 16));
                    cell.set_style(Style::default());
                    continue;
                }

                // Body SDF → outline-focused rendering
                let d = sdf::shark(nx, ny, t);
                let (ch, fg) = sdf::shark_render(d, t, nx, ny);

                let cell = &mut buf[(x, y)];
                cell.set_char(ch);
                cell.set_fg(fg);
                cell.set_bg(Color::Rgb(10, 8, 16));
                cell.set_style(Style::default());
            }
        }

        // Header
        if area.height > 2 && area.width > 30 {
            let hdr = "  terminal-zoo  Shark    q quit";
            for (i, ch) in hdr.chars().enumerate() {
                if i >= area.width as usize {
                    break;
                }
                let cell = &mut buf[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(80, 70, 120));
            }
        }
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
