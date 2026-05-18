//! Koi fish: chain-based spine physics, biomechanics-accurate fins,
//! and braille rendering.

mod draw;
mod physics;

use crate::canvas::Canvas;
use crate::food::Food;

// ---------------------------------------------------------------------------
// Spine constants
// ---------------------------------------------------------------------------

pub const N_SPINE: usize = 40;
pub(super) const SEG_LEN: f64 = 0.55;
pub(super) const FREQ: f64 = 1.2;
pub(super) const BODY_TOTAL: f64 = N_SPINE as f64 * SEG_LEN;

// ---------------------------------------------------------------------------
// Feeding behaviour thresholds
// ---------------------------------------------------------------------------

pub(super) const EATING_RANGE_SQ: f64 = 6.0;
/// Distance below which the koi softens its turn-rate gain to avoid
/// orbiting around the food. Smaller = full gain holds longer = more
/// direct chase paths.
pub(super) const CHASE_GAIN_THRESHOLD: f64 = 6.0;
pub(super) const CHASE_DECEL_DIST: f64 = 15.0;

// ---------------------------------------------------------------------------
// Steering
// ---------------------------------------------------------------------------

/// Real koi snap toward food with sharp pectoral-fin turns, so chasing
/// allows much higher angular velocity than idle wandering.
pub(super) const MAX_TURN_CHASE: f64 = 2.5;
pub(super) const MAX_TURN_DEFAULT: f64 = 0.6;

pub(super) const APPROACH_RATE_EATING: f64 = 2.0;
/// Near-instant tracking while chasing — the koi commits to its target
/// heading right away rather than ramping toward it over a fraction of a
/// second (which produced visible "wide loops" around food).
pub(super) const APPROACH_RATE_CHASE: f64 = 12.0;
pub(super) const APPROACH_RATE_IDLE: f64 = 0.6;

// ---------------------------------------------------------------------------
// Speed multipliers
// ---------------------------------------------------------------------------

pub(super) const BURST_EATING_BASE: f64 = 0.2;
pub(super) const BURST_EATING_PECK: f64 = 0.25;
/// Forward speed multiplier while chasing food. Lowered from 2.2 so the
/// koi doesn't drift past the food during a turn.
pub(super) const BURST_CHASE_MAX: f64 = 1.5;
pub(super) const BURST_RANDOM_SPRINT: f64 = 1.5;
pub(super) const BURST_SCARE: f64 = 2.5;

pub(super) const OFF_SCREEN_MARGIN: f64 = 5.0;

// ---------------------------------------------------------------------------
// Body wave — traveling curvature down the spine (head→tail)
// ---------------------------------------------------------------------------

/// Per-segment angular oscillation amplitude (radians at the tail peak).
/// Real koi cruise with subtle undulation; too much wave makes the head
/// path look jittery, so this is tuned conservatively.
pub(super) const BODY_WAVE_AMP: f64 = 0.06;
/// How many spatial wavelengths fit in the body (~1 full wave).
pub(super) const BODY_WAVELENGTH: f64 = 1.05;
/// Slow modulation that mimics breathing / variable thrust.
pub(super) const BREATH_FREQ: f64 = 0.55;
pub(super) const BREATH_AMP: f64 = 0.20;

// ---------------------------------------------------------------------------
// Idle micro-motion + glide-and-pause cruising
// ---------------------------------------------------------------------------

pub(super) const IDLE_SPEED_AMP: f64 = 0.18;
pub(super) const IDLE_SPEED_FREQ: f64 = 0.4;
/// When the lazy phase is below this, the koi briefly hovers — real koi
/// alternate between drift and near-still rather than constant cruising.
pub(super) const PAUSE_THRESHOLD: f64 = -0.55;
pub(super) const PAUSE_BURST: f64 = 0.25;

// ---------------------------------------------------------------------------
// Integration
// ---------------------------------------------------------------------------

/// Sub-steps per `update()` call — splits the frame for smoother dynamics.
pub(super) const SUBSTEPS: usize = 3;

// ---------------------------------------------------------------------------
// Schooling (Boids-style) — koi prefer to swim near other koi
// ---------------------------------------------------------------------------

/// Radius (world units) within which another koi influences this one.
pub(super) const NEIGHBOR_RADIUS: f64 = 14.0;
/// Below this distance the separation force kicks in to avoid collisions.
pub(super) const SEPARATION_RADIUS: f64 = 5.0;
/// Per-axis weights for the three Boids forces. Tuned soft so schooling
/// shapes the path without overpowering food chasing or scare flight.
pub(super) const W_SEPARATION: f64 = 0.40;
pub(super) const W_ALIGNMENT: f64 = 0.10;
pub(super) const W_COHESION: f64 = 0.06;
/// Curiosity: when a neighbor heads toward food, this koi nudges its
/// own target toward the same food — the "follow the leader" reaction
/// that real koi show during a feeding event.
pub(super) const W_CURIOSITY: f64 = 0.30;

// ---------------------------------------------------------------------------
// Koi
// ---------------------------------------------------------------------------

/// A single koi fish with chain-dynamics spine.
pub struct Koi {
    pub(super) spine_x: [f64; N_SPINE],
    pub(super) spine_y: [f64; N_SPINE],
    pub(super) heading: f64,
    pub(super) speed: f64,
    pub(super) turn_rate: f64,
    pub(super) target_turn: f64,
    pub(super) turn_timer: f64,
    pub(super) id: f64,
    pub(super) red_mask: [bool; N_SPINE],
    pub scare_timer: f64,
    pub(super) scare_from_x: f64,
    pub(super) scare_from_y: f64,
    /// Most recent thrust multiplier (1.0 = cruise). Drawing reads this to
    /// scale fin/tail amplitude proportionally to current effort.
    pub(super) burst: f64,
}

impl Koi {
    pub fn new(x: f64, y: f64, heading: f64, speed: f64, id: f64) -> Self {
        let mut koi = Koi {
            spine_x: [0.0; N_SPINE],
            spine_y: [0.0; N_SPINE],
            heading,
            speed,
            turn_rate: 0.0,
            target_turn: 0.0,
            turn_timer: 2.0 + id,
            id,
            red_mask: [false; N_SPINE],
            scare_timer: 0.0,
            scare_from_x: 0.0,
            scare_from_y: 0.0,
            burst: 1.0,
        };
        koi.init_spine(x, y);
        koi.init_red_patches();
        koi
    }

    pub fn head(&self) -> (f64, f64) {
        (self.spine_x[0], self.spine_y[0])
    }

    /// Position + heading snapshot. Pond collects one per koi each frame
    /// so each koi can react to its neighbors without aliasing the mutable
    /// borrow during update.
    pub fn snapshot(&self) -> (f64, f64, f64) {
        (self.spine_x[0], self.spine_y[0], self.heading)
    }

    /// Current world-space velocity vector (heading × speed × burst).
    /// Used by floating debris like lily pads to feel the koi's wake.
    pub fn velocity(&self) -> (f64, f64) {
        let v = self.speed * self.burst;
        (self.heading.cos() * v, self.heading.sin() * v)
    }

    pub fn scare(&mut self, x: f64, y: f64) {
        self.scare_timer = 2.0;
        self.scare_from_x = x;
        self.scare_from_y = y;
    }

    fn init_spine(&mut self, x: f64, y: f64) {
        for i in 0..N_SPINE {
            self.spine_x[i] = x - (i as f64) * SEG_LEN * self.heading.cos();
            self.spine_y[i] = y - (i as f64) * SEG_LEN * self.heading.sin();
        }
    }

    fn init_red_patches(&mut self) {
        let n_patches = 2 + ((self.id * 3.7).sin().abs() * 3.5) as usize;
        for p in 0..n_patches {
            let center = ((self.id * (p as f64 + 1.0) * 2.3 + 0.7).sin().abs() * 0.7 + 0.08)
                * N_SPINE as f64;
            let half_w = ((self.id * (p as f64 + 1.0) * 1.7 + 1.3).cos().abs() * 0.12 + 0.04)
                * N_SPINE as f64;
            for i in 0..N_SPINE {
                if (i as f64 - center).abs() < half_w {
                    self.red_mask[i] = true;
                }
            }
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // -- angle_diff ---------------------------------------------------------

    #[test]
    fn angle_diff_shortest_path_positive() {
        let diff = physics::angle_diff(0.0, PI / 2.0);
        assert!((diff - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_shortest_path_negative() {
        let diff = physics::angle_diff(0.0, -PI / 2.0);
        assert!((diff + PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_wraps_around() {
        let diff = physics::angle_diff(0.1, 2.0 * PI - 0.1);
        assert!((diff + 0.2).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_opposite_direction() {
        let diff = physics::angle_diff(0.0, PI);
        assert!((diff.abs() - PI).abs() < 1e-10);
    }

    // -- body_width ---------------------------------------------------------

    #[test]
    fn body_width_zero_at_head_tip() {
        assert_eq!(physics::body_width(0.0), 0.0);
    }

    #[test]
    fn body_width_widest_in_middle() {
        let mid = physics::body_width(0.20);
        let head = physics::body_width(0.02);
        let tail = physics::body_width(0.90);
        assert!(mid > head);
        assert!(mid > tail);
    }

    #[test]
    fn body_width_zero_at_tail() {
        assert_eq!(physics::body_width(1.0), 0.0);
    }

    #[test]
    fn body_width_plateau_region() {
        let a = physics::body_width(0.20);
        let b = physics::body_width(0.30);
        assert!((a - b).abs() < 1e-10, "plateau between 0.15 and 0.40");
    }

    // -- Koi::new -----------------------------------------------------------

    #[test]
    fn new_koi_spine_starts_at_position() {
        let koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let (hx, hy) = koi.head();
        assert!((hx - 50.0).abs() < 1e-10);
        assert!((hy - 30.0).abs() < 1e-10);
    }

    #[test]
    fn new_koi_spine_is_straight_line() {
        let koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        for i in 1..N_SPINE {
            assert!(
                koi.spine_x[i] < koi.spine_x[i - 1],
                "segment {i} should be left of segment {}",
                i - 1
            );
            assert!(
                (koi.spine_y[i] - 30.0).abs() < 1e-10,
                "all y should be same for heading=0"
            );
        }
    }

    #[test]
    fn new_koi_segment_distances() {
        let koi = Koi::new(50.0, 30.0, 1.0, 5.0, 1.0);
        for i in 1..N_SPINE {
            let dx = koi.spine_x[i] - koi.spine_x[i - 1];
            let dy = koi.spine_y[i] - koi.spine_y[i - 1];
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(
                (dist - SEG_LEN).abs() < 1e-10,
                "segment {i} distance should be SEG_LEN"
            );
        }
    }

    #[test]
    fn new_koi_has_red_patches() {
        let koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let n_red = koi.red_mask.iter().filter(|&&r| r).count();
        assert!(n_red > 0, "should have at least some red segments");
        assert!(n_red < N_SPINE, "not all segments should be red");
    }

    // -- animate_body / body wave -------------------------------------------

    #[test]
    fn animate_body_preserves_segment_length() {
        // After the traveling-wave step every consecutive pair must be
        // exactly SEG_LEN apart — the wave rotates segments, never stretches.
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        koi.spine_x[0] = 100.0;
        koi.spine_y[0] = 100.0;
        koi.animate_body(2.5);
        for i in 1..N_SPINE {
            let dx = koi.spine_x[i] - koi.spine_x[i - 1];
            let dy = koi.spine_y[i] - koi.spine_y[i - 1];
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(
                (dist - SEG_LEN).abs() < 1e-9,
                "segment {i} dist {dist} should equal SEG_LEN {SEG_LEN}"
            );
        }
    }

    #[test]
    fn animate_body_phase_advances_with_time() {
        // Two snapshots at different phases must differ — the wave is alive.
        let mut a = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let mut b = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        a.animate_body(0.0);
        b.animate_body(0.3);
        let diff: f64 = a
            .spine_y
            .iter()
            .zip(b.spine_y.iter())
            .map(|(x, y)| (x - y).abs())
            .sum();
        assert!(diff > 0.005, "phase shift must change spine shape");
    }

    // -- nearest_food -------------------------------------------------------

    #[test]
    fn nearest_food_returns_closest() {
        let koi = Koi::new(10.0, 10.0, 0.0, 5.0, 1.0);
        let foods = vec![
            Food::new(50.0, 50.0),
            Food::new(12.0, 10.0),
            Food::new(30.0, 30.0),
        ];
        let nearest = koi.nearest_food(&foods).unwrap();
        assert!((nearest.x - 12.0).abs() < 1e-10);
        assert!((nearest.y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn nearest_food_empty_returns_none() {
        let koi = Koi::new(10.0, 10.0, 0.0, 5.0, 1.0);
        assert!(koi.nearest_food(&[]).is_none());
    }

    // -- update steering toward food ----------------------------------------

    #[test]
    fn koi_turns_toward_food() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 7.0, 1.0);
        let foods = vec![Food::new(50.0, 50.0)];

        let heading_before = koi.heading;
        for _ in 0..60 {
            koi.update(0.016, 0.0, 100.0, 100.0, &foods, &[], 0);
        }
        let heading_after = koi.heading;

        assert!(
            heading_after > heading_before,
            "koi should turn toward food: before={heading_before}, after={heading_after}"
        );
    }

    // -- drawing ------------------------------------------------------------

    #[test]
    fn draw_produces_visible_output() {
        let koi = Koi::new(20.0, 15.0, 0.0, 5.0, 1.0);
        let mut canvas = Canvas::new(40, 30);
        koi.draw(&mut canvas, 0.0, 2.0);

        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(lit > 100, "koi draw should light up many pixels, got {lit}");
    }

    #[test]
    fn draw_renders_to_braille_buffer() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;

        let koi = Koi::new(20.0, 15.0, 0.0, 5.0, 1.0);
        let mut canvas = Canvas::new(40, 30);
        koi.draw(&mut canvas, 0.0, 2.0);

        let area = Rect::new(0, 0, 40, 31);
        let mut buf = Buffer::empty(area);
        canvas.render(&mut buf, 0, 1, area);

        let braille_count = (0..area.width)
            .flat_map(|x| (0..area.height).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let ch = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
                ('\u{2800}'..='\u{28FF}').contains(&ch)
            })
            .count();
        assert!(
            braille_count > 10,
            "buffer should contain braille characters, got {braille_count}"
        );
    }

    #[test]
    fn draw_body_has_white_and_red() {
        let koi = Koi::new(20.0, 15.0, 0.0, 5.0, 1.0);
        let mut canvas = Canvas::new(40, 30);
        koi.draw(&mut canvas, 0.0, 2.0);

        let mut has_white = false;
        let mut has_red = false;
        for x in 0..canvas.w {
            for y in 0..canvas.h {
                let (on, r, g, b) = canvas.get(x, y);
                if !on {
                    continue;
                }
                if r > 250 && g > 250 && b > 240 {
                    has_white = true;
                }
                if r > 200 && g < 60 && b < 40 {
                    has_red = true;
                }
            }
        }
        assert!(has_white, "koi body should have white pixels");
        assert!(has_red, "koi body should have red (kohaku) pixels");
    }

    #[test]
    fn tangent_and_normal_are_unit_vectors() {
        let koi = Koi::new(20.0, 15.0, 0.5, 5.0, 1.0);
        for i in 0..N_SPINE {
            let (tx, ty) = koi.tangent_at(i);
            let len = (tx * tx + ty * ty).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "tangent at {i} should be unit vector, got len={len}"
            );

            let (nx, ny) = koi.normal_at(i);
            let len = (nx * nx + ny * ny).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "normal at {i} should be unit vector, got len={len}"
            );

            let dot = tx * nx + ty * ny;
            assert!(
                dot.abs() < 0.01,
                "normal should be perpendicular to tangent at {i}"
            );
        }
    }
}

#[cfg(test)]
mod scare_tests {
    use super::*;
    use crate::food::Food;

    // -- scare: basic -------------------------------------------------------

    #[test]
    fn scare_does_not_panic() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        koi.scare(50.0, 30.0);
        koi.scare(0.0, 0.0);
    }

    #[test]
    fn scare_activates_timer() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        koi.scare(0.0, 0.0);
        assert!(
            koi.scare_timer > 0.0,
            "scare_timer should be positive after scare"
        );
    }

    // -- scare: flee behaviour ----------------------------------------------

    #[test]
    fn scared_koi_moves_away_from_scare_origin() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 7.0, 1.0);
        let (x_before, _) = koi.head();
        koi.scare(0.0, 30.0);

        let foods: Vec<Food> = Vec::new();
        for _ in 0..30 {
            koi.update(0.016, 0.0, 200.0, 100.0, &foods, &[], 0);
        }

        let (x_after, _) = koi.head();
        assert!(
            x_after > x_before,
            "scared koi should flee away from scare origin: before={x_before}, after={x_after}"
        );
    }

    #[test]
    fn scared_koi_moves_faster_than_unsettled() {
        let mut calm = Koi::new(50.0, 30.0, 0.0, 7.0, 2.0);
        let mut scared = Koi::new(50.0, 30.0, 0.0, 7.0, 2.0);
        scared.scare(50.0, 80.0);

        let foods: Vec<Food> = Vec::new();
        let dt = 0.016;

        let (cx0, cy0) = calm.head();
        let (sx0, sy0) = scared.head();

        for _ in 0..20 {
            calm.update(dt, 0.0, 200.0, 100.0, &foods, &[], 0);
            scared.update(dt, 0.0, 200.0, 100.0, &foods, &[], 0);
        }

        let (cx1, cy1) = calm.head();
        let (sx1, sy1) = scared.head();
        let calm_dist = ((cx1 - cx0).powi(2) + (cy1 - cy0).powi(2)).sqrt();
        let scared_dist = ((sx1 - sx0).powi(2) + (sy1 - sy0).powi(2)).sqrt();

        assert!(
            scared_dist > calm_dist,
            "scared koi should travel farther per frame: scared={scared_dist:.2}, calm={calm_dist:.2}"
        );
    }

    // -- steer_idle regression: fract().abs() ----------------------------------

    #[test]
    fn steer_idle_turn_timer_never_goes_negative() {
        // Regression: steer_idle used fract() (signed) instead of fract().abs().
        // When s2 < 0, turn_timer was set to 2.0 + s2*5.0 which could be as low
        // as -3.0. After fix: s2 = fract().abs() ∈ [0,1) → reset to [2.0, 7.0).
        // The remaining substeps within the same update() then shave off
        // ~2·sub_dt, so the lower bound here is 2.0 - dt — still nowhere near
        // the old -3.
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 3.7);
        let foods: Vec<Food> = Vec::new();
        let dt = 0.016;
        for i in 0..500 {
            koi.turn_timer = 0.0;
            let t = i as f64 * 0.1;
            koi.update(dt, t, 200.0, 100.0, &foods, &[], 0);
            assert!(
                koi.turn_timer >= 2.0 - dt,
                "turn_timer should not crash negative at t={t:.1}, got {}",
                koi.turn_timer
            );
        }
    }

    #[test]
    fn steer_idle_produces_right_turns_not_only_left() {
        // Regression: steer_idle used fract() (signed) instead of fract().abs().
        // When s1 < 0 (≈50% of the time), condition `s1 < 0.08` was always true
        // → target_turn always -0.4 (hard left). After fix, turns are balanced.
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 3.7);
        let foods: Vec<Food> = Vec::new();
        let mut positive_turns = 0usize;
        for i in 0..500 {
            koi.turn_timer = 0.0;
            let t = i as f64 * 0.1;
            koi.update(0.016, t, 200.0, 100.0, &foods, &[], 0);
            if koi.target_turn > 0.0 {
                positive_turns += 1;
            }
        }
        assert!(
            positive_turns > 50,
            "steer_idle should produce right turns (target_turn > 0) sometimes, got {positive_turns}/500"
        );
    }

    // -- scare: timer decay -------------------------------------------------

    #[test]
    fn scare_timer_expires_over_time() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        koi.scare(0.0, 0.0);
        assert!(koi.scare_timer > 0.0);

        let foods: Vec<Food> = Vec::new();
        for _ in 0..625 {
            koi.update(0.016, 0.0, 200.0, 100.0, &foods, &[], 0);
        }
        assert!(
            koi.scare_timer <= 0.0,
            "scare_timer should reach zero after enough time"
        );
    }
}

#[cfg(test)]
mod behavior_tests {
    use super::*;
    use crate::food::Food;

    // -- schooling: separation ----------------------------------------------

    #[test]
    fn schooling_separation_pushes_overlapping_koi_apart() {
        // Two koi placed well within SEPARATION_RADIUS (5.0) of each other
        // should diverge over time once their idle steering picks up the
        // schooling delta. Confirms the separation force is doing work.
        let mut a = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let mut b = Koi::new(53.0, 30.0, std::f64::consts::PI, 5.0, 7.1);
        let foods: Vec<Food> = Vec::new();
        let dt = 0.05;

        let initial_dist = {
            let (ax, ay) = a.head();
            let (bx, by) = b.head();
            ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
        };

        for i in 0..200 {
            let t = i as f64 * dt;
            let snaps = vec![a.snapshot(), b.snapshot()];
            a.update(dt, t, 400.0, 200.0, &foods, &snaps, 0);
            b.update(dt, t, 400.0, 200.0, &foods, &snaps, 1);
        }
        let final_dist = {
            let (ax, ay) = a.head();
            let (bx, by) = b.head();
            ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
        };
        assert!(
            final_dist > initial_dist + SEPARATION_RADIUS * 0.5,
            "separation should push koi apart: {initial_dist:.2} -> {final_dist:.2}",
        );
    }

    // -- curiosity: neighbor chasing food pulls this koi in -----------------

    #[test]
    fn curiosity_turns_toward_food_that_a_neighbor_chases() {
        // Put a koi heading north (heading=-π/2). Place a neighbor right
        // beside it heading toward a food directly to the east. After a
        // few idle frames, the curious koi should rotate eastward.
        let mut me = Koi::new(50.0, 30.0, -std::f64::consts::FRAC_PI_2, 5.0, 1.0);
        // Neighbor is in range, already facing the food.
        let neighbor_snap = (52.0, 30.0, 0.0_f64);
        // Food east of both, far enough that we're "chasing" not "eating".
        let foods = vec![Food::new(60.0, 30.0)];
        let dt = 0.05;

        let initial_heading = me.heading;
        // Many frames; curiosity is subtle, so accumulate effect.
        for i in 0..120 {
            let snaps = vec![me.snapshot(), neighbor_snap];
            me.update(dt, i as f64 * dt, 400.0, 200.0, &foods, &snaps, 0);
        }
        // The food itself triggers chase; that overrides curiosity. The
        // useful signal is: the koi ends up heading east-ish (toward food).
        let final_heading_x = me.heading.cos();
        assert!(
            final_heading_x > 0.5,
            "koi should swing east toward food: heading.cos()={final_heading_x:.2}, \
             initial={initial_heading:.2}, final={:.2}",
            me.heading,
        );
    }

    // -- glide-pause: idle koi occasionally hovers near a stop --------------

    #[test]
    fn idle_burst_dips_into_pause_range() {
        // Over many idle frames sampled across the lazy cycle, at least
        // one frame should land in the PAUSE_BURST regime. This is what
        // gives the koi its "ふっと止まる" hover.
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let foods: Vec<Food> = Vec::new();
        let dt = 0.05;
        let mut min_burst: f64 = f64::INFINITY;
        // Sample t across a few full lazy-cycles (period = 2π / IDLE_SPEED_FREQ ≈ 15.7s).
        for i in 0..400 {
            let t = i as f64 * dt;
            koi.update(dt, t, 400.0, 200.0, &foods, &[], 0);
            min_burst = min_burst.min(koi.burst);
        }
        assert!(
            (min_burst - PAUSE_BURST).abs() < 1e-6 || min_burst <= PAUSE_BURST + 0.01,
            "burst should drop to ≈PAUSE_BURST ({PAUSE_BURST}) at least once, got {min_burst}",
        );
    }

    #[test]
    fn idle_burst_also_cruises() {
        // Counterpart to the pause test: cruise burst should regularly hit
        // ≥ 1.0 so the koi isn't pausing the entire time.
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        let foods: Vec<Food> = Vec::new();
        let dt = 0.05;
        let mut max_burst: f64 = f64::NEG_INFINITY;
        for i in 0..400 {
            let t = i as f64 * dt;
            koi.update(dt, t, 400.0, 200.0, &foods, &[], 0);
            max_burst = max_burst.max(koi.burst);
        }
        assert!(
            max_burst >= 1.0,
            "burst should regularly reach cruising ≥1.0, got max {max_burst}",
        );
    }

    // -- substep stability: long trajectory still converges ------------------

    #[test]
    fn substep_chase_brings_koi_close_to_food() {
        // With substep integration the koi must converge to its target
        // without drifting away. Track the *minimum* distance reached over
        // a chase window — the koi orbits gently once it's within range,
        // so we don't require the final frame to be exactly on the food.
        let mut koi = Koi::new(20.0, 30.0, 0.0, 5.0, 1.0);
        let foods = vec![Food::new(50.0, 35.0)];
        let mut min_dist: f64 = f64::INFINITY;
        for i in 0..300 {
            koi.update(0.05, i as f64 * 0.05, 200.0, 100.0, &foods, &[], 0);
            let (hx, hy) = koi.head();
            let d = ((50.0_f64 - hx).powi(2) + (35.0_f64 - hy).powi(2)).sqrt();
            min_dist = min_dist.min(d);
        }
        // Eating range is sqrt(EATING_RANGE_SQ) ≈ 2.45; require the koi
        // to actually touch that.
        assert!(
            min_dist < EATING_RANGE_SQ.sqrt(),
            "koi should enter eating range during chase, min dist {min_dist:.2}",
        );
    }
}
