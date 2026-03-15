use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// Simple shark as line art: each row is (y_offset, string)
// Facing left, dorsal fin on top, tail on right
const SHARK: &[&str] = &[
    r"            /\            ",
    r"       ____/  \___        ",
    r"  ____/    '   \  \____   ",
    r" /  O    |  |  |       \==",
    r" \____   |  |  |   ___/==",
    r"      \____.___\__/       ",
];

const SHARK_W: usize = 28;
const SHARK_H: usize = 6;

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

        // Dark background
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 10, 20));
                cell.set_fg(Color::Rgb(10, 10, 20));
            }
        }

        // Center the shark
        let cx = (area.width as i32 - SHARK_W as i32) / 2;
        let cy = (area.height as i32 - SHARK_H as i32) / 2;

        for (row, line) in SHARK.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }

                // Undulation: sine wave bends each column, stronger toward tail
                let col_ratio = col as f64 / SHARK_W as f64; // 0=left(head), 1=right(tail)
                let bend = (col_ratio * 3.0 + t * 3.5).sin()
                    * 1.8
                    * col_ratio
                    * col_ratio;

                let px = cx + col as i32;
                let py = cy + row as i32 + bend.round() as i32;

                if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                    continue;
                }

                let color = match ch {
                    'O' => Color::Rgb(220, 230, 240),
                    '=' => Color::Rgb(120, 140, 170),
                    '|' => Color::Rgb(80, 100, 140),
                    _ => {
                        // Gradient: lighter at head, darker toward tail
                        let v = 180.0 - 60.0 * col_ratio;
                        Color::Rgb((v * 0.5) as u8, (v * 0.66) as u8, v as u8)
                    }
                };

                let cell = &mut buf[(px as u16, py as u16)];
                cell.set_char(ch);
                cell.set_fg(color);
                cell.set_style(Style::default());
            }
        }

        // Header
        if area.height > 2 && area.width > 20 {
            let hdr = "  terminal-zoo  Shark    q quit";
            for (i, ch) in hdr.chars().enumerate() {
                if i >= area.width as usize {
                    break;
                }
                let cell = &mut buf[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(60, 55, 90));
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
