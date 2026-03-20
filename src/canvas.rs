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

    /// Check whether a sub-pixel is set (for testing).
    #[cfg(test)]
    pub fn get(&self, x: usize, y: usize) -> (bool, u8, u8, u8) {
        if x < self.w && y < self.h {
            self.px[y * self.w + x]
        } else {
            (false, 0, 0, 0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn new_canvas_dimensions() {
        let c = Canvas::new(10, 5);
        assert_eq!(c.w, 20);
        assert_eq!(c.h, 20);
    }

    #[test]
    fn dot_sets_pixel() {
        let mut c = Canvas::new(10, 5);
        c.dot(3, 7, 255, 128, 0);
        let (on, r, g, b) = c.get(3, 7);
        assert!(on);
        assert_eq!((r, g, b), (255, 128, 0));
    }

    #[test]
    fn dot_ignores_negative_coords() {
        let mut c = Canvas::new(10, 5);
        c.dot(-1, 0, 255, 0, 0);
        c.dot(0, -1, 255, 0, 0);
        // No panic, all pixels still off
        assert!(!c.get(0, 0).0);
    }

    #[test]
    fn dot_ignores_out_of_bounds() {
        let mut c = Canvas::new(10, 5);
        c.dot(20, 0, 255, 0, 0);
        c.dot(0, 20, 255, 0, 0);
        // No panic
    }

    #[test]
    fn fat_sets_2x2_block() {
        let mut c = Canvas::new(10, 5);
        c.fat(4, 4, 100, 100, 100);
        assert!(c.get(4, 4).0);
        assert!(c.get(5, 4).0);
        assert!(c.get(4, 5).0);
        assert!(c.get(5, 5).0);
        assert!(!c.get(3, 4).0);
        assert!(!c.get(6, 4).0);
    }

    #[test]
    fn thick_sets_3x3_block() {
        let mut c = Canvas::new(10, 5);
        c.thick(5, 5, 100, 100, 100);
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                assert!(
                    c.get((5 + dx) as usize, (5 + dy) as usize).0,
                    "pixel at ({}, {}) should be set",
                    5 + dx,
                    5 + dy
                );
            }
        }
        assert!(!c.get(3, 5).0);
        assert!(!c.get(7, 5).0);
    }

    // -- render: braille encoding -------------------------------------------

    fn make_buf(w: u16, h: u16) -> (Buffer, Rect) {
        let area = Rect::new(0, 0, w, h);
        (Buffer::empty(area), area)
    }

    #[test]
    fn render_single_dot_top_left() {
        // sub-pixel (0,0) in cell (0,0) → BRAILLE_DOT[0][0] = 0x01 → U+2801 '⠁'
        let mut c = Canvas::new(4, 2);
        c.dot(0, 0, 255, 0, 0);
        let (mut buf, area) = make_buf(4, 3);
        c.render(&mut buf, 0, 0, area);
        assert_eq!(buf[(0u16, 0u16)].symbol(), "⠁");
    }

    #[test]
    fn render_all_dots_in_cell() {
        // Set all 8 sub-pixels in cell (0,0) → bits=0xFF → U+28FF '⣿'
        let mut c = Canvas::new(4, 2);
        for dy in 0..4 {
            for dx in 0..2 {
                c.dot(dx, dy, 255, 255, 255);
            }
        }
        let (mut buf, area) = make_buf(4, 3);
        c.render(&mut buf, 0, 0, area);
        assert_eq!(buf[(0u16, 0u16)].symbol(), "⣿");
    }

    #[test]
    fn render_averages_foreground_color() {
        // Two dots: (255,0,0) and (0,255,0) → average (127,127,0)
        let mut c = Canvas::new(4, 2);
        c.dot(0, 0, 255, 0, 0);
        c.dot(1, 0, 0, 255, 0);
        let (mut buf, area) = make_buf(4, 3);
        c.render(&mut buf, 0, 0, area);
        let cell = &buf[(0u16, 0u16)];
        assert_eq!(cell.fg, Color::Rgb(127, 127, 0));
    }

    #[test]
    fn render_empty_cell_is_untouched() {
        let c = Canvas::new(4, 2);
        let (mut buf, area) = make_buf(4, 3);
        let before = buf[(0u16, 0u16)].symbol().to_string();
        c.render(&mut buf, 0, 0, area);
        assert_eq!(buf[(0u16, 0u16)].symbol(), before);
    }

    #[test]
    fn render_with_offset() {
        let mut c = Canvas::new(2, 1);
        c.dot(0, 0, 255, 0, 0);
        let (mut buf, area) = make_buf(10, 10);
        c.render(&mut buf, 3, 2, area);
        // Should be at cell (3, 2)
        assert_eq!(buf[(3u16, 2u16)].symbol(), "⠁");
        // Origin should be untouched
        assert_ne!(buf[(0u16, 0u16)].symbol(), "⠁");
    }

    #[test]
    fn render_respects_area_bounds() {
        let mut c = Canvas::new(4, 2);
        c.dot(0, 0, 255, 0, 0);
        // Render with area smaller than canvas — offset pushes cell out of bounds
        let (mut buf, area) = make_buf(2, 2);
        c.render(&mut buf, 5, 0, area);
        // No panic, and cell (0,0) is untouched
        assert_ne!(buf[(0u16, 0u16)].symbol(), "⠁");
    }
}
