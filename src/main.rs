use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// Block characters for high-resolution density rendering
const BLOCKS: &[char] = &[' ', '░', '▒', '▓', '█'];

struct Shark {
    name: &'static str,
    len: i32,
    max_half: f64,
    wave_k: f64,
    wave_speed: f64,
    amp_head: f64,
    amp_tail: f64,
    use_blocks: bool,
    dark: (f64, f64, f64),
    light: (f64, f64, f64),
}

const SHARKS: &[Shark] = &[
    // 1: Classic — ASCII, counter-shaded
    Shark {
        name: "Classic",
        len: 40,
        max_half: 2.0,
        wave_k: 7.0,
        wave_speed: 3.5,
        amp_head: 0.3,
        amp_tail: 1.7,
        use_blocks: false,
        dark: (55.0, 75.0, 125.0),
        light: (165.0, 175.0, 185.0),
    },
    // 2: Block — large, block characters, smooth edges
    Shark {
        name: "Block",
        len: 55,
        max_half: 3.5,
        wave_k: 5.0,
        wave_speed: 2.5,
        amp_head: 0.15,
        amp_tail: 1.0,
        use_blocks: true,
        dark: (40.0, 65.0, 120.0),
        light: (160.0, 175.0, 200.0),
    },
    // 3: Block small — compact block shark, faster
    Shark {
        name: "Block Mini",
        len: 35,
        max_half: 2.5,
        wave_k: 6.5,
        wave_speed: 3.5,
        amp_head: 0.2,
        amp_tail: 1.3,
        use_blocks: true,
        dark: (50.0, 80.0, 140.0),
        light: (150.0, 170.0, 200.0),
    },
    // 4: Block XL — very large, slow, majestic
    Shark {
        name: "Block XL",
        len: 70,
        max_half: 4.5,
        wave_k: 4.0,
        wave_speed: 1.8,
        amp_head: 0.1,
        amp_tail: 0.8,
        use_blocks: true,
        dark: (35.0, 60.0, 110.0),
        light: (155.0, 170.0, 195.0),
    },
    // 5: Block Neon — bright cyan/magenta
    Shark {
        name: "Block Neon",
        len: 50,
        max_half: 3.0,
        wave_k: 5.5,
        wave_speed: 3.0,
        amp_head: 0.2,
        amp_tail: 1.2,
        use_blocks: true,
        dark: (20.0, 140.0, 200.0),
        light: (200.0, 100.0, 220.0),
    },
    // 6: Block Deep — dark tones, subtle
    Shark {
        name: "Block Deep",
        len: 50,
        max_half: 3.0,
        wave_k: 5.0,
        wave_speed: 2.2,
        amp_head: 0.12,
        amp_tail: 0.9,
        use_blocks: true,
        dark: (20.0, 35.0, 70.0),
        light: (80.0, 100.0, 140.0),
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

        // Clear
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

        if s.use_blocks {
            self.draw_block_shark(buf, area, s, sx, &cy_at);
        } else {
            self.draw_ascii_shark(buf, area, s, sx, &cy_at);
        }

        // Header
        if area.height > 2 && area.width > 20 {
            let hdr = format!(
                "  terminal-zoo  {} ({}/{})    \u{2190} \u{2192} switch  q quit",
                s.name,
                self.current + 1,
                SHARKS.len()
            );
            for (i, ch) in hdr.chars().enumerate() {
                if i >= area.width as usize {
                    break;
                }
                let cell = &mut buf[(i as u16, 0)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(50, 45, 75));
            }
        }
    }

    /// Block-character shark: uses ░▒▓█ for smooth sub-cell edges
    fn draw_block_shark(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        s: &Shark,
        sx: i32,
        cy_at: &dyn Fn(f64) -> f64,
    ) {
        for col in 0..s.len {
            let r = col as f64 / s.len as f64;
            let cy = cy_at(r);

            // Body half-thickness (smooth f64, NOT rounded yet)
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

            // Scan a range of rows around the centerline
            let scan_top = (cy - half - 1.5).floor() as i32;
            let scan_bot = (cy + half + 1.5).ceil() as i32;

            for py in scan_top..=scan_bot {
                if py < 0 || py >= area.height as i32 {
                    continue;
                }

                // Signed distance from this cell center to the body edge
                // negative = inside, positive = outside
                let dist_from_center = (py as f64 - cy).abs();
                let edge_dist = half - dist_from_center; // positive = inside

                // Map edge_dist to block character (sub-cell anti-aliasing)
                if edge_dist < -0.5 {
                    continue; // fully outside
                }

                let density = if edge_dist > 0.5 {
                    1.0 // fully inside
                } else {
                    (edge_dist + 0.5).clamp(0.0, 1.0) // edge: 0..1
                };

                let block_idx = (density * (BLOCKS.len() - 1) as f64).round() as usize;
                let ch = BLOCKS[block_idx.min(BLOCKS.len() - 1)];
                if ch == ' ' {
                    continue;
                }

                // Counter-shading based on vertical position within body
                let vert = ((py as f64 - (cy - half)) / (2.0 * half)).clamp(0.0, 1.0);
                let cr = s.dark.0 + (s.light.0 - s.dark.0) * vert;
                let cg = s.dark.1 + (s.light.1 - s.dark.1) * vert;
                let cb = s.dark.2 + (s.light.2 - s.dark.2) * vert;

                // Dim edge characters slightly
                let edge_dim = 0.6 + 0.4 * density;
                let fr = (cr * edge_dim) as u8;
                let fg = (cg * edge_dim) as u8;
                let fb = (cb * edge_dim) as u8;

                set(buf, px, py, ch, Color::Rgb(fr, fg, fb), area);
            }

            // ── Dorsal fin ──
            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * (s.max_half * 1.0);
                let body_top = (cy - half).round() as i32;
                for h in 1..=(fh.ceil() as i32) {
                    let fin_density = if h as f64 > fh - 0.5 {
                        (fh - h as f64 + 0.5).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let bi = (fin_density * (BLOCKS.len() - 1) as f64).round() as usize;
                    let ch = BLOCKS[bi.min(BLOCKS.len() - 1)];
                    if ch != ' ' {
                        set(
                            buf,
                            px,
                            body_top - h,
                            ch,
                            Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8),
                            area,
                        );
                    }
                }
            }

            // ── Pectoral fin ──
            if r > 0.32 && r < 0.46 {
                let fh = (1.0 - ((r - 0.39) / 0.07).powi(2)).max(0.0) * (s.max_half * 0.5);
                let body_bot = (cy + half).round() as i32;
                for h in 1..=(fh.ceil() as i32) {
                    let fin_density = if h as f64 > fh - 0.5 {
                        (fh - h as f64 + 0.5).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let bi = (fin_density * (BLOCKS.len() - 1) as f64).round() as usize;
                    let ch = BLOCKS[bi.min(BLOCKS.len() - 1)];
                    if ch != ' ' {
                        set(
                            buf,
                            px,
                            body_bot + h,
                            ch,
                            Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8),
                            area,
                        );
                    }
                }
            }

            // ── Tail fork ──
            if r > 0.88 {
                let spread = (r - 0.88) / 0.12 * (s.max_half * 1.5);
                let body_top = (cy - half).round() as i32;
                let body_bot = (cy + half).round() as i32;
                let ti = (spread.fract() * (BLOCKS.len() - 1) as f64).round() as usize;
                let ch = BLOCKS[ti.min(BLOCKS.len() - 1)];
                if ch != ' ' {
                    set(
                        buf,
                        px,
                        body_top - spread as i32,
                        ch,
                        Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8),
                        area,
                    );
                    set(
                        buf,
                        px,
                        body_bot + spread as i32,
                        ch,
                        Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8),
                        area,
                    );
                }
            }
        }

        // ── Eye ──
        let eye_r = 0.12;
        let eye_cy = cy_at(eye_r);
        let eye_x = sx + (s.len as f64 * eye_r).round() as i32;
        let eye_y = (eye_cy - s.max_half * 0.15).round() as i32;
        set(buf, eye_x, eye_y, '●', Color::Rgb(20, 25, 40), area);

        // ── Gill slits ──
        for i in 0..3 {
            let gr = 0.20 + i as f64 * 0.02;
            let gcy = cy_at(gr);
            let gx = sx + (s.len as f64 * gr).round() as i32;
            let gy = gcy.round() as i32;
            set(buf, gx, gy, '┊', Color::Rgb(40, 55, 90), area);
        }
    }

    /// Original ASCII shark (variant 1)
    fn draw_ascii_shark(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        s: &Shark,
        sx: i32,
        cy_at: &dyn Fn(f64) -> f64,
    ) {
        for col in 0..s.len {
            let r = col as f64 / s.len as f64;
            let cy = cy_at(r);

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

                let cr = s.dark.0 + (s.light.0 - s.dark.0) * vert;
                let cg = s.dark.1 + (s.light.1 - s.dark.1) * vert;
                let cb = s.dark.2 + (s.light.2 - s.dark.2) * vert;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb(cr as u8, cg as u8, cb as u8))
                } else if py == top {
                    ('-', Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8))
                } else if py == bot {
                    ('-', Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8))
                } else {
                    ('~', Color::Rgb(cr as u8, cg as u8, cb as u8))
                };
                set(buf, px, py, ch, fg, area);
            }

            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * 2.5;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '/' } else { '|' };
                    set(
                        buf,
                        px,
                        top - h,
                        ch,
                        Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8),
                        area,
                    );
                }
            }

            if r > 0.32 && r < 0.46 {
                let fh = (1.0 - ((r - 0.39) / 0.07).powi(2)).max(0.0) * 1.5;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '\\' } else { '|' };
                    set(
                        buf,
                        px,
                        bot + h,
                        ch,
                        Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8),
                        area,
                    );
                }
            }

            if r > 0.88 {
                let spread = ((r - 0.88) / 0.12 * 3.0) as i32;
                set(
                    buf,
                    px,
                    top - spread,
                    '/',
                    Color::Rgb(s.dark.0 as u8, s.dark.1 as u8, s.dark.2 as u8),
                    area,
                );
                set(
                    buf,
                    px,
                    bot + spread,
                    '\\',
                    Color::Rgb(s.light.0 as u8, s.light.1 as u8, s.light.2 as u8),
                    area,
                );
            }
        }

        let eye_r = 0.13;
        let eye_cy = cy_at(eye_r);
        let eye_x = sx + (s.len as f64 * eye_r).round() as i32;
        let eye_y = (eye_cy - 0.4).round() as i32;
        set(buf, eye_x, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        for i in 0..3 {
            let gr = 0.22 + i as f64 * 0.025;
            let gcy = cy_at(gr);
            let gx = sx + (s.len as f64 * gr).round() as i32;
            let gy = gcy.round() as i32;
            set(buf, gx, gy, ':', Color::Rgb(60, 80, 120), area);
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
        current: 0,
        exit: false,
        elapsed: 0.0,
    }
    .run(terminal);
    ratatui::restore();
    result
}
