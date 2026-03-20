//! Koi physics: steering, chain dynamics, and per-frame update.

use super::*;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Helpers (pub(super) so mod.rs tests can access them)
// ---------------------------------------------------------------------------

pub(super) fn angle_diff(from: f64, to: f64) -> f64 {
    (to - from + PI).rem_euclid(2.0 * PI) - PI
}

pub(super) fn body_width(s: f64) -> f64 {
    let frac = if s < 0.05 {
        s / 0.05 * 0.10
    } else if s < 0.15 {
        0.10 + (s - 0.05) / 0.10 * 0.08
    } else if s < 0.40 {
        0.18
    } else if s < 0.75 {
        0.18 - (s - 0.4) / 0.35 * 0.08
    } else {
        0.10 * (1.0 - s) / 0.25
    };
    frac * BODY_TOTAL
}

// ---------------------------------------------------------------------------
// Internal type returned by nearest-food search
// ---------------------------------------------------------------------------

pub(super) struct NearestFood {
    pub(super) dist_sq: f64,
    pub(super) x: f64,
    pub(super) y: f64,
}

// ---------------------------------------------------------------------------
// Physics impl
// ---------------------------------------------------------------------------

impl Koi {
    pub(super) fn nearest_food(&self, foods: &[Food]) -> Option<NearestFood> {
        foods
            .iter()
            .map(|f| {
                let dx = f.x - self.spine_x[0];
                let dy = f.y - self.spine_y[0];
                NearestFood {
                    dist_sq: dx * dx + dy * dy,
                    x: f.x,
                    y: f.y,
                }
            })
            .min_by(|a, b| a.dist_sq.partial_cmp(&b.dist_sq).unwrap())
    }

    fn steer_idle(&mut self, dt: f64, t: f64) {
        self.turn_timer -= dt;
        if self.turn_timer <= 0.0 {
            let s1 = ((self.id * 7.3 + t * 3.1).sin() * 1e4).fract().abs();
            let s2 = ((self.id * 11.7 + t * 2.3).cos() * 1e4).fract().abs();
            self.target_turn = if s1 > 0.92 {
                0.4
            } else if s1 < 0.08 {
                -0.4
            } else if s1 > 0.75 {
                0.15
            } else if s1 < 0.25 {
                -0.15
            } else {
                0.0
            };
            self.turn_timer = 2.0 + s2 * 5.0;
        }
    }

    fn apply_turn(&mut self, dt: f64, approach_rate: f64, max_turn: f64) {
        let approach = approach_rate * dt;
        if (self.target_turn - self.turn_rate).abs() < approach {
            self.turn_rate = self.target_turn;
        } else if self.target_turn > self.turn_rate {
            self.turn_rate += approach;
        } else {
            self.turn_rate -= approach;
        }
        self.turn_rate = self.turn_rate.clamp(-max_turn, max_turn);
    }

    fn steer_back(&mut self, dt: f64, w: f64, h: f64) {
        let (hx, hy) = self.head();
        let fully_out = hx < -OFF_SCREEN_MARGIN
            || hx > w + OFF_SCREEN_MARGIN
            || hy < -OFF_SCREEN_MARGIN
            || hy > h + OFF_SCREEN_MARGIN;
        if fully_out {
            let toward = (h / 2.0 - hy).atan2(w / 2.0 - hx);
            self.heading += angle_diff(self.heading, toward) * 0.3 * dt;
        }
    }

    pub(super) fn propagate_chain(&mut self) {
        for i in 1..N_SPINE {
            let dx = self.spine_x[i - 1] - self.spine_x[i];
            let dy = self.spine_y[i - 1] - self.spine_y[i];
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > SEG_LEN {
                let ratio = SEG_LEN / dist;
                self.spine_x[i] = self.spine_x[i - 1] - dx * ratio;
                self.spine_y[i] = self.spine_y[i - 1] - dy * ratio;
            }
        }
    }

    pub fn update(&mut self, dt: f64, t: f64, w: f64, h: f64, foods: &[Food]) {
        let nearest = self.nearest_food(foods);
        let dist_sq = nearest.as_ref().map(|f| f.dist_sq).unwrap_or(f64::MAX);
        let eating = dist_sq < EATING_RANGE_SQ;
        let chasing = !eating && nearest.is_some();

        // --- steering decision ---
        if self.scare_timer > 0.0 {
            let away =
                (self.spine_y[0] - self.scare_from_y).atan2(self.spine_x[0] - self.scare_from_x);
            self.target_turn = angle_diff(self.heading, away).clamp(-1.5, 1.5);
            self.scare_timer -= dt;
        } else if eating {
            let food = nearest.as_ref().unwrap();
            let toward = (food.y - self.spine_y[0]).atan2(food.x - self.spine_x[0]);
            let orbit = (t * 0.7 + self.id * 2.3).sin() * 0.9;
            self.target_turn = angle_diff(self.heading, toward + orbit).clamp(-0.6, 0.6);
            self.turn_timer = 0.5;
        } else if chasing {
            let food = nearest.as_ref().unwrap();
            let toward = (food.y - self.spine_y[0]).atan2(food.x - self.spine_x[0]);
            let diff = angle_diff(self.heading, toward);
            let gain = if dist_sq.sqrt() > CHASE_GAIN_THRESHOLD {
                1.0
            } else {
                0.5
            };
            self.target_turn = (diff * gain).clamp(-1.0, 1.0);
            self.turn_timer = 0.5;
        } else {
            self.steer_idle(dt, t);
        }

        // --- smooth turn rate ---
        let (approach_rate, max_turn) = if eating {
            (APPROACH_RATE_EATING, MAX_TURN_DEFAULT)
        } else if chasing {
            (APPROACH_RATE_CHASE, MAX_TURN_CHASE)
        } else {
            (APPROACH_RATE_IDLE, MAX_TURN_DEFAULT)
        };
        self.apply_turn(dt, approach_rate, max_turn);

        // --- swimming undulation ---
        let swim_wave = (t * 2.0 * PI * FREQ).sin() * 0.10;
        self.heading += (self.turn_rate + swim_wave) * dt;

        // --- boundary correction ---
        self.steer_back(dt, w, h);

        // --- forward motion ---
        let burst = if self.scare_timer > 0.0 {
            BURST_SCARE
        } else if eating {
            let peck = ((t * 1.5 + self.id * 1.3).sin().max(0.0)).powi(3);
            BURST_EATING_BASE + peck * BURST_EATING_PECK
        } else if chasing {
            let dist = dist_sq.sqrt();
            if dist > CHASE_DECEL_DIST {
                BURST_CHASE_MAX
            } else {
                1.0 + dist / CHASE_DECEL_DIST * 1.2
            }
        } else if (t * 0.1 + self.id).sin() > 0.97 {
            BURST_RANDOM_SPRINT
        } else {
            1.0
        };
        self.spine_x[0] += self.heading.cos() * self.speed * burst * dt;
        self.spine_y[0] += self.heading.sin() * self.speed * burst * dt;

        // --- chain dynamics ---
        self.propagate_chain();
    }
}
