use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

struct Shark {
    name: &'static str,
    len: i32,
    a_max: f64,       // max amplitude in cells (at tail)
    lambda: f64,      // wavelength in columns
    freq: f64,        // wave frequency in Hz
    envelope: fn(f64) -> f64,
    aa: bool,         // sub-cell anti-aliased edges
    trail: bool,      // motion trail (ghost of previous frames)
    gap_fill: bool,   // fill gaps when edge jumps > 1 row
    flex: f64,        // body narrows at curves
    dark: (f64, f64, f64),
    light: (f64, f64, f64),
}

// Amplitude envelopes
fn linear_env(x: f64) -> f64 {
    0.1 + 0.9 * x
}
fn carangiform_env(x: f64) -> f64 {
    // Videler & Hess (1984): head barely moves, tail sweeps wide
    (0.02 - 0.08 * x + 0.16 * x * x) / 0.10
}
fn stiff_env(x: f64) -> f64 {
    // Very tail-heavy, like a tuna
    x * x * x
}

const SHARKS: &[Shark] = &[
    // 1: Base — directional edges, reference
    Shark {
        name: "Base",
        len: 40,
        a_max: 2.5,
        lambda: 35.0,
        freq: 1.5,
        envelope: linear_env,
        aa: false,
        trail: false,
        gap_fill: false,
        flex: 0.0,
        dark: (55.0, 75.0, 125.0),
        light: (165.0, 175.0, 185.0),
    },
    // 2: Anti-aliased — sub-cell edge rendering
    Shark {
        name: "AA Edges",
        len: 45,
        a_max: 3.0,
        lambda: 35.0,
        freq: 1.3,
        envelope: carangiform_env,
        aa: true,
        trail: false,
        gap_fill: true,
        flex: 0.0,
        dark: (50.0, 72.0, 122.0),
        light: (162.0, 175.0, 190.0),
    },
    // 3: Biomech — proper fish physics envelope + flex
    Shark {
        name: "Biomech",
        len: 48,
        a_max: 3.0,
        lambda: 38.0,
        freq: 1.2,
        envelope: carangiform_env,
        aa: true,
        trail: false,
        gap_fill: true,
        flex: 0.12,
        dark: (48.0, 70.0, 120.0),
        light: (160.0, 174.0, 192.0),
    },
    // 4: Trail — motion blur with ghost echoes
    Shark {
        name: "Trail",
        len: 45,
        a_max: 2.8,
        lambda: 36.0,
        freq: 1.4,
        envelope: carangiform_env,
        aa: true,
        trail: true,
        gap_fill: true,
        flex: 0.08,
        dark: (52.0, 74.0, 124.0),
        light: (160.0, 174.0, 190.0),
    },
    // 5: Full — everything combined, tuned
    Shark {
        name: "Full",
        len: 50,
        a_max: 3.2,
        lambda: 40.0,
        freq: 1.2,
        envelope: stiff_env,
        aa: true,
        trail: true,
        gap_fill: true,
        flex: 0.10,
        dark: (45.0, 68.0, 118.0),
        light: (158.0, 172.0, 194.0),
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
        let k = 2.0 * PI / s.lambda;
        let omega = 2.0 * PI * s.freq;

        // Motion trail: render previous frames first (underneath)
        if s.trail {
            for i in 1..=3 {
                let t_past = t - i as f64 * 0.04;
                let fade = 0.25 / i as f64;
                self.draw_shark_body(buf, area, s, sx, mid_y, k, omega, t_past, fade);
            }
        }

        // Current frame
        self.draw_shark_body(buf, area, s, sx, mid_y, k, omega, t, 1.0);

        // Eye
        let eye_r = 0.12;
        let eye_cy = shark_cy(s, mid_y, k, omega, t, eye_r);
        let eye_x = sx + (s.len as f64 * eye_r).round() as i32;
        let half_at_eye = shark_half(s, eye_r, k, omega, t);
        let eye_y = (eye_cy - half_at_eye * 0.2).round() as i32;
        set(buf, eye_x, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        // Gills
        for i in 0..3 {
            let gr = 0.21 + i as f64 * 0.022;
            let gx = sx + (s.len as f64 * gr).round() as i32;
            let gy = shark_cy(s, mid_y, k, omega, t, gr).round() as i32;
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

    fn draw_shark_body(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        s: &Shark,
        sx: i32,
        mid_y: f64,
        k: f64,
        omega: f64,
        t: f64,
        intensity: f64,
    ) {
        let mut prev_top: Option<i32> = None;
        let mut prev_bot: Option<i32> = None;

        for col in 0..s.len {
            let r = col as f64 / s.len as f64;
            let cy = shark_cy(s, mid_y, k, omega, t, r);
            let half = shark_half(s, r, k, omega, t);

            let px = sx + col;
            if px < 0 || px >= area.width as i32 {
                continue;
            }

            // Slope for directional edges
            let r_next = ((col + 1) as f64 / s.len as f64).min(1.0);
            let slope = shark_cy(s, mid_y, k, omega, t, r_next) - cy;

            // Exact floating-point edge positions
            let top_exact = cy - half;
            let bot_exact = cy + half;
            let top = top_exact.round() as i32;
            let bot = bot_exact.round() as i32;

            // Edge characters based on slope
            let (top_ch, bot_ch) = edge_chars(slope);

            // Gap filling: connect to previous column
            if s.gap_fill {
                if let (Some(pt), Some(pb)) = (prev_top, prev_bot) {
                    // Fill vertical gaps in top edge
                    if top < pt - 1 {
                        for gy in (top + 1)..pt {
                            set_fade(buf, px - 1, gy, '/', s.dark, intensity * 0.6, area);
                        }
                    } else if top > pt + 1 {
                        for gy in (pt + 1)..top {
                            set_fade(buf, px, gy, '\\', s.dark, intensity * 0.6, area);
                        }
                    }
                    // Fill vertical gaps in bottom edge
                    if bot < pb - 1 {
                        for gy in (bot + 1)..pb {
                            set_fade(buf, px - 1, gy, '\\', s.light, intensity * 0.6, area);
                        }
                    } else if bot > pb + 1 {
                        for gy in (pb + 1)..bot {
                            set_fade(buf, px, gy, '/', s.light, intensity * 0.6, area);
                        }
                    }
                }
            }

            // Sub-cell anti-aliased edges
            if s.aa {
                let top_frac = (top_exact - top_exact.floor()).abs();
                let bot_frac = (bot_exact - bot_exact.floor()).abs();

                // Hint cell above top edge (the edge is drifting toward it)
                if top_frac > 0.25 && top_frac < 0.75 {
                    let hint_y = if top_exact < top as f64 { top - 1 } else { top + 1 };
                    set_fade(buf, px, hint_y, '·', s.dark, intensity * 0.4, area);
                }
                // Hint cell below bottom edge
                if bot_frac > 0.25 && bot_frac < 0.75 {
                    let hint_y = if bot_exact > bot as f64 { bot + 1 } else { bot - 1 };
                    set_fade(buf, px, hint_y, '·', s.light, intensity * 0.4, area);
                }
            }

            // Body rendering
            for py in top..=bot {
                let vert = if top == bot { 0.5 }
                    else { (py - top) as f64 / (bot - top) as f64 };
                let depth = if top == bot { 0.0 }
                    else { (0.5 - (vert - 0.5).abs()) * 2.0 };

                let cr = s.dark.0 + (s.light.0 - s.dark.0) * vert;
                let cg = s.dark.1 + (s.light.1 - s.dark.1) * vert;
                let cb = s.dark.2 + (s.light.2 - s.dark.2) * vert;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb((cr * intensity) as u8, (cg * intensity) as u8, (cb * intensity) as u8))
                } else if py == top {
                    (top_ch, Color::Rgb(
                        (s.dark.0 * intensity) as u8,
                        (s.dark.1 * intensity) as u8,
                        (s.dark.2 * intensity) as u8))
                } else if py == bot {
                    (bot_ch, Color::Rgb(
                        (s.light.0 * intensity) as u8,
                        (s.light.1 * intensity) as u8,
                        (s.light.2 * intensity) as u8))
                } else {
                    let ch = if depth > 0.7 { '=' }
                        else if depth > 0.3 { '·' }
                        else { ' ' };
                    if ch == ' ' {
                        (' ', Color::Rgb(8, 10, 18))
                    } else {
                        (ch, Color::Rgb((cr * intensity) as u8, (cg * intensity) as u8, (cb * intensity) as u8))
                    }
                };

                if ch != ' ' || intensity >= 1.0 {
                    set(buf, px, py, ch, fg, area);
                }
            }

            // Dorsal fin
            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * (s.a_max * 0.9);
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '/' } else { '|' };
                    set_fade(buf, px, top - h, ch, s.dark, intensity, area);
                }
            }

            // Pectoral fin
            if r > 0.32 && r < 0.46 {
                let fh = (1.0 - ((r - 0.39) / 0.07).powi(2)).max(0.0) * (s.a_max * 0.45);
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '\\' } else { '|' };
                    set_fade(buf, px, bot + h, ch, s.light, intensity, area);
                }
            }

            // Tail fork
            if r > 0.88 {
                let spread = ((r - 0.88) / 0.12 * (s.a_max * 1.2)) as i32;
                set_fade(buf, px, top - spread, '/', s.dark, intensity, area);
                set_fade(buf, px, bot + spread, '\\', s.light, intensity, area);
            }

            prev_top = Some(top);
            prev_bot = Some(bot);
        }
    }
}

fn shark_cy(s: &Shark, mid_y: f64, k: f64, omega: f64, t: f64, r: f64) -> f64 {
    let col = r * s.len as f64;
    let amp = s.a_max * (s.envelope)(r);
    mid_y + (k * col - omega * t).sin() * amp
}

fn shark_half(s: &Shark, r: f64, k: f64, omega: f64, t: f64) -> f64 {
    let base = if r < 0.12 {
        r / 0.12 * (s.a_max * 0.8)
    } else if r > 0.82 {
        (1.0 - r) / 0.18 * (s.a_max * 0.8)
    } else {
        s.a_max * 0.8
    };

    if s.flex > 0.0 {
        let col = r * s.len as f64;
        let curve = (k * col - omega * t).sin().abs();
        base * (1.0 - curve * s.flex)
    } else {
        base
    }
}

fn edge_chars(slope: f64) -> (char, char) {
    if slope < -0.4 {
        ('/', '\\')
    } else if slope < -0.12 {
        ('\'', '.')
    } else if slope > 0.4 {
        ('\\', '/')
    } else if slope > 0.12 {
        ('.', '\'')
    } else {
        ('\u{2500}', '\u{2500}') // ─
    }
}

fn set(buf: &mut ratatui::buffer::Buffer, x: i32, y: i32, ch: char, fg: Color, area: ratatui::layout::Rect) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_style(Style::default());
    }
}

fn set_fade(buf: &mut ratatui::buffer::Buffer, x: i32, y: i32, ch: char, base: (f64, f64, f64), intensity: f64, area: ratatui::layout::Rect) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        // Don't overwrite a "real" character with a faint one
        if intensity < 1.0 && cell.symbol() != " " {
            return;
        }
        cell.set_char(ch);
        cell.set_fg(Color::Rgb(
            (base.0 * intensity) as u8,
            (base.1 * intensity) as u8,
            (base.2 * intensity) as u8,
        ));
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
