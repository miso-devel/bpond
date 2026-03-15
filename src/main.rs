mod animals;
mod ghost_anim;

use animals::{AnimalDef, ANIMAL_DEFS};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ghost_anim::GhostAnimation;
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
    /// 0 = Ghost (special gostty-style animation), 1..N = ANIMAL_DEFS[0..N-1]
    current: usize,
    total: usize,
    exit: bool,
    elapsed: f64,
    ghost: GhostAnimation,
    center_x: i32,
    center_y: i32,
}

impl App {
    fn new(cols: u16, rows: u16) -> Self {
        let ghost = GhostAnimation::load();
        let total = 1 + ANIMAL_DEFS.len(); // ghost + regular animals

        App {
            current: 0,
            total,
            exit: false,
            elapsed: 0.0,
            ghost,
            center_x: (cols as i32 - 77) / 2, // ghost width
            center_y: (rows as i32 - 41) / 2,  // ghost height
        }
    }

    fn recenter(&mut self) {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        if self.current == 0 {
            self.center_x = (cols as i32 - 77) / 2;
            self.center_y = (rows as i32 - 41) / 2;
        } else {
            let art = ANIMAL_DEFS[self.current - 1].frames[0];
            let art_w = art.iter().map(|l| l.len()).max().unwrap_or(0) as i32;
            let art_h = art.len() as i32;
            self.center_x = (cols as i32 - art_w) / 2;
            self.center_y = (rows as i32 - art_h) / 2;
        }
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
                self.current = (self.current + 1) % self.total;
                self.recenter();
            }
            KeyCode::Left | KeyCode::Char('p') => {
                self.current = if self.current == 0 {
                    self.total - 1
                } else {
                    self.current - 1
                };
                self.recenter();
            }
            _ => {}
        }
    }

    fn current_frame<'a>(&self, def: &'a AnimalDef) -> &'a [&'a str] {
        let total: f64 = def.sequence.iter().map(|&(_, d)| d).sum();
        let pos = self.elapsed % total;
        let mut acc = 0.0;
        for &(idx, dur) in def.sequence {
            acc += dur;
            if pos < acc {
                return def.frames[idx];
            }
        }
        def.frames[0]
    }

    fn draw(&self, frame: &mut Frame) {
        if self.current == 0 {
            // Ghost: gostty-style 235-frame playback
            self.ghost.draw(frame, self.elapsed);
        } else {
            // Regular animals
            self.draw_animal(frame);
        }
    }

    fn draw_animal(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let t = self.elapsed;

        // Dark background
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 8, 16));
                cell.set_fg(Color::Rgb(10, 8, 16));
            }
        }

        let def = &ANIMAL_DEFS[self.current - 1];
        let art = self.current_frame(def);

        let art_h = art.len();
        let ax = self.center_x;
        let ay = self.center_y;

        // Breathing
        let breath = 0.78 + 0.22 * (t * 0.8).sin();
        let wave_pos = 0.5 + 0.4 * (t * 0.5).sin();
        let glow_strength = 0.08 + 0.06 * (t * 1.2).sin();

        // Pass 1: Glow aura
        for (row, line) in art.iter().enumerate() {
            let row_ratio = row as f64 / art_h.max(1) as f64;
            let r = lerp_u8(def.color_top.0, def.color_bot.0, row_ratio);
            let g = lerp_u8(def.color_top.1, def.color_bot.1, row_ratio);
            let b = lerp_u8(def.color_top.2, def.color_bot.2, row_ratio);

            for (col, ch) in line.chars().enumerate() {
                if ch != ' ' || !has_art_neighbor(art, row, col) {
                    continue;
                }
                let px = ax + col as i32;
                let py = ay + row as i32;
                if !in_bounds(px, py, area) {
                    continue;
                }
                let glow = glow_strength
                    * (0.8 + 0.2 * (t * 1.5 + row as f64 * 0.12 + col as f64 * 0.08).sin());
                let gr = (r as f64 * glow).min(255.0) as u8;
                let gg = (g as f64 * glow).min(255.0) as u8;
                let gb = (b as f64 * glow).min(255.0) as u8;
                if gr > 0 || gg > 0 || gb > 0 {
                    let cell = &mut buf[(px as u16, py as u16)];
                    cell.set_char('·');
                    cell.set_fg(Color::Rgb(gr, gg, gb));
                }
            }
        }

        // Pass 2: Main art
        for (row, line) in art.iter().enumerate() {
            let row_ratio = row as f64 / art_h.max(1) as f64;
            let r = lerp_u8(def.color_top.0, def.color_bot.0, row_ratio);
            let g = lerp_u8(def.color_top.1, def.color_bot.1, row_ratio);
            let b = lerp_u8(def.color_top.2, def.color_bot.2, row_ratio);

            let wave_dist = (row_ratio - wave_pos).abs();
            let wave_boost = 1.0 + 0.25 * (1.0 - (wave_dist * 3.0).min(1.0)).max(0.0);

            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let px = ax + col as i32;
                let py = ay + row as i32;
                if !in_bounds(px, py, area) {
                    continue;
                }

                let on_edge = is_edge_char(art, row, col);
                let (display_ch, base_weight) = if on_edge {
                    animate_edge_char(ch, t, row, col)
                } else {
                    (ch, char_weight(ch))
                };

                let phase = col as f64 * 0.12 + row as f64 * 0.08;
                let shimmer = 0.88 + 0.12 * (t * 2.0 + phase).sin();
                let weight = base_weight * breath * shimmer * wave_boost;

                let fr = (r as f64 * weight).clamp(0.0, 255.0) as u8;
                let fg = (g as f64 * weight).clamp(0.0, 255.0) as u8;
                let fb = (b as f64 * weight).clamp(0.0, 255.0) as u8;

                let cell = &mut buf[(px as u16, py as u16)];
                cell.set_char(display_ch);
                cell.set_fg(Color::Rgb(fr, fg, fb));
                cell.set_style(Style::default().add_modifier(Modifier::BOLD));
            }
        }

        // Header
        self.draw_header(frame, def.name, def.color_top, def.color_bot);
    }

    fn draw_header(
        &self,
        frame: &mut Frame,
        name: &str,
        color_top: (u8, u8, u8),
        color_bot: (u8, u8, u8),
    ) {
        let area = frame.area();
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
                    name,
                    Style::default().fg(Color::Rgb(
                        color_top.0 / 2 + color_bot.0 / 2,
                        color_top.1 / 2 + color_bot.1 / 2,
                        color_top.2 / 2 + color_bot.2 / 2,
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

fn char_weight(ch: char) -> f64 {
    match ch {
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
    }
}

fn animate_edge_char(ch: char, t: f64, row: usize, col: usize) -> (char, f64) {
    let wave = (t * 1.8 + row as f64 * 0.2 + col as f64 * 0.12).sin();
    let c = match ch {
        '·' | '.' => if wave > 0.3 { '+' } else { '·' },
        '+' => if wave > 0.33 { '*' } else if wave < -0.33 { '·' } else { '+' },
        '*' => if wave > 0.33 { '=' } else if wave < -0.33 { '+' } else { '*' },
        '=' => if wave > 0.33 { '%' } else if wave < -0.33 { '*' } else { '=' },
        '%' => if wave > 0.33 { '$' } else if wave < -0.33 { '=' } else { '%' },
        _ => ch,
    };
    (c, char_weight(c))
}

fn is_edge_char(art: &[&str], row: usize, col: usize) -> bool {
    const DIRS: [(i32, i32); 8] = [
        (-1, -1), (-1, 0), (-1, 1), (0, -1),
        (0, 1), (1, -1), (1, 0), (1, 1),
    ];
    for &(dr, dc) in &DIRS {
        let nr = row as i32 + dr;
        let nc = col as i32 + dc;
        if nr < 0 || nr >= art.len() as i32 { return true; }
        let line = art[nr as usize];
        if nc < 0 || nc >= line.len() as i32 { return true; }
        if line.as_bytes()[nc as usize] == b' ' { return true; }
    }
    false
}

fn in_bounds(x: i32, y: i32, area: Rect) -> bool {
    x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32
}

fn has_art_neighbor(art: &[&str], row: usize, col: usize) -> bool {
    for &(dr, dc) in &[(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
        let nr = row as i32 + dr;
        let nc = col as i32 + dc;
        if nr < 0 || nr >= art.len() as i32 { continue; }
        let line = art[nr as usize];
        if nc < 0 || nc >= line.len() as i32 { continue; }
        if line.as_bytes()[nc as usize] != b' ' { return true; }
    }
    false
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
