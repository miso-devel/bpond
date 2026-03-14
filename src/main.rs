mod animals;

use animals::{AnimalDef, ANIMAL_DEFS};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use rand::Rng;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(16); // ~60fps

// ─── Particle ───────────────────────────────────────────────────────────────

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max_life: f64,
    ch: char,
    color: (u8, u8, u8),
}

impl Particle {
    fn update(&mut self, dt: f64) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.life -= dt;
    }
    fn alpha(&self) -> f64 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

struct App {
    current_animal: usize,
    particles: Vec<Particle>,
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
            particles: Vec::new(),
            exit: false,
            elapsed: 0.0,
            base_x: (cols as f64 - art_w) / 2.0,
            base_y: (rows as f64 - art_h) / 2.0,
        }
    }

    fn def(&self) -> &'static AnimalDef {
        &ANIMAL_DEFS[self.current_animal]
    }

    fn recenter(&mut self) {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let art = self.def().art_a;
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
            self.on_tick(dt);
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

    fn on_tick(&mut self, dt: f64) {
        self.elapsed += dt;

        for p in &mut self.particles {
            p.update(dt);
        }
        self.particles.retain(|p| p.life > 0.0);

        // Spawn ambient particles
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.08) {
            let def = self.def();
            let (dx, dy) = self.smooth_offset();
            let art = def.art_a;
            let art_w = art.iter().map(|l| l.len()).max().unwrap_or(0) as f64;
            let art_h = art.len() as f64;
            let ax = self.base_x + dx;
            let ay = self.base_y + dy;

            let life = rng.gen_range(1.5..3.5);
            let chars = ['·', '✧', '°', '•'];
            self.particles.push(Particle {
                x: ax + rng.gen_range(0.0..art_w),
                y: ay + rng.gen_range(0.0..art_h),
                vx: rng.gen_range(-0.3..0.3),
                vy: rng.gen_range(-0.8..-0.2),
                life,
                max_life: life,
                ch: chars[rng.gen_range(0..chars.len())],
                color: def.color_top,
            });
        }
    }

    /// Multiple sine waves at different frequencies for organic floating
    fn smooth_offset(&self) -> (f64, f64) {
        let t = self.elapsed;
        let dx = (t * 0.5).sin() * 4.0 + (t * 0.23).sin() * 2.5 + (t * 0.11).cos() * 1.5;
        let dy = (t * 0.37).sin() * 3.0 + (t * 0.17).cos() * 2.0 + (t * 0.08).sin() * 1.5;
        (dx, dy)
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Plain dark background
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(12, 10, 18));
            }
        }

        // Particles
        for p in &self.particles {
            let px = p.x.round() as u16;
            let py = p.y.round() as u16;
            if px > 0 && px < area.width && py > 0 && py < area.height {
                let a = p.alpha();
                let r = (p.color.0 as f64 * a) as u8;
                let g = (p.color.1 as f64 * a) as u8;
                let b = (p.color.2 as f64 * a) as u8;
                let cell = &mut buf[(px, py)];
                cell.set_char(p.ch);
                cell.set_fg(Color::Rgb(r, g, b));
            }
        }

        // Animal
        let def = self.def();
        let (dx, dy) = self.smooth_offset();

        // Breathing: slow blink cycle (~4s)
        let blink = ((self.elapsed * 0.25 * std::f64::consts::TAU).sin() + 1.0) / 2.0;
        let art = if blink > 0.85 { def.art_b } else { def.art_a };

        let art_h = art.len();
        let ax = (self.base_x + dx).round() as i32;
        let ay = (self.base_y + dy).round() as i32;

        for (row, line) in art.iter().enumerate() {
            let row_ratio = row as f64 / art_h.max(1) as f64;

            // Gradient top → bottom
            let r = lerp_u8(def.color_top.0, def.color_bot.0, row_ratio);
            let g = lerp_u8(def.color_top.1, def.color_bot.1, row_ratio);
            let b = lerp_u8(def.color_top.2, def.color_bot.2, row_ratio);

            // Gentle brightness pulse
            let pulse = 0.9 + 0.1 * (self.elapsed * 1.2 + row as f64 * 0.15).sin();
            let r = (r as f64 * pulse).min(255.0) as u8;
            let g = (g as f64 * pulse).min(255.0) as u8;
            let b = (b as f64 * pulse).min(255.0) as u8;

            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let px = ax + col as i32;
                let py = ay + row as i32;
                if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                    continue;
                }

                // Character density → brightness (Ghostty style)
                let weight = match ch {
                    '@' => 1.0,
                    '$' => 0.92,
                    '%' => 0.82,
                    '#' => 0.75,
                    '*' => 0.65,
                    '=' => 0.55,
                    '+' => 0.45,
                    'x' => 0.40,
                    'o' => 0.35,
                    '~' => 0.30,
                    '-' => 0.25,
                    ':' => 0.20,
                    '·' | '\'' | '.' | ',' => 0.15,
                    _ => 0.50,
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

        // Header
        let header_area = Rect::new(0, 0, area.width, 3);
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(40, 35, 55)));

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "terminal-zoo",
                    Style::default()
                        .fg(Color::Rgb(140, 120, 200))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    def.name,
                    Style::default().fg(Color::Rgb(
                        def.color_top.0,
                        def.color_top.1,
                        def.color_top.2,
                    )),
                ),
            ]),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    "← →",
                    Style::default()
                        .fg(Color::Rgb(100, 180, 230))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" switch  ", Style::default().fg(Color::Rgb(50, 45, 70))),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Rgb(200, 100, 100))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" quit", Style::default().fg(Color::Rgb(50, 45, 70))),
            ]),
        ])
        .block(header_block);
        frame.render_widget(header, header_area);
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f64 + (b as f64 - a as f64) * t) as u8
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let (cols, rows) = crossterm::terminal::size()?;
    let result = App::new(cols, rows).run(terminal);
    ratatui::restore();
    result
}
