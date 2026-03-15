use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Creature trait ─────────────────────────────────────────────────────────

trait Creature {
    fn name(&self) -> &str;
    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64);
}

// ─── Shark (horizontal undulation — body thickness oscillates) ──────────────
// A side-view shark swims by sweeping its tail LEFT and RIGHT.
// We represent this as the body thickness (number of rows) pulsing
// with a wave that travels head→tail.

struct SharkCfg;

impl Creature for SharkCfg {
    fn name(&self) -> &str { "Shark" }

    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64) {
        let len: i32 = 48;
        let base_half: f64 = 2.2; // base half-thickness in rows
        let sx = (area.width as i32 - len) / 2;
        let cy = area.height as f64 / 2.0;

        let k = 2.0 * PI / 38.0; // wavelength ~38 cols
        let omega = 2.0 * PI * 1.3;

        // For each column: body center stays on cy,
        // thickness oscillates to simulate lateral (horizontal) undulation
        for col in 0..len {
            let r = col as f64 / len as f64;
            let px = sx + col;
            if px < 0 || px >= area.width as i32 { continue; }

            // Taper at nose and tail
            let taper = if r < 0.12 { r / 0.12 }
                else if r > 0.82 { (1.0 - r) / 0.18 }
                else { 1.0 };

            // Horizontal undulation: thickness pulsates (head→tail wave)
            let envelope = r * r; // carangiform: tail-heavy
            let wave = (k * col as f64 - omega * t).sin();
            let thickness_mod = 1.0 + wave * 0.25 * envelope;

            let half = base_half * taper * thickness_mod;

            // Slight vertical drift (the tail sweeps, shifting the center a tiny bit)
            let v_drift = wave * 0.3 * envelope;
            let center = cy + v_drift;

            let top_exact = center - half;
            let bot_exact = center + half;
            let top = top_exact.round() as i32;
            let bot = bot_exact.round() as i32;

            // Slope for edge chars (how much the center drifts between columns)
            let r_next = ((col + 1) as f64 / len as f64).min(1.0);
            let env_next = r_next * r_next;
            let wave_next = (k * (col + 1) as f64 - omega * t).sin();
            let drift_next = wave_next * 0.3 * env_next;
            let slope = drift_next - v_drift;

            let (top_ch, bot_ch) = edge_chars(slope);

            // AA hint
            let top_frac = (top_exact - top_exact.floor()).abs();
            let bot_frac = (bot_exact - bot_exact.floor()).abs();
            if top_frac > 0.25 && top_frac < 0.75 {
                let hy = if top_exact < top as f64 { top - 1 } else { top + 1 };
                set_c(buf, px, hy, '·', dim_color((50.0, 70.0, 120.0), 0.4), area);
            }
            if bot_frac > 0.25 && bot_frac < 0.75 {
                let hy = if bot_exact > bot as f64 { bot + 1 } else { bot - 1 };
                set_c(buf, px, hy, '·', dim_color((155.0, 170.0, 190.0), 0.4), area);
            }

            for py in top..=bot {
                let vert = if top == bot { 0.5 }
                    else { (py - top) as f64 / (bot - top) as f64 };
                let depth = (0.5 - (vert - 0.5).abs()) * 2.0;

                // Counter-shading
                let cr = 50.0 + 112.0 * vert;
                let cg = 70.0 + 105.0 * vert;
                let cb = 120.0 + 72.0 * vert;

                // Thickness modulation → brightness pulse (thicker = brighter, like light hitting wider surface)
                let bright = 0.85 + 0.15 * thickness_mod;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb((cr * bright) as u8, (cg * bright) as u8, (cb * bright) as u8))
                } else if py == top {
                    (top_ch, Color::Rgb((50.0 * bright) as u8, (70.0 * bright) as u8, (120.0 * bright) as u8))
                } else if py == bot {
                    (bot_ch, Color::Rgb((155.0 * bright) as u8, (170.0 * bright) as u8, (190.0 * bright) as u8))
                } else if depth > 0.7 {
                    ('=', Color::Rgb((cr * bright) as u8, (cg * bright) as u8, (cb * bright) as u8))
                } else if depth > 0.3 {
                    ('·', Color::Rgb((cr * bright * 0.8) as u8, (cg * bright * 0.8) as u8, (cb * bright * 0.8) as u8))
                } else {
                    (' ', Color::Rgb(8, 10, 18))
                };
                if ch != ' ' { set_c(buf, px, py, ch, fg, area); }
            }

            // Dorsal fin
            if r > 0.28 && r < 0.48 {
                let fh = (1.0 - ((r - 0.38) / 0.10).powi(2)).max(0.0) * 2.5;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '/' } else { '|' };
                    set_c(buf, px, top - h, ch, Color::Rgb(50, 70, 120), area);
                }
            }
            // Pectoral fin
            if r > 0.32 && r < 0.46 {
                let fh = (1.0 - ((r - 0.39) / 0.07).powi(2)).max(0.0) * 1.2;
                for h in 1..=(fh.ceil() as i32) {
                    let ch = if h == fh.ceil() as i32 { '\\' } else { '|' };
                    set_c(buf, px, bot + h, ch, Color::Rgb(150, 165, 185), area);
                }
            }
            // Tail fork
            if r > 0.88 {
                let spread = ((r - 0.88) / 0.12 * 3.0) as i32;
                set_c(buf, px, top - spread, '/', Color::Rgb(50, 70, 120), area);
                set_c(buf, px, bot + spread, '\\', Color::Rgb(145, 160, 182), area);
            }
        }

        // Eye
        let eye_x = sx + (len as f64 * 0.12).round() as i32;
        let eye_y = (cy - 0.3).round() as i32;
        set_c(buf, eye_x, eye_y, 'O', Color::Rgb(230, 235, 245), area);

        // Gills
        for i in 0..3 {
            let gc = (len as f64 * (0.21 + i as f64 * 0.022)).round() as i32;
            set_c(buf, sx + gc, cy.round() as i32, ':', Color::Rgb(45, 60, 100), area);
        }
    }
}

// ─── Eel (vertical undulation — same as old shark logic) ────────────────────
// An eel-like creature seen from the side, body bends up/down.

struct EelCfg;

impl Creature for EelCfg {
    fn name(&self) -> &str { "Eel" }

    fn draw(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect, t: f64) {
        let len: i32 = 55;
        let max_half: f64 = 1.2; // thin, eel-like
        let sx = (area.width as i32 - len) / 2;
        let mid_y = area.height as f64 / 2.0;

        let k = 2.0 * PI / 28.0; // shorter wavelength (more waves visible)
        let omega = 2.0 * PI * 1.8; // slightly faster

        // Anguilliform envelope: whole body participates
        let envelope = |x: f64| -> f64 { 0.3 + 0.7 * x };

        let cy_at = |col: i32| -> f64 {
            let r = col as f64 / len as f64;
            let amp = 3.0 * envelope(r);
            mid_y + (k * col as f64 - omega * t).sin() * amp
        };

        // Trail (faded previous positions)
        for echo in 1..=2 {
            let t_past = t - echo as f64 * 0.05;
            let fade = 0.2 / echo as f64;
            let cy_past = |col: i32| -> f64 {
                let r = col as f64 / len as f64;
                let amp = 3.0 * envelope(r);
                mid_y + (k * col as f64 - omega * t_past).sin() * amp
            };
            for col in 0..len {
                let r = col as f64 / len as f64;
                let cy = cy_past(col);
                let half = if r < 0.08 { r / 0.08 * max_half }
                    else if r > 0.90 { (1.0 - r) / 0.10 * max_half }
                    else { max_half };
                let top = (cy - half).round() as i32;
                let bot = (cy + half).round() as i32;
                let px = sx + col;
                for py in top..=bot {
                    set_c(buf, px, py, '·',
                        Color::Rgb((40.0 * fade) as u8, (90.0 * fade) as u8, (50.0 * fade) as u8),
                        area);
                }
            }
        }

        // Main body
        let mut prev_top: Option<i32> = None;
        let mut prev_bot: Option<i32> = None;
        for col in 0..len {
            let r = col as f64 / len as f64;
            let cy = cy_at(col);
            let half = if r < 0.08 { r / 0.08 * max_half }
                else if r > 0.90 { (1.0 - r) / 0.10 * max_half }
                else { max_half };

            let px = sx + col;
            if px < 0 || px >= area.width as i32 { continue; }

            let top = (cy - half).round() as i32;
            let bot = (cy + half).round() as i32;

            let slope = cy_at(col.min(len - 2) + 1) - cy;
            let (top_ch, bot_ch) = edge_chars(slope);

            // Gap fill
            if let (Some(pt), Some(pb)) = (prev_top, prev_bot) {
                for gy in (top + 1)..(pt.min(top + 5)) {
                    set_c(buf, px - 1, gy, '/', Color::Rgb(30, 70, 40), area);
                }
                for gy in (pt + 1)..(top.min(pt + 5)) {
                    set_c(buf, px, gy, '\\', Color::Rgb(30, 70, 40), area);
                }
                for gy in (pb + 1)..(bot.min(pb + 5)) {
                    set_c(buf, px, gy, '/', Color::Rgb(50, 100, 55), area);
                }
                for gy in (bot + 1)..(pb.min(bot + 5)) {
                    set_c(buf, px - 1, gy, '\\', Color::Rgb(50, 100, 55), area);
                }
            }

            // AA hint
            let top_exact = cy - half;
            let bot_exact = cy + half;
            let top_frac = (top_exact - top_exact.floor()).abs();
            let bot_frac = (bot_exact - bot_exact.floor()).abs();
            if top_frac > 0.25 && top_frac < 0.75 {
                let hy = if top_exact < top as f64 { top - 1 } else { top + 1 };
                set_c(buf, px, hy, '·', Color::Rgb(18, 42, 22), area);
            }
            if bot_frac > 0.25 && bot_frac < 0.75 {
                let hy = if bot_exact > bot as f64 { bot + 1 } else { bot - 1 };
                set_c(buf, px, hy, '·', Color::Rgb(30, 55, 30), area);
            }

            for py in top..=bot {
                let vert = if top == bot { 0.5 }
                    else { (py - top) as f64 / (bot - top) as f64 };

                let cr = 25.0 + 40.0 * vert;
                let cg = 60.0 + 50.0 * vert;
                let cb = 30.0 + 30.0 * vert;

                let (ch, fg) = if top == bot {
                    ('<', Color::Rgb(cr as u8, cg as u8, cb as u8))
                } else if py == top {
                    (top_ch, Color::Rgb(25, 60, 30))
                } else if py == bot {
                    (bot_ch, Color::Rgb(55, 100, 55))
                } else {
                    ('~', Color::Rgb(cr as u8, cg as u8, cb as u8))
                };
                if ch != ' ' { set_c(buf, px, py, ch, fg, area); }
            }

            prev_top = Some(top);
            prev_bot = Some(bot);
        }

        // Eye
        let eye_x = sx + (len as f64 * 0.06).round() as i32;
        let eye_y = cy_at((len as f64 * 0.06) as i32).round() as i32;
        set_c(buf, eye_x, eye_y, 'o', Color::Rgb(180, 200, 160), area);
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn edge_chars(slope: f64) -> (char, char) {
    if slope < -0.4 { ('/', '\\') }
    else if slope < -0.12 { ('\'', '.') }
    else if slope > 0.4 { ('\\', '/') }
    else if slope > 0.12 { ('.', '\'') }
    else { ('\u{2500}', '\u{2500}') } // ─
}

fn dim_color(base: (f64, f64, f64), intensity: f64) -> Color {
    Color::Rgb(
        (base.0 * intensity) as u8,
        (base.1 * intensity) as u8,
        (base.2 * intensity) as u8,
    )
}

fn set_c(buf: &mut ratatui::buffer::Buffer, x: i32, y: i32, ch: char, fg: Color, area: ratatui::layout::Rect) {
    if x >= 0 && y >= 0 && x < area.width as i32 && y < area.height as i32 {
        let cell = &mut buf[(x as u16, y as u16)];
        cell.set_char(ch);
        cell.set_fg(fg);
        cell.set_style(Style::default());
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

const CREATURES: &[&dyn Creature] = &[&SharkCfg, &EelCfg];

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
                                self.current = (self.current + 1) % CREATURES.len();
                            }
                            KeyCode::Left | KeyCode::Char('p') => {
                                self.current = if self.current == 0 {
                                    CREATURES.len() - 1
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

        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(8, 10, 18));
                cell.set_fg(Color::Rgb(8, 10, 18));
            }
        }

        CREATURES[self.current].draw(buf, area, self.elapsed);

        if area.height > 2 && area.width > 20 {
            let hdr = format!(
                "  terminal-zoo  {} ({}/{})    \u{2190}\u{2192} switch  q quit",
                CREATURES[self.current].name(), self.current + 1, CREATURES.len()
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

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App { current: 0, exit: false, elapsed: 0.0 }.run(terminal);
    ratatui::restore();
    result
}
