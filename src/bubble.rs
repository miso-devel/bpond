//! Rising bubble particles that spawn automatically throughout the pond.

use crate::canvas::Canvas;

pub struct Bubble {
    pub x: f64,
    pub y: f64,
    rise_speed: f64,
    age: f64,
    lifetime: f64,
}

impl Bubble {
    pub fn new(x: f64, y: f64) -> Self {
        Bubble {
            x,
            y,
            rise_speed: 2.0,
            age: 0.0,
            lifetime: 5.0,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        self.y -= self.rise_speed * dt;
        self.age += dt;
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime && self.y > 0.0
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64) {
        let px = (self.x * scale) as i32;
        let py = (self.y * scale) as i32;
        let fade = (1.0 - self.age / self.lifetime).max(0.0);
        canvas.dot(
            px,
            py,
            (180.0 * fade) as u8,
            (210.0 * fade) as u8,
            (240.0 * fade) as u8,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    // -- new ----------------------------------------------------------------

    #[test]
    fn new_bubble_starts_at_position() {
        let b = Bubble::new(7.0, 25.0);
        assert!((b.x - 7.0).abs() < 1e-10);
        assert!((b.y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn new_bubble_is_alive_initially() {
        let b = Bubble::new(0.0, 10.0);
        assert!(b.is_alive());
    }

    // -- tick: rising -------------------------------------------------------

    #[test]
    fn tick_makes_bubble_rise() {
        let mut b = Bubble::new(10.0, 20.0);
        let y_before = b.y;
        b.tick(0.5);
        assert!(b.y < y_before, "bubble should rise (y decreases)");
    }

    #[test]
    fn tick_advances_age() {
        let mut b = Bubble::new(10.0, 20.0);
        for _ in 0..1000 {
            b.tick(0.1);
            if !b.is_alive() {
                return;
            }
        }
        panic!("bubble should die eventually (lifetime exhausted)");
    }

    // -- is_alive -----------------------------------------------------------

    #[test]
    fn bubble_dies_when_age_exceeds_lifetime() {
        let mut b = Bubble::new(10.0, 20.0);
        b.tick(10.0);
        assert!(!b.is_alive());
    }

    #[test]
    fn bubble_dies_when_it_reaches_surface() {
        let mut b = Bubble::new(10.0, 0.1);
        b.tick(0.1);
        assert!(!b.is_alive(), "bubble above surface should not be alive");
    }

    #[test]
    fn bubble_alive_in_mid_lifetime() {
        let mut b = Bubble::new(10.0, 50.0);
        b.tick(1.0);
        assert!(b.is_alive());
    }

    // -- draw ---------------------------------------------------------------

    #[test]
    fn draw_produces_at_least_one_pixel() {
        let b = Bubble::new(20.0, 15.0);
        let mut canvas = Canvas::new(80, 60);
        b.draw(&mut canvas, 2.0);

        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(
            lit > 0,
            "bubble draw should light up at least one pixel, got {lit}"
        );
    }
}
