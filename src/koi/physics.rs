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

    /// Apply a head→tail traveling curvature wave along the spine, snapping
    /// each segment to exactly `SEG_LEN` from its predecessor. The amplitude
    /// grows toward the tail and is modulated by a slow "breath" oscillation
    /// plus current thrust — a faster fish bends more.
    pub(super) fn animate_body(&mut self, t: f64) {
        let breath = 1.0 + BREATH_AMP * (t * BREATH_FREQ + self.id).sin();
        let thrust = 0.6 + 0.4 * self.burst.clamp(0.2, 2.5);
        for i in 1..N_SPINE {
            let dx = self.spine_x[i] - self.spine_x[i - 1];
            let dy = self.spine_y[i] - self.spine_y[i - 1];
            let cur_angle = dy.atan2(dx);

            let s = i as f64 / N_SPINE as f64;
            // shape(s): zero at head, peak just past mid-body, taper at tip.
            let shape = (s.powf(1.4) * (1.0 - 0.25 * s)).max(0.0);
            let amp = BODY_WAVE_AMP * shape * breath * thrust;
            let phase = 2.0 * PI * FREQ * t - 2.0 * PI * s * BODY_WAVELENGTH + self.id;
            let new_angle = cur_angle + amp * phase.sin();

            self.spine_x[i] = self.spine_x[i - 1] + new_angle.cos() * SEG_LEN;
            self.spine_y[i] = self.spine_y[i - 1] + new_angle.sin() * SEG_LEN;
        }
    }

    /// Boids-style schooling delta-heading. Three forces (separation,
    /// alignment, cohesion) are summed with conservative weights so the
    /// koi school feels coherent without overpowering food chasing or
    /// scare flight. `others` is `(x, y, heading)` for every other koi
    /// in the pond, including this one (skipped via `my_idx`).
    fn schooling_delta(&self, others: &[(f64, f64, f64)], my_idx: usize) -> f64 {
        let (hx, hy) = (self.spine_x[0], self.spine_y[0]);
        let (mut sep_x, mut sep_y) = (0.0, 0.0);
        let mut sep_n = 0;
        let mut align_h = 0.0;
        let mut align_n = 0;
        let (mut coh_x, mut coh_y) = (0.0, 0.0);
        let mut coh_n = 0;

        for (i, &(ox, oy, oh)) in others.iter().enumerate() {
            if i == my_idx {
                continue;
            }
            let dx = ox - hx;
            let dy = oy - hy;
            let dist = (dx * dx + dy * dy).sqrt();
            if !(0.001..=NEIGHBOR_RADIUS).contains(&dist) {
                continue;
            }
            if dist < SEPARATION_RADIUS {
                sep_x -= dx / dist;
                sep_y -= dy / dist;
                sep_n += 1;
            }
            align_h += oh;
            align_n += 1;
            coh_x += ox;
            coh_y += oy;
            coh_n += 1;
        }

        let mut delta = 0.0;
        if sep_n > 0 {
            let target = sep_y.atan2(sep_x);
            delta += angle_diff(self.heading, target) * W_SEPARATION;
        }
        if align_n > 0 {
            let avg = align_h / align_n as f64;
            delta += angle_diff(self.heading, avg) * W_ALIGNMENT;
        }
        if coh_n > 0 {
            let cx = coh_x / coh_n as f64;
            let cy = coh_y / coh_n as f64;
            let target = (cy - hy).atan2(cx - hx);
            delta += angle_diff(self.heading, target) * W_COHESION;
        }
        delta.clamp(-0.6, 0.6)
    }

    /// Curiosity: if any in-range neighbor is heading toward the same food
    /// I'd see, nudge my target toward that food too. Produces the
    /// "follow-the-leader" reaction when one koi breaks for food.
    fn curiosity_delta(
        &self,
        nearest_food: Option<&NearestFood>,
        others: &[(f64, f64, f64)],
        my_idx: usize,
    ) -> f64 {
        let food = match nearest_food {
            Some(f) if f.dist_sq < (NEIGHBOR_RADIUS * 2.5).powi(2) => f,
            _ => return 0.0,
        };
        let (hx, hy) = (self.spine_x[0], self.spine_y[0]);
        let toward_food = (food.y - hy).atan2(food.x - hx);
        for (i, &(ox, oy, oh)) in others.iter().enumerate() {
            if i == my_idx {
                continue;
            }
            let dx = ox - hx;
            let dy = oy - hy;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq > NEIGHBOR_RADIUS * NEIGHBOR_RADIUS {
                continue;
            }
            let n_to_food = (food.y - oy).atan2(food.x - ox);
            if angle_diff(oh, n_to_food).abs() < 0.5 {
                return angle_diff(self.heading, toward_food) * W_CURIOSITY;
            }
        }
        0.0
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f64,
        t: f64,
        w: f64,
        h: f64,
        foods: &[Food],
        others: &[(f64, f64, f64)],
        my_idx: usize,
    ) {
        let sub_dt = dt / SUBSTEPS as f64;
        for s in 0..SUBSTEPS {
            let sub_t = t - dt + sub_dt * (s as f64 + 1.0);
            self.step(sub_dt, sub_t, w, h, foods, others, my_idx);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        dt: f64,
        t: f64,
        w: f64,
        h: f64,
        foods: &[Food],
        others: &[(f64, f64, f64)],
        my_idx: usize,
    ) {
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
            // Minimal orbit — just a tiny wiggle while pecking, not a wide
            // arc around the food.
            let orbit = (t * 0.7 + self.id * 2.3).sin() * 0.15;
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
            self.target_turn = (diff * gain).clamp(-MAX_TURN_CHASE, MAX_TURN_CHASE);
            self.turn_timer = 0.5;
        } else {
            self.steer_idle(dt, t);
            // Schooling and curiosity only modulate idle wandering — not
            // food chasing, not scare flight (those override behavior).
            self.target_turn += self.schooling_delta(others, my_idx);
            self.target_turn += self.curiosity_delta(nearest.as_ref(), others, my_idx);
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

        // --- swimming undulation: small head yaw atop the body wave ---
        // The body wave (animate_body) provides the bulk of side-to-side
        // motion; the head only wags slightly as a reaction to it.
        let swim_wave = (t * 2.0 * PI * FREQ).sin() * 0.05;
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
            // Idle: glide-and-pause. Real koi alternate between gentle
            // forward drift and near-still hovers.
            let lazy = (t * IDLE_SPEED_FREQ + self.id * 1.7).sin();
            if lazy < PAUSE_THRESHOLD {
                PAUSE_BURST
            } else {
                1.0 + IDLE_SPEED_AMP * lazy
            }
        };
        self.burst = burst;
        self.spine_x[0] += self.heading.cos() * self.speed * burst * dt;
        self.spine_y[0] += self.heading.sin() * self.speed * burst * dt;

        // --- chain dynamics + body wave ---
        self.animate_body(t);
    }
}
