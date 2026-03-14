mod animals;

use animals::ANIMAL_DEFS;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(16); // ~60fps

// ─── App ────────────────────────────────────────────────────────────────────

struct App {
    current_animal: usize,
    exit: bool,
    elapsed: f64,
    base_x: f64,
    base_y: f64,
}

impl App {
    fn new(cols: u16, rows: u16) -> Self {
        let art = ANIMAL_DEFS[0].art_a;
        let art_w = art.iter().map(|l| l.len()).max().unwrap_or(0) as f64;
        let art_h = art.len() as f64;

        App {
            current_animal: 0,
            exit: false,
            elapsed: 0.0,
            base_x: (cols as f64 - art_w) / 2.0,
            base_y: (rows as f64 - art_h) / 2.0,
        }
    }

    fn recenter(&mut self) {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let art = ANIMAL_DEFS[self.current_animal].art_a;
        let art_w = art.iter().map(|l| l.len()).max().unwrap_or(0) as f64;
        let art_h = art.len() as f64;
        self.base_x = (cols as f64 - art_w) / 2.0;
        self.base_y = (rows as f64 - art_h) / 2.0;
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut last_tick = Instant::now();

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;

            let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }

            let dt = last_tick.elapsed().as_secs_f64();
            self.elapsed += dt;
            last_tick = Instant::now();
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
            KeyCode::Right | KeyCode::Char('n') => {
                self.current_animal = (self.current_animal + 1) % ANIMAL_DEFS.len();
                self.recenter();
            }
            KeyCode::Left | KeyCode::Char('p') => {
                self.current_animal = if self.current_animal == 0 {
                    ANIMAL_DEFS.len() - 1
                } else {
                    self.current_animal - 1
                };
                self.recenter();
            }
            _ => {}
        }
    }

    /// Multiple sine waves for organic floating motion
    fn smooth_offset(&self) -> (f64, f64) {
        let t = self.elapsed;
        let dx = (t * 0.5).sin() * 4.0 + (t * 0.23).sin() * 2.5 + (t * 0.11).cos() * 1.5;
        let dy = (t * 0.37).sin() * 3.0 + (t * 0.17).cos() * 2.0 + (t * 0.08).sin() * 1.5;
        (dx, dy)
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Dark background — set once, ratatui diffs the rest
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 8, 16));
                cell.set_fg(Color::Rgb(10, 8, 16));
            }
        }

        // Animal
        let def = &ANIMAL_DEFS[self.current_animal];
        let (dx, dy) = self.smooth_offset();

        // Blink: eyes close briefly every ~4s
        let blink_cycle = (self.elapsed * 0.25 * std::f64::consts::TAU).sin();
        let art = if blink_cycle > 0.92 { def.art_b } else { def.art_a };

        let art_h = art.len();
        let ax = (self.base_x + dx).round() as i32;
        let ay = (self.base_y + dy).round() as i32;

        for (row, line) in art.iter().enumerate() {
            let row_ratio = row as f64 / art_h.max(1) as f64;

            // Top-to-bottom color gradient
            let r = lerp_u8(def.color_top.0, def.color_bot.0, row_ratio);
            let g = lerp_u8(def.color_top.1, def.color_bot.1, row_ratio);
            let b = lerp_u8(def.color_top.2, def.color_bot.2, row_ratio);

            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let px = ax + col as i32;
                let py = ay + row as i32;
                if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                    continue;
                }

                // Character density → brightness
                let weight = match ch {
                    '@' => 1.0,
                    '$' => 0.90,
                    '%' => 0.78,
                    '#' => 0.70,
                    '*' => 0.60,
                    '=' => 0.50,
                    '+' => 0.40,
                    'x' => 0.35,
                    'o' => 0.30,
                    '~' => 0.25,
                    '-' => 0.22,
                    ':' => 0.18,
                    '·' | '.' | '\'' | ',' => 0.12,
                    _ => 0.45,
                };

                let fr = (r as f64 * weight).min(255.0) as u8;
                let fg = (g as f64 * weight).min(255.0) as u8;
                let fb = (b as f64 * weight).min(255.0) as u8;

                let cell = &mut buf[(px as u16, py as u16)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(fr, fg, fb));
                cell.set_style(Style::default().add_modifier(Modifier::BOLD));
            }
        }

        // Minimal header
        let header_area = Rect::new(0, 0, area.width, 3);
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(35, 30, 50)));

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "terminal-zoo",
                    Style::default()
                        .fg(Color::Rgb(120, 100, 180))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    def.name,
                    Style::default().fg(Color::Rgb(
                        def.color_top.0 / 2 + def.color_bot.0 / 2,
                        def.color_top.1 / 2 + def.color_bot.1 / 2,
                        def.color_top.2 / 2 + def.color_bot.2 / 2,
                    )),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "← →",
                    Style::default()
                        .fg(Color::Rgb(80, 140, 200))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" switch  ", Style::default().fg(Color::Rgb(40, 35, 60))),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Rgb(180, 80, 80))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" quit", Style::default().fg(Color::Rgb(40, 35, 60))),
            ]),
        ])
        .block(header_block);
        frame.render_widget(header, header_area);
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f64 + (b as f64 - a as f64) * t) as u8
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let (cols, rows) = crossterm::terminal::size()?;
    let result = App::new(cols, rows).run(terminal);
    ratatui::restore();
    result
}
