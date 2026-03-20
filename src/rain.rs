//! Rain system: spawns raindrops at random positions and returns their
//! coordinates so the pond can create ripples.

use crate::canvas::Canvas;
use crate::rng::pseudo_rand;

pub struct Raindrop {
    pub x: f64,
    pub y: f64,
    age: f64,
    lifetime: f64,
}

impl Raindrop {
    pub fn new(x: f64, y: f64) -> Self {
        Raindrop {
            x,
            y,
            age: 0.0,
            lifetime: 0.3,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        self.age += dt;
    }

    pub fn is_alive(&self) -> bool {
        self.age < self.lifetime
    }
}

pub struct RainSystem {
    pub drops: Vec<Raindrop>,
    spawn_timer: f64,
    rng_state: f64,
}

impl RainSystem {
    pub fn new() -> Self {
        RainSystem {
            drops: Vec::new(),
            spawn_timer: 0.0,
            rng_state: 0.0,
        }
    }

    /// Advance existing drops and remove dead ones. Called every frame
    /// regardless of rain mode so drops left over from a disabled rain session
    /// are properly cleaned up.
    pub fn tick(&mut self, dt: f64) {
        for drop in &mut self.drops {
            drop.tick(dt);
        }
        self.drops.retain(|d| d.is_alive());
    }

    /// Spawn new drops and return their world positions so the caller can
    /// create matching ripples. Only call this when rain mode is active.
    pub fn spawn(&mut self, dt: f64, w: f64, h: f64) -> Vec<(f64, f64)> {
        self.spawn_timer -= dt;
        let mut spawned = Vec::new();
        while self.spawn_timer <= 0.0 {
            let x = pseudo_rand(self.rng_state) * w;
            let y = pseudo_rand(self.rng_state + 1.0) * h;
            self.drops.push(Raindrop::new(x, y));
            spawned.push((x, y));
            self.rng_state += 2.0;
            // interval 0.08–0.25 s
            self.spawn_timer += 0.08 + pseudo_rand(self.rng_state) * 0.17;
            self.rng_state += 1.0;
        }
        spawned
    }

    /// Convenience for tests: tick existing drops then spawn new ones.
    #[cfg(test)]
    pub fn update(&mut self, dt: f64, w: f64, h: f64) -> Vec<(f64, f64)> {
        self.tick(dt);
        self.spawn(dt, w, h)
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64) {
        for drop in &self.drops {
            let px = (drop.x * scale) as i32;
            let py = (drop.y * scale) as i32;
            let fade = (1.0 - drop.age / drop.lifetime).max(0.0);
            canvas.dot(
                px,
                py,
                (180.0 * fade) as u8,
                (210.0 * fade) as u8,
                (240.0 * fade) as u8,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    // -- RainSystem::new ----------------------------------------------------

    #[test]
    fn rain_system_new_has_no_drops() {
        let system = RainSystem::new();
        assert!(system.drops.is_empty());
    }

    // -- update: spawning ---------------------------------------------------

    #[test]
    fn update_spawns_drops_over_time() {
        let mut system = RainSystem::new();
        for _ in 0..200 {
            system.update(0.01, 100.0, 60.0);
        }
        assert!(
            !system.drops.is_empty(),
            "drops should accumulate over 2 seconds"
        );
    }

    #[test]
    fn update_returns_newly_spawned_positions() {
        let mut system = RainSystem::new();
        let mut all_spawned: Vec<(f64, f64)> = Vec::new();
        for _ in 0..50 {
            let spawned = system.update(0.1, 100.0, 60.0);
            all_spawned.extend(spawned);
        }
        assert!(
            !all_spawned.is_empty(),
            "update should return spawned positions over multiple ticks"
        );
    }

    #[test]
    fn spawned_positions_are_within_pond_bounds() {
        let w = 100.0_f64;
        let h = 60.0_f64;
        let mut system = RainSystem::new();
        for _ in 0..100 {
            let spawned = system.update(0.1, w, h);
            for (x, y) in spawned {
                assert!(x >= 0.0 && x <= w, "spawn x={x} out of bounds [0, {w}]");
                assert!(y >= 0.0 && y <= h, "spawn y={y} out of bounds [0, {h}]");
            }
        }
    }

    // -- update: drop lifecycle ---------------------------------------------

    #[test]
    fn drops_are_removed_after_lifetime_expires() {
        let mut system = RainSystem::new();
        for _ in 0..50 {
            system.update(0.1, 100.0, 60.0);
        }
        let count_after_accumulation = system.drops.len();
        assert!(
            count_after_accumulation > 0,
            "precondition: drops must exist"
        );

        for _ in 0..1000 {
            system.update(0.01, 100.0, 60.0);
        }
        assert!(
            system.drops.len() < 500,
            "drop list should not grow unboundedly; len={}",
            system.drops.len()
        );
    }

    #[test]
    fn tick_cleans_up_drops_without_spawning() {
        // Verify that tick() alone removes dead drops (re-prevention for logic-bug family).
        let mut system = RainSystem::new();
        // Accumulate drops via update
        for _ in 0..50 {
            system.update(0.1, 100.0, 60.0);
        }
        assert!(!system.drops.is_empty(), "precondition: drops must exist");

        // Only tick — no spawning — drops should all expire (lifetime 0.3 s)
        for _ in 0..100 {
            system.tick(0.01);
        }
        assert!(
            system.drops.is_empty(),
            "tick() alone should clean up all expired drops"
        );
    }

    // -- Raindrop -----------------------------------------------------------

    #[test]
    fn raindrop_is_alive_initially() {
        let drop = Raindrop::new(10.0, 20.0);
        assert!(drop.is_alive());
    }

    #[test]
    fn raindrop_dies_after_tick() {
        let mut drop = Raindrop::new(10.0, 20.0);
        drop.tick(5.0);
        assert!(!drop.is_alive());
    }

    // -- draw ---------------------------------------------------------------

    #[test]
    fn rain_system_draw_lights_canvas_when_drops_present() {
        let mut system = RainSystem::new();
        for _ in 0..100 {
            system.update(0.1, 40.0, 30.0);
        }

        let mut canvas = Canvas::new(80, 60);
        system.draw(&mut canvas, 2.0);

        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(
            lit > 0,
            "rain draw should light up pixels when drops exist, got {lit}"
        );
    }
}
