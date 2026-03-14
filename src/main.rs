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

const TICK_RATE: Duration = Duration::from_millis(33); // ~30fps

// ─── Animal ASCII Art ───────────────────────────────────────────────────────

#[allow(dead_code)]
struct AnimalArt {
    frames: &'static [&'static [&'static str]],
    name: &'static str,
    base_color: (u8, u8, u8),
    accent_color: (u8, u8, u8),
}

const CAT_FRAMES: &[&[&str]] = &[
    &[
        r"    /\_____/\    ",
        r"   /  o   o  \   ",
        r"  ( ==  ^  == )  ",
        r"   )         (   ",
        r"  (           )  ",
        r" ( (  )   (  ) ) ",
        r"(__(__)___(__)__)",
    ],
    &[
        r"    /\_____/\    ",
        r"   /  -   -  \   ",
        r"  ( ==  ^  == )  ",
        r"   )         (   ",
        r"  (           )  ",
        r" ( (  )   (  ) ) ",
        r"(__(__)___(__)__)",
    ],
    &[
        r"    /\_____/\    ",
        r"   /  o   o  \   ",
        r"  ( ==  ^  == )  ",
        r"   )  ~~~~~  (   ",
        r"  (           )  ",
        r" (  (  )  ( )  ) ",
        r"(__(__)__(__)___)",
    ],
    &[
        r"    /\_____/\    ",
        r"   /  o   o  \   ",
        r"  ( ==  ^  == )  ",
        r"   )         (   ",
        r"  (           )  ",
        r" (  ( )   ( )  ) ",
        r"(___(__)_(__)___)",
    ],
];

const DOG_FRAMES: &[&[&str]] = &[
    &[
        r"    __         ",
        r" o-''))_____\\  ",
        r"'--__/ * * * )  ",
        r"   /  U       ) ",
        r"  /|     |     |",
        r" (_|     |     |",
    ],
    &[
        r"    __         ",
        r" o-''))_____\\  ",
        r"'--__/ * * * )  ",
        r"   /  U       ) ",
        r"   |    |    |  ",
        r"  (_|   |   |_) ",
    ],
    &[
        r"     _         ",
        r" o-''))_____\\  ",
        r"'--__/ * * * )  ",
        r"   /  U       ) ",
        r"  /|     |     |",
        r" (_|     |     |",
    ],
    &[
        r"    __         ",
        r" o-'))______\\  ",
        r"'--__/ * * * )  ",
        r"   /  U       ) ",
        r"   |    |    |  ",
        r"  (_|   |   |_) ",
    ],
];

const BIRD_FRAMES: &[&[&str]] = &[
    &[
        r"        .---.    ",
        r"       /     \   ",
        r"      | () () |  ",
        r"       \  ^  /   ",
        r"    ---==/  \==  ",
        r"   /   /|    |\  ",
        r"       ||    ||  ",
    ],
    &[
        r"        .---.    ",
        r"       /     \   ",
        r"      | () () |  ",
        r"       \  ^  /   ",
        r"  \---==/  \==---",
        r"       /|  |\    ",
        r"       ||  ||    ",
    ],
    &[
        r"        .---.    ",
        r"       /     \   ",
        r"      | () () |  ",
        r"       \  ^  /   ",
        r"    ---==/  \==  ",
        r"  \\  /|    |\  \\",
        r"       ||    ||  ",
    ],
    &[
        r"        .---.    ",
        r"       /     \   ",
        r"      | -- -- |  ",
        r"       \  ^  /   ",
        r"    ---==/  \==  ",
        r"   /   /|    |\  ",
        r"       ||    ||  ",
    ],
];

const FISH_FRAMES: &[&[&str]] = &[
    &[
        r"         .----.   ",
        r"        /  oo  \  ",
        r"  |    |        | ",
        r" / \    \      /  ",
        r"|   |    '----'   ",
        r" \ /  ._/|  |\   ",
        r"  |    ~~'~~~~'   ",
    ],
    &[
        r"          .----.  ",
        r"         /  oo  \ ",
        r"  \\    |        |",
        r"  / \    \      / ",
        r" |   |    '----'  ",
        r"  \ /  ._/|  |\  ",
        r"   \\   ~~'~~~~'  ",
    ],
    &[
        r"         .----.   ",
        r"        / oo   \  ",
        r"  |    |        | ",
        r" / \    \      /  ",
        r"|   |    '----'   ",
        r" \ /  ._/|  |\   ",
        r"  |    ~~'~~~~'   ",
    ],
    &[
        r"         .----.   ",
        r"        /  oo  \  ",
        r"  //   |        | ",
        r" / \    \      /  ",
        r"|   |    '----'   ",
        r" \ /  ._/|  |\   ",
        r"  //   ~~'~~~~'   ",
    ],
];

const RABBIT_FRAMES: &[&[&str]] = &[
    &[
        r"   (\   /)   ",
        r"   ( ^.^ )   ",
        r"  o(u   u)o  ",
        r"    |   |    ",
        r"    (( ))    ",
    ],
    &[
        r"   (\ ~ /)   ",
        r"   ( ^.^ )   ",
        r"  o(u   u)o  ",
        r"    |   |    ",
        r"    (( ))    ",
    ],
    &[
        r"   (\   /)   ",
        r"   ( -.- )   ",
        r"  o(u   u)o  ",
        r"   | | | |   ",
        r"    (( ))    ",
    ],
    &[
        r"   (\   /)   ",
        r"   ( o.o )   ",
        r"  o(u   u)o  ",
        r"    |   |    ",
        r"   _(( ))_   ",
    ],
];

const ANIMALS: &[AnimalArt] = &[
    AnimalArt {
        frames: CAT_FRAMES,
        name: "Cat",
        base_color: (255, 150, 50),
        accent_color: (255, 200, 100),
    },
    AnimalArt {
        frames: DOG_FRAMES,
        name: "Dog",
        base_color: (100, 180, 255),
        accent_color: (150, 210, 255),
    },
    AnimalArt {
        frames: BIRD_FRAMES,
        name: "Bird",
        base_color: (100, 255, 150),
        accent_color: (180, 255, 200),
    },
    AnimalArt {
        frames: FISH_FRAMES,
        name: "Fish",
        base_color: (80, 200, 255),
        accent_color: (130, 230, 255),
    },
    AnimalArt {
        frames: RABBIT_FRAMES,
        name: "Rabbit",
        base_color: (255, 130, 200),
        accent_color: (255, 180, 220),
    },
];

// ─── Particle System ────────────────────────────────────────────────────────

#[derive(Clone)]
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
    fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    fn update(&mut self, dt: f64) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.vy += 2.0 * dt; // gravity
        self.life -= dt;
    }

    fn alpha(&self) -> f64 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

// ─── Star Background ────────────────────────────────────────────────────────

struct Star {
    x: f64,
    y: f64,
    brightness: f64,
    twinkle_speed: f64,
    twinkle_phase: f64,
}

// ─── Animal Entity ──────────────────────────────────────────────────────────

struct Animal {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    art_idx: usize,
    frame_idx: usize,
    frame_timer: f64,
    trail: Vec<(f64, f64, f64)>, // x, y, alpha
}

impl Animal {
    fn new(art_idx: usize, x: f64, y: f64) -> Self {
        let mut rng = rand::thread_rng();
        Animal {
            x,
            y,
            vx: rng.gen_range(-6.0..6.0),
            vy: rng.gen_range(-3.0..3.0),
            art_idx,
            frame_idx: 0,
            frame_timer: 0.0,
            trail: Vec::new(),
        }
    }

    fn art(&self) -> &'static AnimalArt {
        &ANIMALS[self.art_idx]
    }

    fn width(&self) -> usize {
        self.art().frames[0][0].len()
    }

    fn height(&self) -> usize {
        self.art().frames[0].len()
    }

    fn update(&mut self, dt: f64, max_x: f64, max_y: f64, particles: &mut Vec<Particle>) {
        let mut rng = rand::thread_rng();

        // Store trail position
        self.trail.push((self.x, self.y, 1.0));
        if self.trail.len() > 8 {
            self.trail.remove(0);
        }
        for t in &mut self.trail {
            t.2 -= dt * 2.0;
        }
        self.trail.retain(|t| t.2 > 0.0);

        self.x += self.vx * dt;
        self.y += self.vy * dt;

        // Add slight wave motion
        self.vy += (self.frame_timer * 2.0).sin() * 0.3 * dt;

        let w = self.width() as f64;
        let h = self.height() as f64;

        // Bounce with particles
        if self.x <= 1.0 {
            self.x = 1.0;
            self.vx = self.vx.abs() + rng.gen_range(0.0..2.0);
            self.spawn_bounce_particles(particles, true);
        } else if self.x + w >= max_x - 1.0 {
            self.x = max_x - w - 1.0;
            self.vx = -self.vx.abs() - rng.gen_range(0.0..2.0);
            self.spawn_bounce_particles(particles, true);
        }

        if self.y <= 3.0 {
            self.y = 3.0;
            self.vy = self.vy.abs() + rng.gen_range(0.0..1.0);
            self.spawn_bounce_particles(particles, false);
        } else if self.y + h >= max_y - 2.0 {
            self.y = max_y - h - 2.0;
            self.vy = -self.vy.abs() - rng.gen_range(0.0..1.0);
            self.spawn_bounce_particles(particles, false);
        }

        self.vx = self.vx.clamp(-10.0, 10.0);
        self.vy = self.vy.clamp(-5.0, 5.0);

        // Animate frames
        self.frame_timer += dt;
        if self.frame_timer >= 0.18 {
            self.frame_timer -= 0.18;
            self.frame_idx = (self.frame_idx + 1) % self.art().frames.len();
        }
    }

    fn spawn_bounce_particles(&self, particles: &mut Vec<Particle>, vertical_wall: bool) {
        let mut rng = rand::thread_rng();
        let art = self.art();
        let sparkle_chars = ['*', '+', '.', '·', '✦', '✧', '⚬'];

        for _ in 0..rng.gen_range(4..10) {
            let (vx, vy) = if vertical_wall {
                (rng.gen_range(-3.0..3.0), rng.gen_range(-4.0..4.0))
            } else {
                (rng.gen_range(-4.0..4.0), rng.gen_range(-3.0..3.0))
            };
            let life = rng.gen_range(0.4..1.2);
            particles.push(Particle {
                x: self.x + self.width() as f64 / 2.0,
                y: self.y + self.height() as f64 / 2.0,
                vx,
                vy,
                life,
                max_life: life,
                ch: sparkle_chars[rng.gen_range(0..sparkle_chars.len())],
                color: art.accent_color,
            });
        }
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

struct App {
    animals: Vec<Animal>,
    particles: Vec<Particle>,
    stars: Vec<Star>,
    exit: bool,
    tick: u64,
    elapsed: f64,
}

impl App {
    fn new(cols: u16, rows: u16) -> Self {
        let mut rng = rand::thread_rng();

        let animals: Vec<Animal> = (0..4)
            .map(|i| {
                let art_idx = i % ANIMALS.len();
                let x = rng.gen_range(3.0..(cols as f64 - 20.0).max(5.0));
                let y = rng.gen_range(4.0..(rows as f64 - 12.0).max(6.0));
                Animal::new(art_idx, x, y)
            })
            .collect();

        let stars: Vec<Star> = (0..60)
            .map(|_| Star {
                x: rng.gen_range(0.0..cols as f64),
                y: rng.gen_range(0.0..rows as f64),
                brightness: rng.gen_range(0.3..1.0),
                twinkle_speed: rng.gen_range(0.5..3.0),
                twinkle_phase: rng.gen_range(0.0..std::f64::consts::TAU),
            })
            .collect();

        App {
            animals,
            particles: Vec::new(),
            stars,
            exit: false,
            tick: 0,
            elapsed: 0.0,
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
                        self.handle_key(key.code)?;
                    }
                }
            }

            if last_tick.elapsed() >= TICK_RATE {
                let dt = last_tick.elapsed().as_secs_f64();
                self.on_tick(dt);
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
            KeyCode::Char('a') | KeyCode::Char(' ') => self.add_animal(),
            KeyCode::Char('d') | KeyCode::Backspace => {
                if self.animals.len() > 1 {
                    self.animals.pop();
                }
            }
            KeyCode::Char('r') => self.add_random_burst(),
            _ => {}
        }
        Ok(())
    }

    fn add_animal(&mut self) {
        let mut rng = rand::thread_rng();
        let art_idx = rng.gen_range(0..ANIMALS.len());
        let x = rng.gen_range(5.0..40.0);
        let y = rng.gen_range(5.0..20.0);
        self.animals.push(Animal::new(art_idx, x, y));
    }

    fn add_random_burst(&mut self) {
        let mut rng = rand::thread_rng();
        let sparkle_chars = ['*', '+', '·', '✦', '✧', '⚬', '◦', '•'];
        let colors = [
            (255, 100, 100),
            (100, 255, 100),
            (100, 100, 255),
            (255, 255, 100),
            (255, 100, 255),
            (100, 255, 255),
        ];

        let cx = rng.gen_range(10.0..60.0);
        let cy = rng.gen_range(5.0..20.0);

        for _ in 0..30 {
            let angle: f64 = rng.gen_range(0.0..std::f64::consts::TAU);
            let speed = rng.gen_range(3.0..12.0);
            let life = rng.gen_range(0.8..2.0);
            self.particles.push(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed * 0.5,
                life,
                max_life: life,
                ch: sparkle_chars[rng.gen_range(0..sparkle_chars.len())],
                color: colors[rng.gen_range(0..colors.len())],
            });
        }
    }

    fn on_tick(&mut self, dt: f64) {
        self.tick += 1;
        self.elapsed += dt;

        // Get terminal size for bounds
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));

        // Update animals
        let mut new_particles = Vec::new();
        for animal in &mut self.animals {
            animal.update(dt, cols as f64, rows as f64, &mut new_particles);
        }
        self.particles.extend(new_particles);

        // Update particles
        for p in &mut self.particles {
            p.update(dt);
        }
        self.particles.retain(|p| p.is_alive());

        // Ambient particles from animals
        let mut rng = rand::thread_rng();
        if self.tick % 5 == 0 {
            for animal in &self.animals {
                if rng.gen_bool(0.3) {
                    let art = animal.art();
                    let life = rng.gen_range(0.5..1.5);
                    self.particles.push(Particle {
                        x: animal.x + rng.gen_range(0.0..animal.width() as f64),
                        y: animal.y + animal.height() as f64,
                        vx: rng.gen_range(-1.0..1.0),
                        vy: rng.gen_range(-2.0..-0.5),
                        life,
                        max_life: life,
                        ch: ['·', '✧', '.'][rng.gen_range(0..3)],
                        color: art.base_color,
                    });
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Background gradient
        let bg_buf = frame.buffer_mut();
        for y in 0..area.height {
            let ratio = y as f64 / area.height as f64;
            let r = lerp_u8(10, 25, ratio);
            let g = lerp_u8(5, 15, ratio);
            let b = lerp_u8(30, 50, ratio);
            for x in 0..area.width {
                let cell = &mut bg_buf[(x, y)];
                cell.set_bg(Color::Rgb(r, g, b));
            }
        }

        // Stars with twinkling
        for star in &self.stars {
            let sx = star.x as u16;
            let sy = star.y as u16;
            if sx < area.width && sy < area.height {
                let twinkle =
                    ((self.elapsed * star.twinkle_speed + star.twinkle_phase).sin() + 1.0) / 2.0;
                let b = (star.brightness * twinkle * 255.0) as u8;
                let ch = if twinkle > 0.7 { '✦' } else if twinkle > 0.4 { '·' } else { '.' };
                let cell = &mut bg_buf[(sx, sy)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(b, b, (b as f64 * 0.8) as u8));
            }
        }

        // Animal trails (ghost effect)
        for animal in &self.animals {
            let art = animal.art();
            for (tx, ty, alpha) in &animal.trail {
                if *alpha <= 0.0 {
                    continue;
                }
                let frame_data = &art.frames[animal.frame_idx];
                let ix = tx.round() as u16;
                let iy = ty.round() as u16;
                let a = (*alpha * 0.3).clamp(0.0, 1.0);

                for (row, line) in frame_data.iter().enumerate() {
                    for (col, ch) in line.chars().enumerate() {
                        if ch == ' ' {
                            continue;
                        }
                        let px = ix + col as u16;
                        let py = iy + row as u16;
                        if px < area.width && py < area.height {
                            let r = (art.base_color.0 as f64 * a * 0.5) as u8;
                            let g = (art.base_color.1 as f64 * a * 0.5) as u8;
                            let b = (art.base_color.2 as f64 * a * 0.5) as u8;
                            let cell = &mut bg_buf[(px, py)];
                            cell.set_char(ch);
                            cell.set_fg(Color::Rgb(r, g, b));
                        }
                    }
                }
            }
        }

        // Animals with gradient coloring
        for animal in &self.animals {
            let art = animal.art();
            let frame_data = &art.frames[animal.frame_idx];
            let ix = animal.x.round() as u16;
            let iy = animal.y.round() as u16;

            for (row, line) in frame_data.iter().enumerate() {
                let row_ratio = row as f64 / frame_data.len().max(1) as f64;

                for (col, ch) in line.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    let px = ix + col as u16;
                    let py = iy + row as u16;
                    if px >= area.width || py >= area.height {
                        continue;
                    }

                    // Gradient from base to accent color, top to bottom
                    let r = lerp_u8(art.base_color.0, art.accent_color.0, row_ratio);
                    let g = lerp_u8(art.base_color.1, art.accent_color.1, row_ratio);
                    let b = lerp_u8(art.base_color.2, art.accent_color.2, row_ratio);

                    // Subtle shimmer
                    let shimmer = ((self.elapsed * 3.0 + col as f64 * 0.5).sin() * 15.0) as i16;
                    let r = (r as i16 + shimmer).clamp(0, 255) as u8;
                    let g = (g as i16 + shimmer).clamp(0, 255) as u8;
                    let b = (b as i16 + shimmer).clamp(0, 255) as u8;

                    let cell = &mut bg_buf[(px, py)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(r, g, b));
                    cell.set_style(Style::default().add_modifier(Modifier::BOLD));
                }
            }
        }

        // Particles
        for p in &self.particles {
            let px = p.x.round() as u16;
            let py = p.y.round() as u16;
            if px < area.width && py < area.height {
                let a = p.alpha();
                let r = (p.color.0 as f64 * a) as u8;
                let g = (p.color.1 as f64 * a) as u8;
                let b = (p.color.2 as f64 * a) as u8;
                let cell = &mut bg_buf[(px, py)];
                cell.set_char(p.ch);
                cell.set_fg(Color::Rgb(r, g, b));
            }
        }

        // Header bar
        let header_area = Rect::new(0, 0, area.width, 3);
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Rgb(80, 80, 120)));

        let title_spans = vec![
            Span::styled("  ✦ ", Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled(
                "terminal-zoo",
                Style::default()
                    .fg(Color::Rgb(180, 140, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ✦  ", Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled(
                format!("Animals: {}", self.animals.len()),
                Style::default().fg(Color::Rgb(100, 200, 150)),
            ),
            Span::styled("  │  ", Style::default().fg(Color::Rgb(60, 60, 90))),
            Span::styled(
                format!("Particles: {}", self.particles.len()),
                Style::default().fg(Color::Rgb(200, 150, 100)),
            ),
        ];
        let title_line = Line::from(title_spans);

        let controls_spans = vec![
            Span::styled("  [", Style::default().fg(Color::Rgb(60, 60, 90))),
            Span::styled(
                "A/Space",
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] Add  [", Style::default().fg(Color::Rgb(60, 60, 90))),
            Span::styled(
                "D/BS",
                Style::default()
                    .fg(Color::Rgb(255, 130, 130))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] Remove  [", Style::default().fg(Color::Rgb(60, 60, 90))),
            Span::styled(
                "R",
                Style::default()
                    .fg(Color::Rgb(255, 200, 100))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] Burst  [", Style::default().fg(Color::Rgb(60, 60, 90))),
            Span::styled(
                "Q",
                Style::default()
                    .fg(Color::Rgb(255, 100, 100))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] Quit", Style::default().fg(Color::Rgb(60, 60, 90))),
        ];
        let controls_line = Line::from(controls_spans);

        let header = Paragraph::new(vec![title_line, controls_line]).block(header_block);
        frame.render_widget(header, header_area);

        // Footer with animated gradient bar
        let footer_y = area.height.saturating_sub(1);
        for x in 0..area.width {
            let hue_offset = (self.elapsed * 30.0 + x as f64 * 2.0) % 360.0;
            let (r, g, b) = hsl_to_rgb(hue_offset, 0.7, 0.4);
            let cell = &mut frame.buffer_mut()[(x, footer_y)];
            cell.set_char('▀');
            cell.set_fg(Color::Rgb(r, g, b));
        }
    }
}

// ─── Color Helpers ──────────────────────────────────────────────────────────

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f64 + (b as f64 - a as f64) * t) as u8
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - (h2 % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h2 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
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
