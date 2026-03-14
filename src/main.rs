use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{self, ClearType},
};
use rand::Rng;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const FPS: u64 = 30;
const FRAME_DURATION: Duration = Duration::from_millis(1000 / FPS);

#[derive(Clone)]
struct AnimalFrames {
    frames: Vec<Vec<&'static str>>,
    width: u16,
    height: u16,
}

impl AnimalFrames {
    fn cat() -> Self {
        let frames = vec![
            vec![
                r"  /\_/\  ",
                r" ( o.o ) ",
                r"  > ^ <  ",
                r" /|   |\ ",
                r"(_|   |_)",
            ],
            vec![
                r"  /\_/\  ",
                r" ( -.- ) ",
                r"  > ^ <  ",
                r" /|   |\ ",
                r"(_|   |_)",
            ],
            vec![
                r"  /\_/\  ",
                r" ( o.o ) ",
                r"  > ^ <  ",
                r"  ||  |\ ",
                r" (_| |_) ",
            ],
            vec![
                r"  /\_/\  ",
                r" ( o.o ) ",
                r"  > ^ <  ",
                r" /|  ||  ",
                r" (_||_)  ",
            ],
        ];
        AnimalFrames {
            frames,
            width: 9,
            height: 5,
        }
    }

    fn dog() -> Self {
        let frames = vec![
            vec![
                r" |\_/|   ",
                r" |q p|  /",
                r" ( 0 )\/ ",
                r" /   \   ",
                r" |__ |   ",
            ],
            vec![
                r" |\_/|   ",
                r" |q p|  /",
                r" ( 0 )\/ ",
                r"  / \    ",
                r"  |__|   ",
            ],
            vec![
                r"  \_/|   ",
                r" |q p|  /",
                r" ( 0 )\/ ",
                r" /   \   ",
                r" |__ |   ",
            ],
            vec![
                r" |\_/    ",
                r" |q p|  /",
                r" ( 0 )\/ ",
                r"  / \    ",
                r"  |__|   ",
            ],
        ];
        AnimalFrames {
            frames,
            width: 10,
            height: 5,
        }
    }

    fn bird() -> Self {
        let frames = vec![
            vec![
                r"   __     ",
                r" <(o )___ ",
                r"  ( ._> / ",
                r"   `---'  ",
            ],
            vec![
                r"   __     ",
                r" <(o )___ ",
                r"  ( ._> / ",
                r"   `---'  ",
            ],
            vec![
                r"     _    ",
                r" \  (o)__ ",
                r"  (  _> / ",
                r"   `---'  ",
            ],
            vec![
                r"          ",
                r" _/(o)___ ",
                r"  ( ._> / ",
                r"   `---'  ",
            ],
        ];
        AnimalFrames {
            frames,
            width: 10,
            height: 4,
        }
    }

    fn fish() -> Self {
        let frames = vec![
            vec![r" ><(((o> "],
            vec![r" ><((o>  "],
            vec![r" ><(o>   "],
            vec![r" ><((o>  "],
        ];
        AnimalFrames {
            frames,
            width: 9,
            height: 1,
        }
    }

    fn rabbit() -> Self {
        let frames = vec![
            vec![
                r" (\(\    ",
                r" ( -.-)  ",
                r" o_('')(')",
            ],
            vec![
                r" (\(\    ",
                r" ( o.o)  ",
                r" o_('')(')",
            ],
            vec![
                r"  /)/)   ",
                r" ( -.-)  ",
                r" o_('')(')",
            ],
            vec![
                r"  /)/)   ",
                r" ( o.o)  ",
                r" o_('')(')",
            ],
        ];
        AnimalFrames {
            frames,
            width: 10,
            height: 3,
        }
    }
}

struct Animal {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    frames: AnimalFrames,
    frame_idx: usize,
    frame_timer: f64,
}

impl Animal {
    fn new(frames: AnimalFrames, x: f64, y: f64) -> Self {
        let mut rng = rand::thread_rng();
        Animal {
            x,
            y,
            vx: rng.gen_range(-8.0..8.0),
            vy: rng.gen_range(-4.0..4.0),
            frames,
            frame_idx: 0,
            frame_timer: 0.0,
        }
    }

    fn update(&mut self, dt: f64, max_x: f64, max_y: f64) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        let mut rng = rand::thread_rng();

        if self.x <= 0.0 {
            self.x = 0.0;
            self.vx = self.vx.abs() + rng.gen_range(-1.0..1.0);
        } else if self.x + self.frames.width as f64 >= max_x {
            self.x = max_x - self.frames.width as f64;
            self.vx = -self.vx.abs() + rng.gen_range(-1.0..1.0);
        }

        if self.y <= 1.0 {
            self.y = 1.0;
            self.vy = self.vy.abs() + rng.gen_range(-0.5..0.5);
        } else if self.y + self.frames.height as f64 >= max_y {
            self.y = max_y - self.frames.height as f64;
            self.vy = -self.vy.abs() + rng.gen_range(-0.5..0.5);
        }

        self.vx = self.vx.clamp(-12.0, 12.0);
        self.vy = self.vy.clamp(-6.0, 6.0);

        self.frame_timer += dt;
        if self.frame_timer >= 0.2 {
            self.frame_timer = 0.0;
            self.frame_idx = (self.frame_idx + 1) % self.frames.frames.len();
        }
    }

    fn draw(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        let frame = &self.frames.frames[self.frame_idx];
        let ix = self.x.round() as u16;
        let iy = self.y.round() as u16;

        for (i, line) in frame.iter().enumerate() {
            execute!(stdout, cursor::MoveTo(ix, iy + i as u16), Print(line))?;
        }
        Ok(())
    }
}

fn random_animal(rng: &mut rand::rngs::ThreadRng, cols: u16, rows: u16) -> Animal {
    let choices: Vec<AnimalFrames> = vec![
        AnimalFrames::cat(),
        AnimalFrames::dog(),
        AnimalFrames::bird(),
        AnimalFrames::fish(),
        AnimalFrames::rabbit(),
    ];
    let idx = rng.gen_range(0..choices.len());
    let frames = choices[idx].clone();
    let x = rng.gen_range(2.0..(cols as f64 - frames.width as f64 - 2.0).max(3.0));
    let y = rng.gen_range(2.0..(rows as f64 - frames.height as f64 - 2.0).max(3.0));
    Animal::new(frames, x, y)
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        cursor::Hide,
        terminal::Clear(ClearType::All)
    )?;

    let (cols, rows) = terminal::size()?;
    let mut rng = rand::thread_rng();

    let mut animals: Vec<Animal> = (0..5)
        .map(|_| random_animal(&mut rng, cols, rows))
        .collect();

    let mut last_frame = Instant::now();

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_frame).as_secs_f64();
        last_frame = now;

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('a') => {
                        let (cols, rows) = terminal::size()?;
                        animals.push(random_animal(&mut rng, cols, rows));
                    }
                    KeyCode::Char('d') => {
                        if animals.len() > 1 {
                            animals.pop();
                        }
                    }
                    _ => {}
                }
            }
        }

        let (cols, rows) = terminal::size()?;

        for animal in &mut animals {
            animal.update(dt, cols as f64, rows as f64);
        }

        execute!(stdout, terminal::Clear(ClearType::All))?;

        let header = format!(
            " terminal-zoo | Animals: {} | [A]dd [D]elete [Q]uit",
            animals.len()
        );
        execute!(stdout, cursor::MoveTo(0, 0), Print(&header))?;

        for animal in &animals {
            animal.draw(&mut stdout)?;
        }

        stdout.flush()?;

        let elapsed = now.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
    }

    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
