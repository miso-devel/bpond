//! Braille sub-pixel canvas for high-resolution terminal rendering.
//!
//! Each terminal cell maps to a 2×4 grid of sub-pixels using Unicode
//! braille characters (U+2800–U+28FF). This gives 8× the resolution
//! of normal character rendering.

use ratatui::style::{Color, Style};

const BRAILLE_BASE: u32 = 0x2800;

/// Bit positions for each sub-pixel within a braille character.
/// `BRAILLE_DOT[col][row]` gives the bit to set for that position.
const BRAILLE_DOT: [[u32; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // left column (x=0)
    [0x08, 0x10, 0x20, 0x80], // right column (x=1)
];

/// A pixel buffer that renders to braille characters.
pub struct Canvas {
    /// Sub-pixel dimensions (2× char width, 4× char height).
    pub w: usize,
    pub h: usize,
    cw: usize,
    px: Vec<(bool, u8, u8, u8)>,
}

impl Canvas {
    /// Create a canvas for the given terminal cell dimensions.
    pub fn new(cw: usize, ch: usize) -> Self {
        Canvas {
            w: cw * 2,
            h: ch * 4,
            cw,
            px: vec![(false, 0, 0, 0); cw * 2 * ch * 4],
        }
    }

    /// Set a single sub-pixel.
    pub fn dot(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = (true, r, g, b);
        }
    }

    /// 2×2 sub-pixel block.
    pub fn fat(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in 0..2 {
            for dx in 0..2 {
                self.dot(x + dx, y + dy, r, g, b);
            }
        }
    }

    /// 3×3 sub-pixel block.
    pub fn thick(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                self.dot(x + dx, y + dy, r, g, b);
            }
        }
    }

    /// Render the canvas into a ratatui buffer using braille characters.
    pub fn render(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        ox: u16,
        oy: u16,
        area: ratatui::layout::Rect,
    ) {
        let ch = self.h / 4;
        for cy in 0..ch {
            for cx in 0..self.cw {
                let mut bits = 0u32;
                let (mut tr, mut tg, mut tb, mut n) = (0u32, 0u32, 0u32, 0u32);
                #[allow(clippy::needless_range_loop)]
                for dy in 0..4usize {
                    for dx in 0..2usize {
                        let (on, r, g, b) = self.px[(cy * 4 + dy) * self.w + cx * 2 + dx];
                        if on {
                            bits |= BRAILLE_DOT[dx][dy];
                            tr += r as u32;
                            tg += g as u32;
                            tb += b as u32;
                            n += 1;
                        }
                    }
                }
                if bits == 0 {
                    continue;
                }
                let bx = ox as i32 + cx as i32;
                let by = oy as i32 + cy as i32;
                if bx < 0 || by < 0 || bx >= area.width as i32 || by >= area.height as i32 {
                    continue;
                }
                let cell = &mut buf[(bx as u16, by as u16)];
                cell.set_char(char::from_u32(BRAILLE_BASE + bits).unwrap_or(' '));
                cell.set_fg(Color::Rgb((tr / n) as u8, (tg / n) as u8, (tb / n) as u8));
                cell.set_style(Style::default());
            }
        }
    }
}
