mod sdf;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Style},
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16); // ~60fps

type FeaturesFn = fn(f64, f64, f64) -> Option<(char, Color)>;

struct Animal {
    name: &'static str,
    sdf_fn: fn(f64, f64, f64) -> f64,
    features_fn: FeaturesFn,
    color_body: (f64, f64, f64),
    color_aura: (f64, f64, f64),
}

const ANIMALS: &[Animal] = &[
    Animal {
        name: "Ghost",
        sdf_fn: sdf::ghost,
        features_fn: sdf::ghost_features,
        color_body: (0.82, 0.82, 0.95),
        color_aura: (0.25, 0.40, 1.0),
    },
    Animal {
        name: "Shark",
        sdf_fn: sdf::shark,
        features_fn: sdf::shark_features,
        color_body: (0.45, 0.55, 0.72),
        color_aura: (0.20, 0.30, 0.55),
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
                                self.current = (self.current + 1) % ANIMALS.len();
                            }
                            KeyCode::Left | KeyCode::Char('p') => {
                                self.current = if self.current == 0 {
                                    ANIMALS.len() - 1
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
        let w = area.width as f64;
        let h = area.height as f64;
        let aspect = 0.5;
        let animal = &ANIMALS[self.current];
        let (cr, cg, cb) = animal.color_body;
        let (ar, ag, ab) = animal.color_aura;

        for y in 0..area.height {
            for x in 0..area.width {
                let nx = (x as f64 - w / 2.0) * aspect / (h / 2.0);
                let ny = (y as f64 - h / 2.0) / (h / 2.0);

                // Features layer first (eyes, gills, etc.)
                if let Some((fch, fcolor)) = (animal.features_fn)(nx, ny, t) {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(fch);
                    cell.set_fg(fcolor);
                    cell.set_bg(Color::Rgb(10, 8, 16));
                    cell.set_style(Style::default());
                    continue;
                }

                let d = (animal.sdf_fn)(nx, ny, t);

                let (ch, fg) = if d < -0.03 {
                    let brightness = (-d * 8.0).min(1.0);
                    let ch = sdf::density_char(brightness);
                    let v = 0.7 + 0.3 * brightness;
                    (
                        ch,
                        Color::Rgb(
                            (cr * v * 255.0) as u8,
                            (cg * v * 255.0) as u8,
                            (cb * v * 255.0) as u8,
                        ),
                    )
                } else if d < 0.0 {
                    (
                        '%',
                        Color::Rgb(
                            (cr * 0.65 * 255.0) as u8,
                            (cg * 0.65 * 255.0) as u8,
                            (cb * 0.65 * 255.0) as u8,
                        ),
                    )
                } else if d < 0.06 {
                    let fade = 1.0 - d / 0.06;
                    let shimmer = 0.7 + 0.3 * (t * 3.0 + nx * 5.0 + ny * 3.0).sin();
                    let v = fade * shimmer * 0.7;
                    let ch = if fade > 0.5 { '=' } else { '+' };
                    (
                        ch,
                        Color::Rgb(
                            (ar * v * 255.0) as u8,
                            (ag * v * 255.0) as u8,
                            (ab * v * 255.0) as u8,
                        ),
                    )
                } else if d < 0.15 {
                    let fade = 1.0 - (d - 0.06) / 0.09;
                    let shimmer = 0.5 + 0.5 * (t * 2.0 + nx * 8.0 + ny * 4.0).cos();
                    let v = fade * shimmer * 0.4;
                    if v > 0.04 {
                        let ch = if fade > 0.4 { '+' } else { '·' };
                        (
                            ch,
                            Color::Rgb(
                                (ar * v * 255.0) as u8,
                                (ag * v * 255.0) as u8,
                                (ab * v * 255.0) as u8,
                            ),
                        )
                    } else {
                        (' ', Color::Rgb(10, 8, 16))
                    }
                } else {
                    (' ', Color::Rgb(10, 8, 16))
                };

                let cell = &mut buf[(x, y)];
                cell.set_char(ch);
                cell.set_fg(fg);
                cell.set_bg(Color::Rgb(10, 8, 16));
                cell.set_style(Style::default());
            }
        }

        // Header
        if area.height > 2 && area.width > 30 {
            let hdr = format!(
                "  terminal-zoo  {}    \u{2190} \u{2192} switch  q quit",
                animal.name
            );
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
        current: 0,
        exit: false,
        elapsed: 0.0,
    }
    .run(terminal);
    ratatui::restore();
    result
}
