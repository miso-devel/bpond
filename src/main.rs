use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

#[derive(Clone, Copy)]
enum FillStyle {
    Tilde,    // ~ fill (classic)
    Dots,     // · sparse fill
    Empty,    // outline only
    Gradient, // varies by depth: center dense, edge sparse
}

struct Shark {
    name: &'static str,
    len: i32,
    max_half: f64,
    wave_k: f64,
    wave_speed: f64,
    amp_head: f64,
    amp_tail: f64,
    directional_edges: bool, // use /\ based on body slope
    fill: FillStyle,
    dark: (f64, f64, f64),
    light: (f64, f64, f64),
}

const SHARKS: &[Shark] = &[
    // 1: Classic — the original
    Shark {
        name: "Classic",
        len: 40,
        max_half: 2.0,
        wave_k: 7.0,
        wave_speed: 3.5,
        amp_head: 0.3,
        amp_tail: 1.7,
        directional_edges: false,
        fill: FillStyle::Tilde,
        dark: (55.0, 75.0, 125.0),
        light: (165.0, 175.0, 185.0),
    },
    // 2: Directional — edges show body curve direction
    Shark {
        name: "Directional",
        len: 40,
        max_half: 2.0,
        wave_k: 7.0,
        wave_speed: 3.5,
        amp_head: 0.3,
        amp_tail: 1.7,
        directional_edges: true,
        fill: FillStyle::Dots,
        dark: (55.0, 75.0, 125.0),
        light: (165.0, 175.0, 185.0),
    },
    // 3: Outline — no fill, clean silhouette
    Shark {
        name: "Outline",
        len: 40,
        max_half: 2.0,
        wave_k: 7.0,
        wave_speed: 3.5,
        amp_head: 0.3,
        amp_tail: 1.7,
        directional_edges: true,
        fill: FillStyle::Empty,
        dark: (80.0, 110.0, 180.0),
        light: (150.0, 170.0, 210.0),
    },
    // 4: Large Smooth — bigger body, less amplitude = fewer integer jumps
    Shark {
        name: "Large Smooth",
        len: 55,
        max_half: 3.0,
        wave_k: 5.0,
        wave_speed: 2.2,
        amp_head: 0.12,
        amp_tail: 0.9,
        directional_edges: true,
        fill: FillStyle::Gradient,
        dark: (45.0, 65.0, 115.0),
        light: (160.0, 172.0, 192.0),
    },
    // 5: Dense — heavier fill, bolder look
    Shark {
        name: "Dense",
        len: 45,
        max_half: 2.5,
        wave_k: 6.0,
        wave_speed: 3.0,
        amp_head: 0.2,
        amp_tail: 1.3,
        directional_edges: false,
        fill: FillStyle::Gradient,
        dark: (40.0, 60.0, 110.0),
        light: (150.0, 165.0, 185.0),
    },
    // 6: Refined — best combination of all improvements
    Shark {
        name: "Refined",
        len: 48,
        max_half: 2.5,
        wave_k: 5.5,
        wave_speed: 2.8,
        amp_head: 0.15,
        amp_tail: 1.1,
        directional_edges: true,
        fill: FillStyle::Gradient,
        dark: (50.0, 72.0, 125.0),
        light: (162.0, 175.0, 195.0),
    },
];

struct App {
    current: usize,
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
                    if k.kind == KeyEventKind::Press {
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
                            KeyCode::Right | KeyCode::Char('n') => {
                                self.current = (self.current + 1) % SHARKS.len();
                            }
                            KeyCode::Left | KeyCode::Char('p') => {
                                self.current = if self.current == 0 {
                                    SHARKS.len() - 1
                                } else {
                                    self.current - 1
                                };
                            }
                            _ => {}
                        }
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
        let s = &SHARKS[self.current];

        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(8, 10, 18));
                cell.set_fg(Color::Rgb(8, 10, 18));
            }
        }

        let sx = (area.width as i32 - s.len) / 2;
        let mid_y = area.height as f64 / 2.0;

        let cy_at = |r: f64| -> f64 {
            let amp = s.amp_head + (s.amp_tail - s.amp_head) * r;
            mid_y + (r * s.wave_k + t * s.wave_speed).sin() * amp
        };

        // Body slope at each column (for directional edges)
        let slope_at = |r: f64| -> f64 {
            let dr = 0.01;
            cy_at((r + dr).min(1.0)) - cy_at((r - dr).max(0.0))
        };

        for col in 0..s.len {
            let r = col as f64 / s.len as f64;
            let cy = cy_at(r);
            let slope = slope_at(r);

            let half = if r < 0.12 {
                r / 0.12 * s.max_half
            } else if r > 0.82 {
                (1.0 - r) / 0.18 * s.max_half
            } else {
                s.max_half
            };

            let px = sx + col;
            if px < 0 || px >= area.width as i32 {
                continue;
            }

            let top = (cy - half).round() as i32;
            let bot = (cy + half).round() as i32;

            for py in top..=bot {
                let vert = if top == bot {
                    0.5
                } else {
                    (py - top) as f64 / (bot - top) as f64
                };
                let depth = if top == bot {
                    0.0
                } else {
                    (0.5 - (vert - 0.5).abs()) * 2.0 // 0 at edges, 1 at center
                };

                let cr = s.dark.0 + (s.light.0 - s.dark.0) * vert;
                let cg = s.dark.1 + (s.light.1 - s.dark.1) * vert;
                let cb = s.dark.2 + (s.light.2 - s.dark.2) * vert;

                let is_top = py == top;
                let is_bot = py == bot;

                let (ch, fg) = if top == bot {
                    // Nose tip
                    ('<', Color::Rgb(cr as u8, cg as u8, cb as u8))
                } else if is_top {
                    // Top edge
                    let ch = if s.directional_edges {
                        if slope < -0.3 { '/' } else if slope > 0.3 { '\\' } else { '─' }
                    } else {
                        '-'
                    };
                    (ch, Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8))
                } else if is_bot {
                    // Bottom edge
                    let ch = if s.directional_edges {
                        if slope < -0.3 { '/' } else if slope > 0.3 { '\\' } else { '─' }
                    } else {
                        '-'
                    };
                    (ch, Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8))
                } else {
                    // Interior
                    match s.fill {
                        FillStyle::Tilde => {
                            ('~', Color::Rgb(cr as u8, cg as u8, cb as u8))
                        }
                        FillStyle::Dots => {
                            ('·', Color::Rgb(cr as u8, cg as u8, cb as u8))
                        }
                        FillStyle::Empty => {
                            (' ', Color::Rgb(8, 10, 18))
                        }
                        FillStyle::Gradient => {
                            let ch = if depth > 0.7 {
                                '='
                            } else if depth > 0.3 {
                                '·'
                            } else {
                                ' '
                            };
                            if ch == ' ' {
                                (' ', Color::Rgb(8, 10, 18))
                            } else {
                                (ch, Color::Rgb(cr as u8, cg as u8, cb as u8))
                            }
                        }
                    }
                };
                set(buf, px, py, ch, fg, area);
            }

            // ── Dorsal fin ──
            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * (s.max_half * 1.1);
                let body_top = (cy - half).round() as i32;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '/' } else { '|' };
                    set(
                        buf, px, body_top - h, ch,
                        Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8),
                        area,
                    );
                }
            }

            // ── Pectoral fin ──
            if r > 0.32 && r < 0.46 {
                let fh = (1.0 - ((r - 0.39) / 0.07).powi(2)).max(0.0) * (s.max_half * 0.55);
                let body_bot = (cy + half).round() as i32;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '\\' } else { '|' };
                    set(
                        buf, px, body_bot + h, ch,
                        Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8),
                        area,
                    );
                }
            }

            // ── Tail fork ──
            if r > 0.88 {
                let spread = ((r - 0.88) / 0.12 * (s.max_half * 1.4)) as i32;
                let body_top = (cy - half).round() as i32;
                let body_bot = (cy + half).round() as i32;
                set(buf, px, body_top - spread, '/',
                    Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8), area);
                set(buf, px, body_bot + spread, '\\',
                    Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8), area);
            }
        }

        // ── Eye ──
        let eye_r = 0.12;
        let eye_cy = cy_at(eye_r);
        let eye_x = sx + (s.len as f64 * eye_r).round() as i32;
        let eye_y = (eye_cy - s.max_half * 0.18).round() as i32;
        set(buf, eye_x, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        // ── Gill slits ──
        for i in 0..3 {
            let gr = 0.21 + i as f64 * 0.022;
            let gcy = cy_at(gr);
            let gx = sx + (s.len as f64 * gr).round() as i32;
            let gy = gcy.round() as i32;
            set(buf, gx, gy, ':', Color::Rgb(45, 60, 100), area);
        }

        // Header
        if area.height > 2 && area.width > 20 {
            let hdr = format!(
                "  terminal-zoo  {} ({}/{})    \u{2190}\u{2192} switch  q quit",
                s.name, self.current + 1, SHARKS.len()
            );
            for (i, ch) in hdr.chars().enumerate() {
                if i >= area.width as usize { break; }
                let cell = &mut buf[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(50, 45, 75));
            }
        }
    }
}

fn set(
    buf: &mut ratatui::buffer::Buffer, x: i32, y: i32,
    ch: char, fg: Color, area: ratatui::layout::Rect,
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
    let result = App { current: 0, exit: false, elapsed: 0.0 }.run(terminal);
    ratatui::restore();
    result
}
