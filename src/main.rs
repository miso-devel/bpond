use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::style::{Color, Style};
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_millis(16);

// ─── Shark sprite (8 frames, shared by all variants) ────────────────────────
const FRAMES: &[&[&str]] = &[
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######...........        ",
      "  .####=====######=====##>.      ",
      "  .##@#::---########---##=>>.    ",
      "  .####=====######=====##>.      ",
      "   .....######...........        ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######..........>.       ",
      "  .####=====######=====##=>>.    ",
      "  .##@#::---########---###>.     ",
      "  .####=====######=====##.       ",
      "   .....######...........        ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######.........=>>.      ",
      "  .####=====######=====#>#>.     ",
      "  .##@#::---########---##.       ",
      "  .####=====######=====##.       ",
      "   .....######...........        ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######..........>.       ",
      "  .####=====######=====##=>>.    ",
      "  .##@#::---########---###>.     ",
      "  .####=====######=====##.       ",
      "   .....######...........        ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######...........        ",
      "  .####=====######=====##>.      ",
      "  .##@#::---########---##=>>.    ",
      "  .####=====######=====##>.      ",
      "   .....######...........        ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######...........        ",
      "  .####=====######=====##.       ",
      "  .##@#::---########---###>.     ",
      "  .####=====######=====##=>>.    ",
      "   .....######..........>.       ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######...........        ",
      "  .####=====######=====##.       ",
      "  .##@#::---########---##.       ",
      "  .####=====######=====#>#>.     ",
      "   .....######.........=>>.      ",
      "        .####.                   ",
      "         .^^.                    "],
    &["         .^^.                    ",
      "        .####.                   ",
      "   .....######...........        ",
      "  .####=====######=====##.       ",
      "  .##@#::---########---###>.     ",
      "  .####=====######=====##=>>.    ",
      "   .....######..........>.       ",
      "        .####.                   ",
      "         .^^.                    "],
];

const W: usize = 34;

fn base_color(ch: char) -> (f64, f64, f64) {
    match ch {
        '#' => (75.0, 95.0, 145.0),
        '=' => (95.0, 115.0, 160.0),
        '-' => (170.0, 180.0, 200.0),
        '@' => (240.0, 245.0, 255.0),
        ':' => (50.0, 65.0, 105.0),
        '^' => (60.0, 80.0, 130.0),
        '>' => (70.0, 88.0, 138.0),
        '.' | '`' => (40.0, 55.0, 95.0),
        _ => (75.0, 95.0, 145.0),
    }
}

// ─── 10 animation techniques ────────────────────────────────────────────────

struct Technique {
    name: &'static str,
    desc: &'static str,
}

const TECHNIQUES: &[Technique] = &[
    Technique { name: "1:Frames Only",     desc: "sprite swap only, no bend" },
    Technique { name: "2:Color Breath",    desc: "frames + color pulse" },
    Technique { name: "3:Color Wave",      desc: "frames + traveling color wave" },
    Technique { name: "4:Gentle Bend",     desc: "frames + tiny per-col bend (0.2)" },
    Technique { name: "5:Segment Bend",    desc: "frames + 4-segment block bend" },
    Technique { name: "6:Shadow Trail",    desc: "frames + fading echo behind" },
    Technique { name: "7:Brightness Depth",desc: "frames + depth-based brightness per col" },
    Technique { name: "8:Edge Glow",       desc: "frames + pulsing outline glow" },
    Technique { name: "9:Combo Smooth",    desc: "frames + color wave + segment bend + trail" },
    Technique { name: "10:Full",           desc: "everything combined, tuned" },
];

fn draw_shark(
    buf: &mut ratatui::buffer::Buffer,
    area: ratatui::layout::Rect,
    t: f64,
    speed: f64,
    technique: usize,
) {
    let sprite_h = FRAMES[0].len();
    let sx = (area.width as i32 - W as i32) / 2;
    let sy = (area.height as i32 - sprite_h as i32) / 2;
    let frame_idx = ((t * 8.0 * speed) as usize) % FRAMES.len();
    let sprite = FRAMES[frame_idx];

    // Technique 6, 9, 10: Shadow trail — draw faded previous frame first
    let has_trail = matches!(technique, 5 | 8 | 9);
    if has_trail {
        let prev_idx = ((t * 8.0 * speed - 2.0).max(0.0) as usize) % FRAMES.len();
        let prev = FRAMES[prev_idx];
        for (row, line) in prev.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                if ch == ' ' { continue; }
                let px = sx + col as i32;
                let py = sy + row as i32;
                if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 { continue; }
                let (br, bg, bb) = base_color(ch);
                let cell = &mut buf[(px as u16, py as u16)];
                cell.set_char(ch);
                cell.set_fg(Color::Rgb((br * 0.25) as u8, (bg * 0.25) as u8, (bb * 0.25) as u8));
                cell.set_style(Style::default());
            }
        }
    }

    // Main sprite
    for (row, line) in sprite.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == ' ' { continue; }

            let r = col as f64 / W as f64;

            // ── Position bend ──
            let bend: f64 = match technique {
                3 => {
                    // Technique 4: Gentle per-column bend
                    let omega = 2.0 * PI * 2.0 * speed;
                    (r * 3.0 - omega * t).sin() * 0.2 * r * r
                }
                4 => {
                    // Technique 5: Segment-based bend (groups of ~8 cols move together)
                    let segment = (col / 8) as f64;
                    let seg_r = segment / 4.0;
                    let omega = 2.0 * PI * 2.0 * speed;
                    (seg_r * 3.0 - omega * t).sin() * 0.4 * seg_r * seg_r
                }
                8 | 9 => {
                    // Technique 9, 10: Segment bend (smooth)
                    let segment = (col / 6) as f64;
                    let seg_r = segment / 5.0;
                    let omega = 2.0 * PI * 1.8 * speed;
                    (seg_r * 2.5 - omega * t).sin() * 0.35 * seg_r * seg_r
                }
                _ => 0.0,
            };

            let px = sx + col as i32;
            let py = sy + row as i32 + bend.round() as i32;
            if px < 0 || py < 0 || px >= area.width as i32 || py >= area.height as i32 { continue; }

            // ── Color computation ──
            let (br, bg, bb) = base_color(ch);
            let intensity = match technique {
                1 => {
                    // Technique 2: Color breathing
                    0.85 + 0.15 * (t * 2.0 * speed).sin()
                }
                2 => {
                    // Technique 3: Traveling color wave (head→tail)
                    let phase = r * 4.0 - t * 3.0 * speed;
                    0.75 + 0.25 * phase.sin()
                }
                6 => {
                    // Technique 7: Depth-based — center cols brighter
                    let center_dist = (r - 0.4).abs(); // 0.4 = body center
                    0.65 + 0.35 * (1.0 - center_dist * 2.0).max(0.0)
                }
                7 => {
                    // Technique 8: Edge glow — outline chars pulse
                    if ch == '.' || ch == '`' {
                        0.5 + 0.5 * (t * 5.0 * speed + col as f64 * 0.5).sin()
                    } else {
                        0.9 + 0.1 * (t * 1.5 * speed).sin()
                    }
                }
                8 => {
                    // Technique 9: Combo — wave + breath
                    let wave = 0.8 + 0.2 * (r * 4.0 - t * 3.0 * speed).sin();
                    let breath = 0.92 + 0.08 * (t * 1.5 * speed).sin();
                    wave * breath
                }
                9 => {
                    // Technique 10: Full — wave + glow + depth
                    let wave = 0.82 + 0.18 * (r * 4.0 - t * 2.5 * speed).sin();
                    let depth_bright = if ch == '.' || ch == '`' {
                        0.6 + 0.4 * (t * 4.0 * speed + col as f64 * 0.4).sin()
                    } else {
                        1.0
                    };
                    let breath = 0.94 + 0.06 * (t * 1.2 * speed).sin();
                    wave * depth_bright * breath
                }
                _ => 1.0, // Technique 1: no color change
            };

            let cell = &mut buf[(px as u16, py as u16)];
            cell.set_char(ch);
            cell.set_fg(Color::Rgb(
                (br * intensity).clamp(0.0, 255.0) as u8,
                (bg * intensity).clamp(0.0, 255.0) as u8,
                (bb * intensity).clamp(0.0, 255.0) as u8,
            ));
            cell.set_style(Style::default());
        }
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();

    let mut current = 0usize;
    let mut elapsed = 0.0f64;
    let mut speed = 1.0f64;
    let mut last = Instant::now();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let buf = f.buffer_mut();

            for y in 0..area.height {
                for x in 0..area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(8, 10, 18));
                    cell.set_fg(Color::Rgb(8, 10, 18));
                }
            }

            draw_shark(buf, area, elapsed, speed, current);

            if area.height > 2 && area.width > 20 {
                let t = &TECHNIQUES[current];
                let hdr = format!(
                    "  {} - {}  speed:{:.1}x  \u{2190}\u{2192}:switch  \u{2191}\u{2193}:speed  q:quit",
                    t.name, t.desc, speed
                );
                for (i, ch) in hdr.chars().enumerate() {
                    if i >= area.width as usize { break; }
                    let cell = &mut buf[(i as u16, 0)];
                    cell.set_char(ch);
                    cell.set_fg(Color::Rgb(50, 45, 75));
                }
            }
        })?;

        let timeout = TICK.saturating_sub(last.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press {
                    match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Right | KeyCode::Char('n') => {
                            current = (current + 1) % TECHNIQUES.len();
                        }
                        KeyCode::Left | KeyCode::Char('p') => {
                            current = if current == 0 { TECHNIQUES.len() - 1 } else { current - 1 };
                        }
                        KeyCode::Up => speed = (speed + 0.2).min(5.0),
                        KeyCode::Down => speed = (speed - 0.2).max(0.2),
                        _ => {}
                    }
                }
            }
        }
        elapsed += last.elapsed().as_secs_f64();
        last = Instant::now();
    }

    ratatui::restore();
    Ok(())
}
