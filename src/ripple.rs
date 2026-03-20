//! Expanding ring ripple effect for food drops and raindrops.

use crate::canvas::Canvas;
use std::f64::consts::PI;

pub struct Ripple {
    pub x: f64,
    pub y: f64,
    radius: f64,
    max_radius: f64,
    age: f64,
    lifetime: f64,
}

impl Ripple {
    pub fn new(x: f64, y: f64, max_radius: f64, lifetime: f64) -> Self {
        Ripple {
            x,
            y,
            radius: 0.0,
            max_radius,
            age: 0.0,
            lifetime,
        }
    }

    #[cfg(test)]
    pub fn new_food(x: f64, y: f64) -> Self {
        Self::new(x, y, 22.0, 3.5)
    }

    pub fn new_rain(x: f64, y: f64) -> Self {
        Self::new(x, y, 5.0, 1.2)
    }

    pub fn tick(&mut self, dt: f64) {
        self.age += dt;
        self.radius = (self.max_radius * self.age / self.lifetime).min(self.max_radius);
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }

    #[cfg(test)]
    pub fn current_radius(&self) -> f64 {
        self.radius
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64) {
        if !self.is_alive() {
            return;
        }
        let r_px = (self.radius * scale) as i32;
        if r_px == 0 {
            return;
        }
        let alpha = (1.0 - self.age / self.lifetime).max(0.0);
        let cx = (self.x * scale) as i32;
        let cy = (self.y * scale) as i32;
        let steps = (r_px * 6).max(16);
        for i in 0..steps {
            let angle = 2.0 * PI * i as f64 / steps as f64;
            let px = cx + (r_px as f64 * angle.cos()) as i32;
            let py = cy + (r_px as f64 * angle.sin()) as i32;
            canvas.dot(
                px,
                py,
                (60.0 * alpha) as u8,
                (120.0 * alpha) as u8,
                (200.0 * alpha) as u8,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    // -- new_food -----------------------------------------------------------

    #[test]
    fn new_food_ripple_starts_at_position() {
        let r = Ripple::new_food(12.0, 34.0);
        assert!((r.x - 12.0).abs() < 1e-10);
        assert!((r.y - 34.0).abs() < 1e-10);
    }

    #[test]
    fn new_food_ripple_is_alive_initially() {
        let r = Ripple::new_food(0.0, 0.0);
        assert!(r.is_alive());
    }

    #[test]
    fn new_food_ripple_radius_is_zero_initially() {
        let r = Ripple::new_food(0.0, 0.0);
        assert!(r.current_radius() < 1e-10);
    }

    // -- new_rain -----------------------------------------------------------

    #[test]
    fn new_rain_ripple_starts_at_position() {
        let r = Ripple::new_rain(5.5, 8.8);
        assert!((r.x - 5.5).abs() < 1e-10);
        assert!((r.y - 8.8).abs() < 1e-10);
    }

    #[test]
    fn new_rain_ripple_is_alive_initially() {
        let r = Ripple::new_rain(0.0, 0.0);
        assert!(r.is_alive());
    }

    #[test]
    fn rain_ripple_has_smaller_max_radius_than_food_ripple() {
        let mut rain = Ripple::new_rain(0.0, 0.0);
        let mut food = Ripple::new_food(0.0, 0.0);

        let large_dt = 10.0;
        rain.tick(large_dt * 0.99_f64.min(1.0));
        food.tick(large_dt * 0.99_f64.min(1.0));

        assert!(rain.current_radius() <= food.current_radius() + 1e-10);
    }

    // -- tick ---------------------------------------------------------------

    #[test]
    fn tick_grows_radius_over_time() {
        let mut r = Ripple::new_food(0.0, 0.0);
        let r0 = r.current_radius();
        r.tick(0.1);
        let r1 = r.current_radius();
        assert!(r1 > r0, "radius should grow after tick");
    }

    #[test]
    fn tick_is_monotonic() {
        let mut r = Ripple::new_food(0.0, 0.0);
        let mut prev = r.current_radius();
        for _ in 0..10 {
            r.tick(0.1);
            let cur = r.current_radius();
            assert!(cur >= prev, "radius must not decrease between ticks");
            prev = cur;
        }
    }

    // -- is_alive -----------------------------------------------------------

    #[test]
    fn ripple_dies_after_lifetime() {
        let mut r = Ripple::new_rain(0.0, 0.0);
        r.tick(2.0);
        assert!(!r.is_alive());
    }

    #[test]
    fn ripple_alive_before_lifetime_expires() {
        let mut r = Ripple::new_rain(0.0, 0.0);
        r.tick(0.5);
        assert!(r.is_alive());
    }

    // -- draw ---------------------------------------------------------------

    #[test]
    fn draw_produces_visible_pixels() {
        let mut r = Ripple::new_food(20.0, 15.0);
        r.tick(0.5);
        let mut canvas = Canvas::new(80, 60);
        r.draw(&mut canvas, 2.0);

        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(
            lit > 0,
            "draw should light up at least one pixel, got {lit}"
        );
    }

    #[test]
    fn draw_dead_ripple_produces_no_visible_pixels() {
        let mut r = Ripple::new_rain(20.0, 15.0);
        r.tick(5.0);
        let mut canvas = Canvas::new(80, 60);
        r.draw(&mut canvas, 2.0);

        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert_eq!(lit, 0, "dead ripple should not draw any visible pixels");
    }
}
