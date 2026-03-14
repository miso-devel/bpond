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

// ─── Large ASCII Art Animals ────────────────────────────────────────────────
// Ghostty-style: big, detailed, lots of texture characters

const CAT_ART: &[&str] = &[
    r#"                          %%%@@@%%%                          "#,
    r#"                     %%@@@@@@@@@@@@@@@%%                     "#,
    r#"                  %@@@@%##***!!!!!***##%@@@@%                "#,
    r#"               %@@@@#*!!::::........::::!!*#@@@@%            "#,
    r#"             %@@@#*!::....              ....::!*#@@@%        "#,
    r#"            @@@@!::..     /\        /\     ..::!@@@@         "#,
    r#"          %@@@*!:.      /    \    /    \      .:!*@@@%       "#,
    r#"         %@@#!:.       /  @@  \  /  @@  \       .:!#@@%     "#,
    r#"        %@@#!:.       |  @@@@  ||  @@@@  |       .:!#@@%    "#,
    r#"       %@@#!:.        |  @@@@  ||  @@@@  |        .:!#@@%   "#,
    r#"       @@#!:.          \  @@  /  \  @@  /          .:!#@@   "#,
    r#"      %@@!:.            \    / () \    /            .:!@@%  "#,
    r#"      %@@!:.             \  / \  / \  /             .:!@@%  "#,
    r#"      @@#!:.              \/   \/   \/              .:!#@@  "#,
    r#"      @@#!:.         .                   .          .:!#@@  "#,
    r#"      @@#!:.       .   .               .   .        .:!#@@  "#,
    r#"      %@@!:.      .     '·._________·.'     .      .:!@@%  "#,
    r#"      %@@!:.       ·.                     .·        .:!@@%  "#,
    r#"       @@#!:.        '·.               .·'         .:!#@@   "#,
    r#"       %@@#!:.          '''·.......·'''           .:!#@@%   "#,
    r#"        %@@#!:.                                  .:!#@@%    "#,
    r#"         %@@#!:.            |     |             .:!#@@%     "#,
    r#"          %@@@*!:.          |     |            .:!*@@@%     "#,
    r#"            @@@@!::..      |       |       ..::!@@@@        "#,
    r#"             %@@@#*!::...  |       |   ...::!*#@@@%         "#,
    r#"               %@@@@#*!!::::|     |::::!!*#@@@@%            "#,
    r#"                  %@@@@%##**|     |**##%@@@@%               "#,
    r#"                     %%@@@@@|     |@@@@@%%                  "#,
    r#"                          %%|     |%%                       "#,
    r#"                        ====='   '=====                     "#,
];

// "Breathing" variant: slightly different body shape
const CAT_ART_2: &[&str] = &[
    r#"                          %%%@@@%%%                          "#,
    r#"                     %%@@@@@@@@@@@@@@@%%                     "#,
    r#"                  %@@@@%##***!!!!!***##%@@@@%                "#,
    r#"               %@@@@#*!!::::........::::!!*#@@@@%            "#,
    r#"             %@@@#*!::....              ....::!*#@@@%        "#,
    r#"            @@@@!::..     /\        /\     ..::!@@@@         "#,
    r#"          %@@@*!:.      /    \    /    \      .:!*@@@%       "#,
    r#"         %@@#!:.       /  --  \  /  --  \       .:!#@@%     "#,
    r#"        %@@#!:.       |  ----  ||  ----  |       .:!#@@%    "#,
    r#"       %@@#!:.        |  ----  ||  ----  |        .:!#@@%   "#,
    r#"       @@#!:.          \  --  /  \  --  /          .:!#@@   "#,
    r#"      %@@!:.            \    / () \    /            .:!@@%  "#,
    r#"      %@@!:.             \  / \  / \  /             .:!@@%  "#,
    r#"      @@#!:.              \/   \/   \/              .:!#@@  "#,
    r#"      @@#!:.         .                   .          .:!#@@  "#,
    r#"      @@#!:.       .   .               .   .        .:!#@@  "#,
    r#"      %@@!:.      .     '·._________·.'     .      .:!@@%  "#,
    r#"      %@@!:.        ·.                   .·         .:!@@%  "#,
    r#"       @@#!:.         '·.             .·'          .:!#@@   "#,
    r#"       %@@#!:.           '''·.....·'''            .:!#@@%   "#,
    r#"        %@@#!:.                                  .:!#@@%    "#,
    r#"         %@@#!:.            |     |             .:!#@@%     "#,
    r#"          %@@@*!:.          |     |            .:!*@@@%     "#,
    r#"            @@@@!::..      |       |       ..::!@@@@        "#,
    r#"             %@@@#*!::...  |       |   ...::!*#@@@%         "#,
    r#"               %@@@@#*!!::::|     |::::!!*#@@@@%            "#,
    r#"                  %@@@@%##**|     |**##%@@@@%               "#,
    r#"                     %%@@@@@|     |@@@@@%%                  "#,
    r#"                          %%|     |%%                       "#,
    r#"                        ====='   '=====                     "#,
];

const DOG_ART: &[&str] = &[
    r#"                 %%#####%%                                   "#,
    r#"              %##*!!::::!!*##%                                "#,
    r#"            %#*!::........::!*#%        %%###%%               "#,
    r#"           %#!::..        ..::!#%    %##*!!*##%              "#,
    r#"          %#!:.    @@    @@   .:!#%%#*!::..::!*#%            "#,
    r#"          #!:.    @@@@  @@@@   .:!#*!:.    .:!*#             "#,
    r#"         %#!:.    @@@@  @@@@    .:!:.       .:!#%            "#,
    r#"         %#!:.     @@    @@      .:..        .:!#%           "#,
    r#"         %#!:.                    .:..        .:!#%          "#,
    r#"          #!:.       \__/         .:..         .:!#          "#,
    r#"          #!:.      /    \        .::..        .:!#          "#,
    r#"          %#!:.    | (  ) |       .::..       .:!#%          "#,
    r#"           #!:.     \    /        ..::..      .:!#           "#,
    r#"           %#!:.     '=='          ..::..    .:!#%           "#,
    r#"            %#!:.                   .::..   .:!#%            "#,
    r#"             %#*!::..                .::.  .:*#%             "#,
    r#"              %##*!!::....           ..::.:!*##%             "#,
    r#"                 %%##**!!::::.......::::!!##%%               "#,
    r#"                     %%%###***!!!!!***###%%%                 "#,
    r#"                          %%%%%@@@%%%%%                      "#,
    r#"                          |   |   |   |                      "#,
    r#"                          |   |   |   |                      "#,
    r#"                         ='   '= ='   '=                    "#,
];

const DOG_ART_2: &[&str] = &[
    r#"                 %%#####%%                                   "#,
    r#"              %##*!!::::!!*##%                                "#,
    r#"            %#*!::........::!*#%        %%###%%               "#,
    r#"           %#!::..        ..::!#%    %##*!!*##%              "#,
    r#"          %#!:.    --    --   .:!#%%#*!::..::!*#%            "#,
    r#"          #!:.    ----  ----   .:!#*!:.    .:!*#             "#,
    r#"         %#!:.    ----  ----    .:!:.       .:!#%            "#,
    r#"         %#!:.     --    --      .:..        .:!#%           "#,
    r#"         %#!:.                    .:..        .:!#%          "#,
    r#"          #!:.       \__/         .:..         .:!#          "#,
    r#"          #!:.      /    \        .::..        .:!#          "#,
    r#"          %#!:.    | (  ) |       .::..       .:!#%          "#,
    r#"           #!:.     \    /        ..::..      .:!#           "#,
    r#"           %#!:.     '=='          ..::..    .:!#%           "#,
    r#"            %#!:.                   .::..   .:!#%            "#,
    r#"             %#*!::..                .::.  .:*#%             "#,
    r#"              %##*!!::....           ..::.:!*##%             "#,
    r#"                 %%##**!!::::.......::::!!##%%               "#,
    r#"                     %%%###***!!!!!***###%%%                 "#,
    r#"                          %%%%%@@@%%%%%                      "#,
    r#"                           |  |   |  |                       "#,
    r#"                           |  |   |  |                       "#,
    r#"                          ='  '= ='  '=                     "#,
];

const FISH_ART: &[&str] = &[
    r#"                                                             "#,
    r#"                       .::::::::::..                          "#,
    r#"                   .:::::::::::::::::::.                      "#,
    r#"                .:::::::*##%%@@%%##*:::::::.                  "#,
    r#"             .::::::*#%@@@@@@@@@@@@@@%#*::::::.               "#,
    r#"           .:::::**%@@@@@%##****##%@@@@@%**:::::.             "#,
    r#"    %%    .::::**%@@@@#*!!::....::!!*#@@@@%**:::::.           "#,
    r#"   %@@% .::::*#@@@#*!::..  @@@@  ..::!*#@@@#*::::.          "#,
    r#"  %@@@@%::::*#@@#*!:.     @@@@@@    .:!*#@@#*:::::.          "#,
    r#"  %@@@@@@::*#@@#!:.       @@@@@@     .:!#@@#*::::::          "#,
    r#"   %@@@@@@*#@@#!:.         @@@@       .:!#@@#*:::::          "#,
    r#"    %%@@@@@@@#!:.                      .:!#@@@@@:::          "#,
    r#"      %@@@@@@!:.    ·..          ..·    .:!@@@@@@:.          "#,
    r#"    %%@@@@@@@#!:.      ''·....·''       .:!#@@@@@:::        "#,
    r#"   %@@@@@@*#@@#!:.                     .:!#@@#*::::::        "#,
    r#"  %@@@@@@::*#@@#!:.                   .:!#@@#*:::::::        "#,
    r#"  %@@@@%::::*#@@#*!::..           ..::!*#@@#*:::::.          "#,
    r#"   %@@% .::::*#@@@#*!::...   ...::!*#@@@#*::::.             "#,
    r#"    %%    .::::**%@@@@#*!!:::::::!!*#@@@@%**:::::.           "#,
    r#"           .:::::**%@@@@@%##****##%@@@@@%**:::::.            "#,
    r#"             .::::::*#%@@@@@@@@@@@@@@%#*::::::.              "#,
    r#"                .:::::::*##%%@@%%##*:::::::.                 "#,
    r#"                   .:::::::::::::::::::.                     "#,
    r#"                       .::::::::::..                         "#,
    r#"                                                             "#,
];

const FISH_ART_2: &[&str] = &[
    r#"                                                             "#,
    r#"                       .::::::::::..                          "#,
    r#"                   .:::::::::::::::::::.                      "#,
    r#"                .:::::::*##%%@@%%##*:::::::.                  "#,
    r#"             .::::::*#%@@@@@@@@@@@@@@%#*::::::.               "#,
    r#"           .:::::**%@@@@@%##****##%@@@@@%**:::::.             "#,
    r#"    %%    .::::**%@@@@#*!!::....::!!*#@@@@%**:::::.           "#,
    r#"   %@@% .::::*#@@@#*!::..  ----  ..::!*#@@@#*::::.          "#,
    r#"  %@@@@%::::*#@@#*!:.     ------    .:!*#@@#*:::::.          "#,
    r#"  %@@@@@@::*#@@#!:.       ------     .:!#@@#*::::::          "#,
    r#"   %@@@@@@*#@@#!:.         ----       .:!#@@#*:::::          "#,
    r#"    %%@@@@@@@#!:.                      .:!#@@@@@:::          "#,
    r#"      %@@@@@@!:.    ·..          ..·    .:!@@@@@@:.          "#,
    r#"    %%@@@@@@@#!:.      ''·....·''       .:!#@@@@@:::        "#,
    r#"   %@@@@@@*#@@#!:.                     .:!#@@#*::::::        "#,
    r#"  %@@@@@@::*#@@#!:.                   .:!#@@#*:::::::        "#,
    r#"  %@@@@%::::*#@@#*!::..           ..::!*#@@#*:::::.          "#,
    r#"   %@@% .::::*#@@@#*!::...   ...::!*#@@@#*::::.             "#,
    r#"    %%    .::::**%@@@@#*!!:::::::!!*#@@@@%**:::::.           "#,
    r#"           .:::::**%@@@@@%##****##%@@@@@%**:::::.            "#,
    r#"             .::::::*#%@@@@@@@@@@@@@@%#*::::::.              "#,
    r#"                .:::::::*##%%@@%%##*:::::::.                 "#,
    r#"                   .:::::::::::::::::::.                     "#,
    r#"                       .::::::::::..                         "#,
    r#"                                                             "#,
];

const RABBIT_ART: &[&str] = &[
    r#"                   %%##%%     %%##%%                         "#,
    r#"                 %#*!!*#%   %#*!!*#%                         "#,
    r#"                %#!:..:#%   %#:..:#%                         "#,
    r#"                %#!:..:#%   %#:...:!#%                       "#,
    r#"                %#!:..:#%   %#:...:!#%                       "#,
    r#"                 %#!:.:#%   %#:...:!#%                       "#,
    r#"                  %#*!::#%%%#::...:!#%                       "#,
    r#"                   %##*:::::::..:!*##%                       "#,
    r#"                 %##*!::........::!*##%                      "#,
    r#"               %#*!::..          ..::!*#%                    "#,
    r#"              %#!::..  @@      @@  ..::!#%                   "#,
    r#"             %#!:.    @@@@    @@@@    .:!#%                  "#,
    r#"             %#!:.    @@@@    @@@@    .:!#%                  "#,
    r#"             %#!:.     @@      @@     .:!#%                  "#,
    r#"              #!:.                     .:!#                  "#,
    r#"              #!:.        \  /         .:!#                  "#,
    r#"              %#!:.       '=='        .:!#%                  "#,
    r#"               %#!:.    ·......·     .:!#%                   "#,
    r#"                %#*!::..          ..::!*#%                   "#,
    r#"                  %##*!!::......::!!*##%                     "#,
    r#"                     %%%##******##%%%                        "#,
    r#"                        |  |  |  |                           "#,
    r#"                        |  |  |  |                           "#,
    r#"                       ='  '=='  '=                          "#,
];

const RABBIT_ART_2: &[&str] = &[
    r#"                   %%##%%     %%##%%                         "#,
    r#"                 %#*!!*#%   %#*!!*#%                         "#,
    r#"                %#!:..:#%   %#:..:#%                         "#,
    r#"                %#!:..:#%   %#:...:!#%                       "#,
    r#"                %#!:..:#%   %#:...:!#%                       "#,
    r#"                 %#!:.:#%   %#:...:!#%                       "#,
    r#"                  %#*!::#%%%#::...:!#%                       "#,
    r#"                   %##*:::::::..:!*##%                       "#,
    r#"                 %##*!::........::!*##%                      "#,
    r#"               %#*!::..          ..::!*#%                    "#,
    r#"              %#!::..  --      --  ..::!#%                   "#,
    r#"             %#!:.    ----    ----    .:!#%                  "#,
    r#"             %#!:.    ----    ----    .:!#%                  "#,
    r#"             %#!:.     --      --     .:!#%                  "#,
    r#"              #!:.                     .:!#                  "#,
    r#"              #!:.        \  /         .:!#                  "#,
    r#"              %#!:.       '=='        .:!#%                  "#,
    r#"               %#!:.    ·......·     .:!#%                   "#,
    r#"                %#*!::..          ..::!*#%                   "#,
    r#"                  %##*!!::......::!!*##%                     "#,
    r#"                     %%%##******##%%%                        "#,
    r#"                         | |  | |                            "#,
    r#"                         | |  | |                            "#,
    r#"                        =' '==' '=                           "#,
];

struct AnimalDef {
    art_a: &'static [&'static str],
    art_b: &'static [&'static str],
    name: &'static str,
    // Gradient colors: top, middle, bottom
    color_top: (u8, u8, u8),
    color_mid: (u8, u8, u8),
    color_bot: (u8, u8, u8),
}

const ANIMAL_DEFS: &[AnimalDef] = &[
    AnimalDef {
        art_a: CAT_ART,
        art_b: CAT_ART_2,
        name: "Cat",
        color_top: (255, 180, 80),
        color_mid: (255, 130, 60),
        color_bot: (200, 100, 40),
    },
    AnimalDef {
        art_a: DOG_ART,
        art_b: DOG_ART_2,
        name: "Dog",
        color_top: (120, 200, 255),
        color_mid: (80, 160, 255),
        color_bot: (50, 120, 200),
    },
    AnimalDef {
        art_a: FISH_ART,
        art_b: FISH_ART_2,
        name: "Fish",
        color_top: (80, 220, 255),
        color_mid: (60, 180, 230),
        color_bot: (40, 140, 200),
    },
    AnimalDef {
        art_a: RABBIT_ART,
        art_b: RABBIT_ART_2,
        name: "Rabbit",
        color_top: (255, 160, 220),
        color_mid: (255, 120, 190),
        color_bot: (220, 90, 160),
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
        self.vy += 1.5 * dt; // gentle gravity
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

// ─── App ────────────────────────────────────────────────────────────────────

struct App {
    current_animal: usize,
    particles: Vec<Particle>,
    stars: Vec<Star>,
    exit: bool,
    elapsed: f64,
    // Smooth position: base position + sine offsets
    base_x: f64,
    base_y: f64,
}

impl App {
    fn new(cols: u16, rows: u16) -> Self {
        let mut rng = rand::thread_rng();

        let stars: Vec<Star> = (0..80)
            .map(|_| Star {
                x: rng.gen_range(0.0..cols as f64),
                y: rng.gen_range(0.0..rows as f64),
                brightness: rng.gen_range(0.2..1.0),
                twinkle_speed: rng.gen_range(0.3..2.5),
                twinkle_phase: rng.gen_range(0.0..std::f64::consts::TAU),
            })
            .collect();

        // Position animal in the lower-right area, like a screen companion
        let art = ANIMAL_DEFS[0].art_a;
        let art_w = art[0].len() as f64;
        let art_h = art.len() as f64;
        let base_x = (cols as f64 - art_w) / 2.0;
        let base_y = (rows as f64 - art_h) / 2.0;

        App {
            current_animal: 0,
            particles: Vec::new(),
            stars,
            exit: false,
            elapsed: 0.0,
            base_x,
            base_y,
        }
    }

    fn current_def(&self) -> &'static AnimalDef {
        &ANIMAL_DEFS[self.current_animal]
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

            if last_tick.elapsed() >= TICK_RATE {
                let dt = last_tick.elapsed().as_secs_f64();
                self.on_tick(dt);
                last_tick = Instant::now();
            }
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

    fn recenter(&mut self) {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        let art = self.current_def().art_a;
        let art_w = art[0].len() as f64;
        let art_h = art.len() as f64;
        self.base_x = (cols as f64 - art_w) / 2.0;
        self.base_y = (rows as f64 - art_h) / 2.0;
    }

    fn on_tick(&mut self, dt: f64) {
        self.elapsed += dt;

        // Update particles
        for p in &mut self.particles {
            p.update(dt);
        }
        self.particles.retain(|p| p.is_alive());

        // Ambient floating particles around the animal
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.15) {
            let def = self.current_def();
            let art = def.art_a;
            let (dx, dy) = self.smooth_offset();
            let ax = self.base_x + dx;
            let ay = self.base_y + dy;
            let art_w = art[0].len() as f64;
            let art_h = art.len() as f64;

            let life = rng.gen_range(1.0..3.0);
            let sparkle_chars = ['·', '✧', '✦', '*', '°', '•', '∘'];
            self.particles.push(Particle {
                x: ax + rng.gen_range(-3.0..art_w + 3.0),
                y: ay + rng.gen_range(-2.0..art_h + 2.0),
                vx: rng.gen_range(-0.5..0.5),
                vy: rng.gen_range(-1.5..-0.3),
                life,
                max_life: life,
                ch: sparkle_chars[rng.gen_range(0..sparkle_chars.len())],
                color: def.color_mid,
            });
        }
    }

    /// Smooth sine-based floating offset — the "ぬるぬる" motion
    fn smooth_offset(&self) -> (f64, f64) {
        let t = self.elapsed;
        // Combine multiple sine waves for organic, non-repetitive motion
        let dx = (t * 0.4).sin() * 3.0
            + (t * 0.7 + 1.0).sin() * 1.5
            + (t * 0.15).sin() * 5.0;
        let dy = (t * 0.3).sin() * 2.0
            + (t * 0.55 + 2.0).sin() * 1.0
            + (t * 0.12 + 0.5).sin() * 3.0;
        (dx, dy)
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // ─── Background gradient ────────────────────────────────
        for y in 0..area.height {
            let ratio = y as f64 / area.height as f64;
            let r = lerp_u8(8, 18, ratio);
            let g = lerp_u8(4, 12, ratio);
            let b = lerp_u8(20, 40, ratio);
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(r, g, b));
            }
        }

        // ─── Twinkling stars ────────────────────────────────────
        for star in &self.stars {
            let sx = star.x as u16;
            let sy = star.y as u16;
            if sx < area.width && sy < area.height {
                let twinkle =
                    ((self.elapsed * star.twinkle_speed + star.twinkle_phase).sin() + 1.0) / 2.0;
                let brightness = (star.brightness * twinkle * 200.0) as u8;
                let ch = if twinkle > 0.8 {
                    '✦'
                } else if twinkle > 0.5 {
                    '·'
                } else if twinkle > 0.2 {
                    '.'
                } else {
                    ' '
                };
                if ch != ' ' {
                    let cell = &mut buf[(sx, sy)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(
                        brightness,
                        brightness,
                        (brightness as f64 * 0.7) as u8,
                    ));
                }
            }
        }

        // ─── Particles (behind animal) ──────────────────────────
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

        // ─── Main animal with smooth offset ─────────────────────
        let def = self.current_def();
        let (dx, dy) = self.smooth_offset();

        // Choose frame based on breathing cycle (~3s period)
        let breath_phase = ((self.elapsed * 0.35).sin() + 1.0) / 2.0;
        let art = if breath_phase > 0.5 {
            def.art_a
        } else {
            def.art_b
        };

        let art_h = art.len();
        let ax = (self.base_x + dx).round() as i32;
        let ay = (self.base_y + dy).round() as i32;

        for (row, line) in art.iter().enumerate() {
            let row_ratio = row as f64 / art_h.max(1) as f64;

            // Per-row wave offset for a subtle undulation effect
            let wave = ((self.elapsed * 1.2 + row as f64 * 0.3).sin() * 0.8) as i32;

            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }
                let px = ax + col as i32 + wave;
                let py = ay + row as i32;
                if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 {
                    continue;
                }
                let px = px as u16;
                let py = py as u16;

                // 3-stop gradient: top → mid → bottom
                let (r, g, b) = if row_ratio < 0.5 {
                    let t = row_ratio * 2.0;
                    (
                        lerp_u8(def.color_top.0, def.color_mid.0, t),
                        lerp_u8(def.color_top.1, def.color_mid.1, t),
                        lerp_u8(def.color_top.2, def.color_mid.2, t),
                    )
                } else {
                    let t = (row_ratio - 0.5) * 2.0;
                    (
                        lerp_u8(def.color_mid.0, def.color_bot.0, t),
                        lerp_u8(def.color_mid.1, def.color_bot.1, t),
                        lerp_u8(def.color_mid.2, def.color_bot.2, t),
                    )
                };

                // Character-based brightness: dense chars are brighter
                let char_weight = match ch {
                    '@' => 1.0,
                    '%' | '#' => 0.85,
                    '*' => 0.7,
                    '!' => 0.55,
                    ':' => 0.4,
                    '.' | '\'' | '·' => 0.25,
                    '=' | '|' | '/' | '\\' => 0.9,
                    '(' | ')' | '{' | '}' | '[' | ']' => 0.8,
                    '-' => 0.6,
                    _ => 0.65,
                };

                // Subtle pulse glow
                let pulse = ((self.elapsed * 1.5 + row as f64 * 0.1).sin() * 0.08 + 1.0) as f64;
                let intensity = char_weight * pulse;

                let fr = (r as f64 * intensity).min(255.0) as u8;
                let fg = (g as f64 * intensity).min(255.0) as u8;
                let fb = (b as f64 * intensity).min(255.0) as u8;

                let cell = &mut buf[(px, py)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb(fr, fg, fb));
                cell.set_style(Style::default().add_modifier(Modifier::BOLD));
            }
        }

        // ─── Header ────────────────────────────────────────────
        let header_area = Rect::new(0, 0, area.width, 3);
        let header_block = Block::default()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Rgb(40, 40, 70)));

        let title_spans = vec![
            Span::styled("  ✦ ", Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled(
                "terminal-zoo",
                Style::default()
                    .fg(Color::Rgb(180, 140, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ✦  ", Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled(
                def.name,
                Style::default()
                    .fg(Color::Rgb(def.color_mid.0, def.color_mid.1, def.color_mid.2))
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        let controls_spans = vec![
            Span::styled("  [", Style::default().fg(Color::Rgb(50, 50, 80))),
            Span::styled(
                "←/→",
                Style::default()
                    .fg(Color::Rgb(100, 200, 255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "] Switch Animal  [",
                Style::default().fg(Color::Rgb(50, 50, 80)),
            ),
            Span::styled(
                "Q",
                Style::default()
                    .fg(Color::Rgb(255, 100, 100))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("] Quit", Style::default().fg(Color::Rgb(50, 50, 80))),
        ];

        let header = Paragraph::new(vec![Line::from(title_spans), Line::from(controls_spans)])
            .block(header_block);
        frame.render_widget(header, header_area);

        // ─── Footer gradient bar ───────────────────────────────
        let footer_y = area.height.saturating_sub(1);
        for x in 0..area.width {
            let hue = (self.elapsed * 20.0 + x as f64 * 1.5) % 360.0;
            let (r, g, b) = hsl_to_rgb(hue, 0.6, 0.35);
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
