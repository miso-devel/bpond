//! Koi fish: chain-based spine physics, biomechanics-accurate fins,
//! and braille rendering.

use crate::canvas::Canvas;
use crate::food::Food;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Spine constants
// ---------------------------------------------------------------------------

pub const N_SPINE: usize = 40;
const SEG_LEN: f64 = 0.55;
const FREQ: f64 = 1.2;
const BODY_TOTAL: f64 = N_SPINE as f64 * SEG_LEN;

// ---------------------------------------------------------------------------
// Feeding behaviour thresholds
// ---------------------------------------------------------------------------

const EATING_RANGE_SQ: f64 = 6.0;
const CHASE_GAIN_THRESHOLD: f64 = 12.0;
const CHASE_DECEL_DIST: f64 = 15.0;

// ---------------------------------------------------------------------------
// Steering
// ---------------------------------------------------------------------------

const MAX_TURN_CHASE: f64 = 1.0;
const MAX_TURN_DEFAULT: f64 = 0.6;

const APPROACH_RATE_EATING: f64 = 2.0;
const APPROACH_RATE_CHASE: f64 = 4.0;
const APPROACH_RATE_IDLE: f64 = 0.6;

// ---------------------------------------------------------------------------
// Speed multipliers
// ---------------------------------------------------------------------------

const BURST_EATING_BASE: f64 = 0.2;
const BURST_EATING_PECK: f64 = 0.25;
const BURST_CHASE_MAX: f64 = 2.2;
const BURST_RANDOM_SPRINT: f64 = 1.5;

const OFF_SCREEN_MARGIN: f64 = 5.0;

// ---------------------------------------------------------------------------
// Fin geometry (used as compile-time parameter sets)
// ---------------------------------------------------------------------------

struct FinParams {
    spine_pos: f64,
    rest_deg: f64,
    amp_deg: f64,
    len_frac: f64,
    radius: f64,
}

const PECTORAL_FINS: FinParams = FinParams {
    spine_pos: 0.20,
    rest_deg: 15.0,
    amp_deg: 30.0,
    len_frac: 0.12,
    radius: 1.5,
};

const PELVIC_FINS: FinParams = FinParams {
    spine_pos: 0.45,
    rest_deg: 10.0,
    amp_deg: 20.0,
    len_frac: 0.08,
    radius: 1.0,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shortest signed angle from `from` to `to`, in (−π, π].
fn angle_diff(from: f64, to: f64) -> f64 {
    (to - from + PI).rem_euclid(2.0 * PI) - PI
}

/// Body half-width at normalized position (0 = head, 1 = tail).
fn body_width(s: f64) -> f64 {
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

struct NearestFood {
    dist_sq: f64,
    x: f64,
    y: f64,
}

// ---------------------------------------------------------------------------
// Koi
// ---------------------------------------------------------------------------

/// A single koi fish with chain-dynamics spine.
pub struct Koi {
    spine_x: [f64; N_SPINE],
    spine_y: [f64; N_SPINE],
    heading: f64,
    speed: f64,
    turn_rate: f64,
    target_turn: f64,
    turn_timer: f64,
    id: f64,
    red_mask: [bool; N_SPINE],
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
        };
        koi.init_spine(x, y);
        koi.init_red_patches();
        koi
    }

    // -- accessors ----------------------------------------------------------

    pub fn head(&self) -> (f64, f64) {
        (self.spine_x[0], self.spine_y[0])
    }

    // -- initialisation helpers ---------------------------------------------

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

    // -- physics sub-steps --------------------------------------------------

    fn nearest_food(&self, foods: &[Food]) -> Option<NearestFood> {
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
            let s1 = ((self.id * 7.3 + t * 3.1).sin() * 1e4).fract();
            let s2 = ((self.id * 11.7 + t * 2.3).cos() * 1e4).fract();
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

    fn propagate_chain(&mut self) {
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

    // -- main update --------------------------------------------------------

    /// Advance the koi by one time step.
    pub fn update(&mut self, dt: f64, t: f64, w: f64, h: f64, foods: &[Food]) {
        let nearest = self.nearest_food(foods);
        let dist_sq = nearest.as_ref().map(|f| f.dist_sq).unwrap_or(f64::MAX);
        let eating = dist_sq < EATING_RANGE_SQ;
        let chasing = !eating && nearest.is_some();

        // --- steering decision ---
        if eating {
            let food = nearest.as_ref().unwrap();
            let toward = (food.y - self.spine_y[0]).atan2(food.x - self.spine_x[0]);
            let orbit = (t * 0.7 + self.id * 2.3).sin() * 0.9;
            self.target_turn = angle_diff(self.heading, toward + orbit).clamp(-0.6, 0.6);
            self.turn_timer = 0.5;
        } else if chasing {
            let food = nearest.as_ref().unwrap();
            let toward = (food.y - self.spine_y[0]).atan2(food.x - self.spine_x[0]);
            let diff = angle_diff(self.heading, toward);
            let dist = dist_sq.sqrt();
            let gain = if dist > CHASE_GAIN_THRESHOLD {
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
        let burst = if eating {
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

    // =======================================================================
    // Drawing
    // =======================================================================

    fn tangent_at(&self, i: usize) -> (f64, f64) {
        let i2 = (i + 1).min(N_SPINE - 1);
        let i1 = i.saturating_sub(1);
        let dx = self.spine_x[i1] - self.spine_x[i2];
        let dy = self.spine_y[i1] - self.spine_y[i2];
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        (dx / l, dy / l)
    }

    fn normal_at(&self, i: usize) -> (f64, f64) {
        let (tx, ty) = self.tangent_at(i);
        (-ty, tx)
    }

    fn to_px(wx: f64, wy: f64, scale: f64) -> (i32, i32) {
        ((wx * scale) as i32, (wy * scale) as i32)
    }

    pub fn draw(&self, canvas: &mut Canvas, t: f64, scale: f64) {
        self.draw_shadow(canvas, scale);
        self.draw_tail(canvas, scale);
        self.draw_fin_pair(canvas, t, scale, &PECTORAL_FINS);
        self.draw_fin_pair(canvas, t, scale, &PELVIC_FINS);
        self.draw_body(canvas, scale);
    }

    fn draw_shadow(&self, canvas: &mut Canvas, scale: f64) {
        for i in (0..N_SPINE).step_by(2) {
            let s = i as f64 / N_SPINE as f64;
            let hw = body_width(s) * 0.7;
            let (nx, ny) = self.normal_at(i);
            let steps = (hw * scale * 1.2) as i32 + 1;
            for pi in -steps..=steps {
                let p = pi as f64 / (steps as f64 / hw);
                if p.abs() > hw {
                    continue;
                }
                let (px, py) =
                    Self::to_px(self.spine_x[i] + nx * p, self.spine_y[i] + ny * p, scale);
                canvas.dot(px + 3, py + 5, 3, 6, 12);
            }
        }
    }

    fn draw_tail(&self, canvas: &mut Canvas, scale: f64) {
        for lobe in [-1.0f64, 1.0] {
            for ti in 0..20 {
                let ft = ti as f64 / 20.0;
                let idx = (N_SPINE - 7 + (ft * 6.0) as usize).min(N_SPINE - 1);
                let (nx, ny) = self.normal_at(idx);
                let spread = lobe * (0.3 + ft * 2.8);
                let (px, py) = Self::to_px(
                    self.spine_x[idx] + nx * spread,
                    self.spine_y[idx] + ny * spread,
                    scale,
                );
                let a = (1.0 - ft * 0.3) * 0.55;
                canvas.thick(
                    px,
                    py,
                    (225.0 * a) as u8,
                    (215.0 * a) as u8,
                    (195.0 * a) as u8,
                );
            }
        }
    }

    fn draw_body(&self, canvas: &mut Canvas, scale: f64) {
        for i in 0..N_SPINE {
            let s = i as f64 / N_SPINE as f64;
            let hw = body_width(s);
            let (nx, ny) = self.normal_at(i);
            let steps = (hw * scale * 2.0) as i32 + 1;
            for pi in -steps..=steps {
                let p = pi as f64 / (steps as f64 / hw);
                if p.abs() > hw {
                    continue;
                }
                let np = (p / hw).abs();
                let (px, py) =
                    Self::to_px(self.spine_x[i] + nx * p, self.spine_y[i] + ny * p, scale);
                let (r, g, b) = if np > 0.78 {
                    (30, 25, 18)
                } else if self.red_mask[i] && np < 0.72 {
                    (235, 45, 25)
                } else {
                    (255, 252, 242)
                };
                canvas.fat(px, py, r, g, b);
            }
        }
    }

    fn draw_fin_pair(&self, canvas: &mut Canvas, t: f64, scale: f64, params: &FinParams) {
        let idx = (N_SPINE as f64 * params.spine_pos) as usize;
        if idx >= N_SPINE {
            return;
        }
        let (nx, ny) = self.normal_at(idx);
        let (tx, ty) = self.tangent_at(idx);
        let rest = params.rest_deg.to_radians();
        let amp = params.amp_deg.to_radians();
        let fin_len = BODY_TOTAL * params.len_frac;

        for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
            let phase = if is_left { 0.0 } else { PI };
            let angle = rest + amp * (2.0 * PI * FREQ * t + phase).sin();
            for fi in 0..12 {
                let ft = fi as f64 / 12.0;
                let spread = side * (angle.sin() * (1.0 - ft * 0.5)) * params.radius;
                let along = -ft * fin_len;
                let wx = self.spine_x[idx] + nx * spread + tx * along;
                let wy = self.spine_y[idx] + ny * spread + ty * along;
                let (px, py) = Self::to_px(wx, wy, scale);
                let a = (1.0 - ft) * 0.5;
                canvas.thick(
                    px,
                    py,
                    (210.0 * a) as u8,
                    (200.0 * a) as u8,
                    (182.0 * a) as u8,
                );
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

    // -- angle_diff ---------------------------------------------------------

    #[test]
    fn angle_diff_shortest_path_positive() {
        let diff = angle_diff(0.0, PI / 2.0);
        assert!((diff - PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_shortest_path_negative() {
        let diff = angle_diff(0.0, -PI / 2.0);
        assert!((diff + PI / 2.0).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_wraps_around() {
        // From 0.1 to 2π-0.1: shortest path is -0.2 (go backward)
        let diff = angle_diff(0.1, 2.0 * PI - 0.1);
        assert!((diff + 0.2).abs() < 1e-10);
    }

    #[test]
    fn angle_diff_opposite_direction() {
        let diff = angle_diff(0.0, PI);
        assert!((diff.abs() - PI).abs() < 1e-10);
    }

    // -- body_width ---------------------------------------------------------

    #[test]
    fn body_width_zero_at_head_tip() {
        assert_eq!(body_width(0.0), 0.0);
    }

    #[test]
    fn body_width_widest_in_middle() {
        let mid = body_width(0.20);
        let head = body_width(0.02);
        let tail = body_width(0.90);
        assert!(mid > head);
        assert!(mid > tail);
    }

    #[test]
    fn body_width_zero_at_tail() {
        assert_eq!(body_width(1.0), 0.0);
    }

    #[test]
    fn body_width_plateau_region() {
        let a = body_width(0.20);
        let b = body_width(0.30);
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
        // heading=0 → spine extends to the left (negative x)
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

    // -- propagate_chain ----------------------------------------------------

    #[test]
    fn propagate_chain_maintains_max_distance() {
        let mut koi = Koi::new(50.0, 30.0, 0.0, 5.0, 1.0);
        // Move head far away
        koi.spine_x[0] = 100.0;
        koi.spine_y[0] = 100.0;
        koi.propagate_chain();

        for i in 1..N_SPINE {
            let dx = koi.spine_x[i] - koi.spine_x[i - 1];
            let dy = koi.spine_y[i] - koi.spine_y[i - 1];
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(
                dist <= SEG_LEN + 1e-10,
                "segment {i} distance {dist} exceeds SEG_LEN {SEG_LEN}"
            );
        }
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
        // Koi facing right (heading=0), food is above (positive y in our coords)
        let mut koi = Koi::new(50.0, 30.0, 0.0, 7.0, 1.0);
        let foods = vec![Food::new(50.0, 50.0)];

        let heading_before = koi.heading;
        for _ in 0..60 {
            koi.update(0.016, 0.0, 100.0, 100.0, &foods);
        }
        let heading_after = koi.heading;

        // heading should have increased (turning toward positive y)
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

        // At least some cells should contain braille characters (U+2800–U+28FF)
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

            // Normal should be perpendicular to tangent
            let dot = tx * nx + ty * ny;
            assert!(
                dot.abs() < 0.01,
                "normal should be perpendicular to tangent at {i}"
            );
        }
    }
}
