//! Pond: manages koi fish and food pellets as a simulation.

use crate::food::{Food, EAT_RANGE_SQ};
use crate::koi::Koi;

pub struct Pond {
    pub fish: Vec<Koi>,
    pub foods: Vec<Food>,
}

impl Pond {
    pub fn new(w: f64, h: f64) -> Self {
        Pond {
            fish: vec![
                Koi::new(w * 0.3, h * 0.35, 0.3, 7.5, 1.0),
                Koi::new(w * 0.7, h * 0.6, 3.5, 7.0, 4.3),
                Koi::new(w * 0.5, h * 0.25, 1.8, 6.5, 7.1),
                Koi::new(w * 0.4, h * 0.7, 5.2, 7.2, 11.5),
            ],
            foods: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f64, t: f64, w: f64, h: f64) {
        let Pond { fish, foods } = self;

        for food in foods.iter_mut() {
            let being_eaten = fish.iter().any(|k| {
                let (hx, hy) = k.head();
                let dx = hx - food.x;
                let dy = hy - food.y;
                dx * dx + dy * dy < EAT_RANGE_SQ
            });
            food.tick(dt, being_eaten);
        }
        foods.retain(|f| f.is_alive());

        for k in fish.iter_mut() {
            k.update(dt, t, w, h, foods);
        }
    }

    pub fn drop_food(&mut self, x: f64, y: f64) {
        self.foods.push(Food::new(x, y));
    }
}

/// Visible world height accounting for header row and braille aspect ratio.
pub fn world_height(th: u16) -> f64 {
    (th.saturating_sub(1) as f64) * 2.0
}

/// Uniform scale factor from world coordinates to canvas sub-pixels.
pub fn compute_scale(tw: u16, th: u16) -> f64 {
    let ch = th.saturating_sub(1) as f64;
    (ch * 4.0 / th as f64).min(tw as f64 * 2.0 / tw as f64)
}

/// Convert screen (terminal cell) coordinates to world coordinates.
pub fn screen_to_world(col: u16, row: u16, scale: f64) -> (f64, f64) {
    (
        col as f64 * 2.0 / scale,
        row.saturating_sub(1) as f64 * 4.0 / scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_height_accounts_for_header() {
        assert_eq!(world_height(24), 46.0);
        assert_eq!(world_height(1), 0.0);
    }

    #[test]
    fn compute_scale_typical_terminal() {
        let scale = compute_scale(80, 24);
        assert!((scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn screen_to_world_header_offset() {
        let scale = 2.0;
        // row 0 is the header — saturating_sub clamps to 0
        let (_, wy0) = screen_to_world(0, 0, scale);
        assert_eq!(wy0, 0.0, "row 0 (header) maps to world y=0 via saturating_sub");

        let (_, wy1) = screen_to_world(0, 1, scale);
        assert_eq!(wy1, 0.0, "row 1 maps to world y=0");

        let (_, wy2) = screen_to_world(0, 2, scale);
        assert!(wy2 > 0.0, "row 2 maps to positive world y");
    }

    #[test]
    fn screen_to_world_roundtrip() {
        let scale = 2.0;
        let (wx, wy) = screen_to_world(40, 12, scale);
        // world → screen: scr_x = wx*scale/2, scr_y = wy*scale/4 + 1
        let scr_x = (wx * scale / 2.0).round() as u16;
        let scr_y = (wy * scale / 4.0 + 1.0).round() as u16;
        assert_eq!(scr_x, 40);
        assert_eq!(scr_y, 12);
    }

    #[test]
    fn pond_new_creates_four_fish() {
        let pond = Pond::new(80.0, 46.0);
        assert_eq!(pond.fish.len(), 4);
        assert!(pond.foods.is_empty());
    }

    #[test]
    fn drop_food_adds_pellet() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(10.0, 20.0);
        pond.drop_food(30.0, 40.0);
        assert_eq!(pond.foods.len(), 2);
    }

    #[test]
    fn update_removes_dead_food() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(10.0, 20.0);
        pond.foods[0].remaining = 0.001;
        pond.update(0.1, 0.0, 80.0, 46.0);
        assert!(pond.foods.is_empty());
    }
}
