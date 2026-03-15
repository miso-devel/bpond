use ratatui::{
    style::{Color, Style},
    Frame,
};

const FRAME_DELAY_MS: u64 = 35; // ~28.6fps, same as gostty
const IMAGE_WIDTH: usize = 77;
const IMAGE_HEIGHT: usize = 41;

/// A single cell: the character and whether it's part of the colored aura.
#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    colored: bool,
}

/// Pre-parsed ghost animation frames from gostty's animation-data.json.
pub struct GhostAnimation {
    frames: Vec<Vec<Vec<Cell>>>,
}

impl GhostAnimation {
    pub fn load() -> Self {
        let json_str = include_str!("../animation-data.json");
        let raw_frames: Vec<Vec<String>> =
            serde_json::from_str(json_str).expect("failed to parse animation-data.json");

        let frames = raw_frames
            .iter()
            .map(|frame| frame.iter().map(|line| parse_line(line)).collect())
            .collect();

        GhostAnimation { frames }
    }

    fn frame_index(&self, elapsed: f64) -> usize {
        let ms = (elapsed * 1000.0) as u64;
        (ms / FRAME_DELAY_MS) as usize % self.frames.len()
    }

    pub fn draw(&self, frame: &mut Frame, elapsed: f64) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Dark background
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_bg(Color::Rgb(10, 8, 16));
                cell.set_fg(Color::Rgb(10, 8, 16));
            }
        }

        let idx = self.frame_index(elapsed);
        let ghost_frame = &self.frames[idx];

        // Center on screen
        let start_x = (area.width as i32 - IMAGE_WIDTH as i32) / 2;
        let start_y = (area.height as i32 - IMAGE_HEIGHT as i32) / 2;

        for (row, line) in ghost_frame.iter().enumerate() {
            for (col, cell) in line.iter().enumerate() {
                if cell.ch == ' ' {
                    continue;
                }
                let px = start_x + col as i32;
                let py = start_y + row as i32;
                if px < 0
                    || py < 0
                    || px >= area.width as i32
                    || py >= area.height as i32
                {
                    continue;
                }

                let color = if cell.colored {
                    // Aura: blue (same as gostty default)
                    Color::Rgb(70, 120, 255)
                } else {
                    // Ghost body: light warm white
                    Color::Rgb(210, 210, 230)
                };

                let buf_cell = &mut buf[(px as u16, py as u16)];
                buf_cell.set_char(cell.ch);
                buf_cell.set_fg(color);
                buf_cell.set_style(Style::default());
            }
        }

        // Header
        let header_text = "  terminal-zoo  Ghost    \u{2190} \u{2192} switch  q quit";
        if area.width > 10 && area.height > 1 {
            for (i, ch) in header_text.chars().enumerate() {
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

/// Parse a line with `<c>...</c>` color tags into cells.
fn parse_line(line: &str) -> Vec<Cell> {
    let mut result = Vec::with_capacity(IMAGE_WIDTH);
    let mut colored = false;
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"<c>" {
            colored = true;
            i += 3;
        } else if i + 4 <= bytes.len() && &bytes[i..i + 4] == b"</c>" {
            colored = false;
            i += 4;
        } else {
            let ch = line[i..].chars().next().unwrap();
            result.push(Cell { ch, colored });
            i += ch.len_utf8();
        }
    }

    result
}
