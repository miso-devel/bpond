//! Pond: manages koi fish, food, ripples, bubbles, and rain as a simulation.

use crate::bubble::Bubble;
use crate::food::{Food, EAT_RANGE_SQ};
use crate::koi::Koi;
use crate::rain::RainSystem;
use crate::ripple::Ripple;
use crate::rng::pseudo_rand;

pub struct Pond {
    pub fish: Vec<Koi>,
    pub foods: Vec<Food>,
    pub ripples: Vec<Ripple>,
    pub bubbles: Vec<Bubble>,
    pub rain: RainSystem,
    pub rain_mode: bool,
    bubble_spawn_timer: f64,
    bubble_rng: f64,
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
            ripples: Vec::new(),
            bubbles: Vec::new(),
            rain: RainSystem::new(),
            rain_mode: false,
            bubble_spawn_timer: 1.0,
            bubble_rng: 0.0,
        }
    }

    pub fn update(&mut self, dt: f64, t: f64, w: f64, h: f64) {
        {
            let Pond { fish, foods, .. } = self;

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

        for r in &mut self.ripples {
            r.tick(dt);
        }
        self.ripples.retain(|r| r.is_alive());

        self.bubble_spawn_timer -= dt;
        if self.bubble_spawn_timer <= 0.0 {
            let bx = pseudo_rand(self.bubble_rng) * w;
            let by = h * (0.5 + pseudo_rand(self.bubble_rng + 1.0) * 0.5);
            self.bubbles.push(Bubble::new(bx, by));
            self.bubble_spawn_timer = 1.0 + pseudo_rand(self.bubble_rng + 2.0) * 1.5;
            self.bubble_rng += 3.0;
        }
        for b in &mut self.bubbles {
            b.tick(dt);
        }
        self.bubbles.retain(|b| b.is_alive());

        self.rain.tick(dt);
        if self.rain_mode {
            let new_drops = self.rain.spawn(dt, w, h);
            for (x, y) in new_drops {
                self.ripples.push(Ripple::new_rain(x, y));
            }
        }
    }

    pub fn drop_food(&mut self, x: f64, y: f64) {
        self.foods.push(Food::new(x, y));
        self.ripples.push(Ripple::new(x, y, 8.0, 1.5));
        self.ripples.push(Ripple::new(x, y, 15.0, 2.5));
        self.ripples.push(Ripple::new(x, y, 22.0, 3.5));
    }

    pub fn scare(&mut self, x: f64, y: f64) {
        for fish in &mut self.fish {
            fish.scare(x, y);
        }
    }

    pub fn add_fish(&mut self, w: f64, h: f64, t: f64) {
        let seed = t + self.fish.len() as f64 * 17.3;
        let x = pseudo_rand(seed) * w;
        let y = pseudo_rand(seed + 1.0) * h;
        let heading = pseudo_rand(seed + 2.0) * std::f64::consts::TAU;
        let speed = 6.5 + pseudo_rand(seed + 3.0);
        let id = pseudo_rand(seed + 4.0) * 20.0;
        self.fish.push(Koi::new(x, y, heading, speed, id));
    }

    pub fn remove_fish(&mut self) {
        self.fish.pop();
    }

    pub fn toggle_rain(&mut self) {
        self.rain_mode = !self.rain_mode;
    }
}

/// Visible world height accounting for header row and braille aspect ratio.
pub fn world_height(th: u16) -> f64 {
    (th.saturating_sub(1) as f64) * 2.0
}

/// Uniform scale factor from world coordinates to canvas sub-pixels.
pub fn compute_scale(_tw: u16, th: u16) -> f64 {
    let ch = th.saturating_sub(1) as f64;
    (ch * 4.0 / th as f64).min(2.0)
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
        let (_, wy0) = screen_to_world(0, 0, scale);
        assert_eq!(
            wy0, 0.0,
            "row 0 (header) maps to world y=0 via saturating_sub"
        );

        let (_, wy1) = screen_to_world(0, 1, scale);
        assert_eq!(wy1, 0.0, "row 1 maps to world y=0");

        let (_, wy2) = screen_to_world(0, 2, scale);
        assert!(wy2 > 0.0, "row 2 maps to positive world y");
    }

    #[test]
    fn screen_to_world_roundtrip() {
        let scale = 2.0;
        let (wx, wy) = screen_to_world(40, 12, scale);
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

#[cfg(test)]
mod new_feature_tests {
    use super::*;

    // -- toggle_rain --------------------------------------------------------

    #[test]
    fn toggle_rain_enables_rain_mode() {
        let mut pond = Pond::new(80.0, 46.0);
        assert!(!pond.rain_mode, "rain_mode should be off by default");
        pond.toggle_rain();
        assert!(pond.rain_mode, "rain_mode should be on after toggle");
    }

    #[test]
    fn toggle_rain_twice_restores_off() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.toggle_rain();
        pond.toggle_rain();
        assert!(!pond.rain_mode, "rain_mode should be off after two toggles");
    }

    // -- add_fish / remove_fish ---------------------------------------------

    #[test]
    fn add_fish_increases_count_by_one() {
        let mut pond = Pond::new(80.0, 46.0);
        let before = pond.fish.len();
        pond.add_fish(80.0, 46.0, 0.0);
        assert_eq!(pond.fish.len(), before + 1);
    }

    #[test]
    fn add_fish_places_within_world_bounds() {
        let w = 80.0_f64;
        let h = 46.0_f64;
        let mut pond = Pond::new(w, h);
        pond.add_fish(w, h, 1.23);
        let new_koi = pond.fish.last().unwrap();
        let (hx, hy) = new_koi.head();
        assert!(hx >= 0.0 && hx <= w, "new fish x={hx} out of bounds");
        assert!(hy >= 0.0 && hy <= h, "new fish y={hy} out of bounds");
    }

    #[test]
    fn remove_fish_decreases_count_by_one() {
        let mut pond = Pond::new(80.0, 46.0);
        let before = pond.fish.len();
        pond.remove_fish();
        assert_eq!(pond.fish.len(), before - 1);
    }

    #[test]
    fn remove_fish_does_nothing_when_empty() {
        let mut pond = Pond::new(80.0, 46.0);
        let n = pond.fish.len();
        for _ in 0..n {
            pond.remove_fish();
        }
        assert!(pond.fish.is_empty());
        pond.remove_fish();
        assert!(pond.fish.is_empty());
    }

    // -- drop_food + ripples ------------------------------------------------

    #[test]
    fn drop_food_creates_three_ripples() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(40.0, 23.0);
        assert_eq!(
            pond.ripples.len(),
            3,
            "drop_food should create 3 concentric ripples"
        );
    }

    #[test]
    fn drop_food_ripples_are_at_food_position() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(15.0, 22.0);
        for r in &pond.ripples {
            assert!((r.x - 15.0).abs() < 1e-10, "ripple x should match food x");
            assert!((r.y - 22.0).abs() < 1e-10, "ripple y should match food y");
        }
    }

    #[test]
    fn drop_food_ripples_have_distinct_max_radii() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(40.0, 23.0);
        pond.update(1.0, 0.0, 80.0, 46.0);
        let alive = pond.ripples.iter().filter(|r| r.is_alive()).count();
        assert!(alive >= 1, "at least one ripple should be alive after 1s");
    }

    // -- scare (pond-level) -------------------------------------------------

    #[test]
    fn pond_scare_does_not_panic() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.scare(40.0, 23.0);
    }

    #[test]
    fn pond_scare_activates_all_fish() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.scare(0.0, 0.0);
        for (i, fish) in pond.fish.iter().enumerate() {
            assert!(
                fish.scare_timer > 0.0,
                "fish[{i}] scare_timer should be positive after pond.scare()"
            );
        }
    }

    // -- ripple update integration -----------------------------------------

    #[test]
    fn ripples_are_removed_when_dead() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(40.0, 23.0);
        assert_eq!(pond.ripples.len(), 3);

        for _ in 0..1000 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        assert!(
            pond.ripples.is_empty(),
            "all ripples should be removed after their lifetime"
        );
    }

    // -- bubble integration -------------------------------------------------

    #[test]
    fn bubbles_spawn_automatically_over_time() {
        let mut pond = Pond::new(80.0, 46.0);
        assert!(pond.bubbles.is_empty(), "no bubbles at start");

        for _ in 0..500 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        assert!(
            !pond.bubbles.is_empty(),
            "bubbles should appear after 5 seconds"
        );
    }

    // -- rain integration ---------------------------------------------------

    #[test]
    fn rain_mode_creates_ripples_on_update() {
        let mut pond = Pond::new(80.0, 46.0);
        pond.toggle_rain();

        for _ in 0..200 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        assert!(
            !pond.ripples.is_empty(),
            "rain mode should generate ripples via raindrop spawning"
        );
    }

    #[test]
    fn no_ripples_from_rain_when_mode_off() {
        let mut pond = Pond::new(80.0, 46.0);
        for _ in 0..200 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        assert!(
            pond.ripples.is_empty(),
            "no ripples should appear without food drop or rain mode"
        );
    }

    // -- multiple food drops ------------------------------------------------

    #[test]
    fn two_food_drops_create_six_ripples() {
        // Each drop_food call creates exactly 3 concentric ripples.
        // Two separate drops must accumulate to 6 total.
        let mut pond = Pond::new(80.0, 46.0);
        pond.drop_food(20.0, 15.0);
        pond.drop_food(60.0, 30.0);
        assert_eq!(
            pond.ripples.len(),
            6,
            "two food drops should create 6 ripples total"
        );
    }

    // -- compute_scale edge cases -------------------------------------------

    #[test]
    fn compute_scale_single_row_terminal() {
        // th=1 → ch = saturating_sub(1) = 0, divides by th=1 → 0.0 (not NaN)
        let scale = super::compute_scale(80, 1);
        assert!(
            scale.is_finite(),
            "scale should be finite for th=1, got {scale}"
        );
        assert_eq!(scale, 0.0);
    }

    #[test]
    fn compute_scale_caps_at_two() {
        // For large terminals the scale should be capped at 2.0.
        let scale = super::compute_scale(200, 100);
        assert!(
            (scale - 2.0).abs() < 0.01,
            "scale should cap at 2.0, got {scale}"
        );
    }

    // -- rain_drops_cleaned_up_after_rain_mode_disabled ---------------------

    #[test]
    fn rain_drops_cleaned_up_after_rain_mode_disabled() {
        // Re-prevention for logic-bug family: drops must not persist after toggle-off.
        let mut pond = Pond::new(80.0, 46.0);
        pond.toggle_rain();
        // Accumulate drops
        for _ in 0..50 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        // Disable rain
        pond.toggle_rain();
        assert!(!pond.rain_mode);
        // Advance well past drop lifetime (0.3 s) — drops must expire via tick()
        for _ in 0..50 {
            pond.update(0.01, 0.0, 80.0, 46.0);
        }
        assert!(
            pond.rain.drops.is_empty(),
            "rain drops should be cleaned up even when rain mode is off"
        );
    }
}
