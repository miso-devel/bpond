//! Lotus pads floating on the pond surface.
//!
//! Each pad is a clean circular disc with a single V-shaped wedge
//! cut from the rim. The wedge tapers from a point at a random inner
//! radius out to its full angular width at the rim, with a small
//! deterministic edge wobble so the cut reads as natural rather than
//! mathematical. Pads also drift around their home positions under
//! spring / damping / ambient current / koi-wake forces.

use crate::canvas::Canvas;
use std::f64::consts::{PI, TAU};

// ===========================================================================
// Tuning constants
// ===========================================================================

mod color {
    pub const FILL: (u8, u8, u8) = (95, 155, 70);
    pub const MID: (u8, u8, u8) = (70, 125, 55);
    pub const EDGE: (u8, u8, u8) = (45, 90, 40);
    pub const HUB: (u8, u8, u8) = (55, 100, 50);
    pub const VEIN: (u8, u8, u8) = (50, 95, 42);
}

const VEIN_COUNT: f64 = 14.0;
const VEIN_HALF_WIDTH: f64 = 0.03;

/// V-wedge geometry tuning. With `WEDGE_HW_MAX = 0.42` and
/// `WEDGE_INNER_NP_MIN = 0.30` the wedge removes ~7% of the disc
/// area; jitter adds at most ~1% more. Comfortably under the 60%
/// upper bound implied by "leaf must stay at least 40% green".
const WEDGE_HW_MIN: f64 = 0.22;
const WEDGE_HW_MAX: f64 = 0.42;
const WEDGE_INNER_NP_MIN: f64 = 0.30;
const WEDGE_INNER_NP_MAX: f64 = 0.65;
/// Peak deterministic edge jitter applied to the wedge half-width
/// (radians). Bounded so the cut can never balloon by more.
const WEDGE_JITTER_AMP: f64 = 0.05;

// Pad radii are large enough that the biggest frog (size MAX with
// half-length ≈ 3.15 wu) fits comfortably inside even the smallest
// pad (diameter 10 wu).
const RADIUS_MIN: f64 = 5.0;
const RADIUS_MAX: f64 = 8.0;

const ROTATION_RATE_MIN: f64 = 0.10;
const ROTATION_RATE_MAX: f64 = 0.30;

/// Drift physics.
const SPRING_K: f64 = 0.18;
const DAMPING: f64 = 0.28;
const AMBIENT_AMP: f64 = 0.32;
const WAKE_RADIUS: f64 = 9.0;
const WAKE_GAIN: f64 = 0.55;

// ===========================================================================
// Free functions
// ===========================================================================

/// Wraparound angular distance (shortest path), in `[0, π]`.
///
/// Accepts any finite inputs — no requirement that arguments be
/// normalised. We map `a - b` into `[0, TAU)` with `rem_euclid`, then
/// fold values above `π` back through TAU.
fn angle_dist(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    if d > PI {
        TAU - d
    } else {
        d
    }
}

/// Global wind drift shared by every pad — a slow, slowly-rotating
/// vector field that gives the pond a coherent direction at any
/// moment.
fn global_wind(t: f64) -> (f64, f64) {
    let angle = t * 0.025;
    let mag = (0.6 + 0.4 * (t * 0.011).sin()) * 0.22;
    (angle.cos() * mag, angle.sin() * mag)
}

// ===========================================================================
// Wedge
// ===========================================================================

/// A V-shaped wedge cut taken out of the disc.
///
/// From `inner_np` outward the cut widens linearly: at `inner_np`
/// its half-width is 0 (a point), at the rim it reaches `hw_at_rim`
/// on each side of `centre`. A small deterministic noise function
/// nudges the half-width so the cut edge isn't a straight line.
#[derive(Clone, Copy, Debug)]
pub struct Wedge {
    /// Centre angle in the pad's local frame (radians).
    pub centre: f64,
    /// Half the wedge's angular width at the rim (radians).
    pub hw_at_rim: f64,
    /// Normalised radius at which the wedge tip sits.
    pub inner_np: f64,
}

impl Wedge {
    /// True if a pixel at `(local_angle, np)` is inside the wedge.
    fn contains(&self, local_angle: f64, np: f64) -> bool {
        if np <= self.inner_np {
            return false;
        }
        let progress = (np - self.inner_np) / (1.0 - self.inner_np);
        let phase = np * 6.0 + self.centre * 3.1;
        let jitter = phase.sin() * (WEDGE_JITTER_AMP * 0.6)
            + (phase * 2.3 + 0.7).cos() * (WEDGE_JITTER_AMP * 0.4);
        let effective_hw = (self.hw_at_rim + jitter) * progress;
        angle_dist(local_angle, self.centre) < effective_hw
    }
}

// ===========================================================================
// LilyPad
// ===========================================================================

pub struct LilyPad {
    x: f64,
    y: f64,
    radius: f64,
    home_x: f64,
    home_y: f64,
    vx: f64,
    vy: f64,
    /// Per-pad phase seed used by the ambient drift sinusoid (and,
    /// historically, by the rim wobble). Keeps pads out of lockstep.
    phase: f64,
    rotation: f64,
    rotation_rate: f64,
    wedge: Option<Wedge>,
    /// Set externally each frame: true if a frog is currently
    /// sitting on this pad. Occupied pads render in a darker,
    /// water-tinted palette so the frog stands out against them.
    occupied: bool,
}

impl LilyPad {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f64,
        y: f64,
        radius: f64,
        phase: f64,
        rotation: f64,
        rotation_rate: f64,
        wedge: Option<Wedge>,
    ) -> Self {
        LilyPad {
            x,
            y,
            radius,
            home_x: x,
            home_y: y,
            vx: 0.0,
            vy: 0.0,
            phase,
            rotation,
            rotation_rate,
            wedge,
            occupied: false,
        }
    }

    #[cfg(test)]
    pub fn velocity(&self) -> (f64, f64) {
        (self.vx, self.vy)
    }

    /// Mark whether a frog is currently resting on this pad. Pond
    /// recomputes this every frame from current frog positions.
    pub fn set_occupied(&mut self, occupied: bool) {
        self.occupied = occupied;
    }

    #[cfg(test)]
    pub fn is_occupied(&self) -> bool {
        self.occupied
    }

    /// Snapshot of `(x, y, radius)` for other actors that need to
    /// reason about pad positions without holding a borrow of the
    /// pond.
    pub fn snapshot(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.radius)
    }

    // -- physics --------------------------------------------------------

    pub fn tick(&mut self, dt: f64, t: f64, koi_data: &[(f64, f64, f64, f64)]) {
        let (ax, ay) = self.acceleration(t, koi_data);
        self.vx += ax * dt;
        self.vy += ay * dt;
        let damp = (-DAMPING * dt).exp();
        self.vx *= damp;
        self.vy *= damp;
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.rotation += self.rotation_rate * dt;
    }

    fn acceleration(&self, t: f64, koi_data: &[(f64, f64, f64, f64)]) -> (f64, f64) {
        let spring_x = -SPRING_K * (self.x - self.home_x);
        let spring_y = -SPRING_K * (self.y - self.home_y);
        let cur_x = (t * 0.07 + self.phase).sin() * AMBIENT_AMP;
        let cur_y = (t * 0.05 + self.phase * 1.3).cos() * AMBIENT_AMP * 0.8;
        let (gw_x, gw_y) = global_wind(t);
        let (wake_x, wake_y) = self.koi_wake(koi_data);
        (
            spring_x + cur_x + gw_x + wake_x,
            spring_y + cur_y + gw_y + wake_y,
        )
    }

    fn koi_wake(&self, koi_data: &[(f64, f64, f64, f64)]) -> (f64, f64) {
        let mut wx = 0.0;
        let mut wy = 0.0;
        for &(kx, ky, kvx, kvy) in koi_data {
            let dx = self.x - kx;
            let dy = self.y - ky;
            let dist = (dx * dx + dy * dy).sqrt();
            if !(0.5..WAKE_RADIUS).contains(&dist) {
                continue;
            }
            let strength = (1.0 - dist / WAKE_RADIUS).powi(2) * WAKE_GAIN;
            wx += kvx * strength;
            wy += kvy * strength;
        }
        (wx, wy)
    }

    // -- rendering ------------------------------------------------------

    pub fn draw(&self, canvas: &mut Canvas, scale: f64, _t: f64) {
        let cx_px = self.x * scale;
        let cy_px = self.y * scale;
        let r_px = self.radius * scale;
        let r_int = r_px.ceil() as i32;
        let cx_i = cx_px as i32;
        let cy_i = cy_px as i32;

        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let dxf = dx as f64;
                let dyf = dy as f64;
                let d = (dxf * dxf + dyf * dyf).sqrt();
                if d > r_px {
                    continue;
                }
                let local_angle = dyf.atan2(dxf) - self.rotation;
                let np = d / r_px;
                if self.pixel_in_wedge(local_angle, np) {
                    continue;
                }
                let (r, g, b) = pixel_colour(local_angle, np, d, self.occupied);
                canvas.dot(cx_i + dx, cy_i + dy, r, g, b);
            }
        }
    }

    fn pixel_in_wedge(&self, local_angle: f64, np: f64) -> bool {
        self.wedge
            .as_ref()
            .is_some_and(|w| w.contains(local_angle, np))
    }
}

/// Mix toward this when a pad has a frog on it — pushes the pad
/// visibly into the water palette so the frog reads clearly on top.
const OCCUPIED_TINT: (u8, u8, u8) = (22, 40, 55);
const OCCUPIED_MIX: f64 = 0.70;

fn pixel_colour(local_angle: f64, np: f64, d: f64, occupied: bool) -> (u8, u8, u8) {
    let base = base_pixel_colour(local_angle, np, d);
    if occupied {
        lerp_color(base, OCCUPIED_TINT, OCCUPIED_MIX)
    } else {
        base
    }
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f64) + ((y as f64) - (x as f64)) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

fn base_pixel_colour(local_angle: f64, np: f64, d: f64) -> (u8, u8, u8) {
    if d < 0.5 {
        return color::HUB;
    }
    let vein_step = local_angle * VEIN_COUNT / TAU;
    let vein_d = (vein_step - vein_step.round()).abs();
    let near_vein = vein_d < VEIN_HALF_WIDTH && (0.14..0.90).contains(&np);

    if np > 0.92 {
        color::EDGE
    } else if near_vein {
        color::VEIN
    } else if np > 0.72 {
        color::MID
    } else if np < 0.14 {
        color::HUB
    } else {
        color::FILL
    }
}

// ===========================================================================
// Spawning
// ===========================================================================

/// Deterministic initial layout of pads inside a pond of size `w × h`.
/// The same dimensions always yield the same arrangement.
pub fn spawn_pads(w: f64, h: f64) -> Vec<LilyPad> {
    const N: usize = 10;
    (0..N).map(|i| random_pad(i, w, h)).collect()
}

fn random_pad(i: usize, w: f64, h: f64) -> LilyPad {
    use crate::rng::pseudo_rand;
    let seed = i as f64 * 13.7 + 4.2;
    let lerp = |t: f64, lo: f64, hi: f64| lo + t * (hi - lo);

    let x = lerp(pseudo_rand(seed), 0.1, 0.9) * w;
    let y = lerp(pseudo_rand(seed + 1.0), 0.1, 0.9) * h;
    let radius = lerp(pseudo_rand(seed + 2.0), RADIUS_MIN, RADIUS_MAX);
    let phase = pseudo_rand(seed + 3.0) * TAU;
    let rotation = pseudo_rand(seed + 4.0) * TAU;
    let rotation_rate = {
        let mag = lerp(
            pseudo_rand(seed + 5.0),
            ROTATION_RATE_MIN,
            ROTATION_RATE_MAX,
        );
        let sign = if pseudo_rand(seed + 9.0) > 0.5 {
            1.0
        } else {
            -1.0
        };
        mag * sign
    };
    let wedge = Wedge {
        centre: pseudo_rand(seed + 11.0) * TAU,
        hw_at_rim: lerp(pseudo_rand(seed + 12.0), WEDGE_HW_MIN, WEDGE_HW_MAX),
        inner_np: lerp(
            pseudo_rand(seed + 13.0),
            WEDGE_INNER_NP_MIN,
            WEDGE_INNER_NP_MAX,
        ),
    };
    LilyPad::new(x, y, radius, phase, rotation, rotation_rate, Some(wedge))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_wedge() -> Wedge {
        Wedge {
            centre: 0.0,
            hw_at_rim: 0.4,
            inner_np: 0.3,
        }
    }

    fn pad_at(x: f64, y: f64, radius: f64, wedge: Option<Wedge>) -> LilyPad {
        LilyPad::new(x, y, radius, 0.0, 0.0, 0.0, wedge)
    }

    fn make_pad() -> LilyPad {
        pad_at(20.0, 15.0, 5.0, Some(default_wedge()))
    }

    /// Empirical painted-pixel ratio against the full disc. Returns
    /// (painted_pixels / disc_pixels) after rendering the pad.
    fn painted_ratio(pad: &LilyPad, scale: f64) -> f64 {
        let r_sp = pad.radius * scale;
        let r_int = r_sp.ceil() as i32;
        let cx_sp = (pad.x * scale) as i32;
        let cy_sp = (pad.y * scale) as i32;
        let max_x_sp = cx_sp + r_int + 4;
        let max_y_sp = cy_sp + r_int + 4;
        let cw = (max_x_sp.max(0) as usize) / 2 + 2;
        let ch = (max_y_sp.max(0) as usize) / 4 + 2;
        let mut canvas = Canvas::new(cw, ch);
        pad.draw(&mut canvas, scale, 0.0);

        let mut painted = 0usize;
        let mut disc = 0usize;
        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                if d > r_sp {
                    continue;
                }
                disc += 1;
                let px = cx_sp + dx;
                let py = cy_sp + dy;
                if px < 0 || py < 0 {
                    continue;
                }
                if canvas.get(px as usize, py as usize).0 {
                    painted += 1;
                }
            }
        }
        painted as f64 / disc as f64
    }

    // -- construction & state ---------------------------------------------

    #[test]
    fn lily_pad_new_holds_position() {
        let p = make_pad();
        assert!((p.x - 20.0).abs() < 1e-10);
        assert!((p.y - 15.0).abs() < 1e-10);
        assert_eq!(p.velocity(), (0.0, 0.0));
    }

    // -- rendering --------------------------------------------------------

    #[test]
    fn draw_produces_visible_pixels() {
        let p = make_pad();
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(lit > 80, "pad should light many pixels, got {lit}");
    }

    #[test]
    fn draw_renders_veins_with_distinct_color() {
        let p = make_pad();
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == color::VEIN
            });
        assert!(found, "veins should be visible");
    }

    #[test]
    fn notch_creates_a_gap_on_the_rim() {
        // Pad with a V-wedge cut pointing east, tip at np=0.30. The
        // rim pixel due east should be inside the wedge and therefore
        // unpainted.
        let p = pad_at(40.0, 30.0, 6.0, Some(default_wedge()));
        let mut canvas = Canvas::new(160, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let cx = 80usize;
        let cy = 60usize;
        let probe_x = cx + 11; // just inside the east rim
        let (on, _, _, _) = canvas.get(probe_x, cy);
        assert!(
            !on,
            "the wedge should leave the rim pixel directly east unpainted"
        );
    }

    // -- physics ----------------------------------------------------------

    #[test]
    fn tick_returns_pad_toward_home_after_displacement() {
        let mut p = make_pad();
        p.x = 35.0;
        p.y = 30.0;
        let initial_dist = ((35.0_f64 - 20.0).powi(2) + (30.0_f64 - 15.0).powi(2)).sqrt();
        for i in 0..1200 {
            let t = i as f64 * 0.05;
            p.tick(0.05, t, &[]);
        }
        let final_dist = ((p.x - 20.0_f64).powi(2) + (p.y - 15.0_f64).powi(2)).sqrt();
        assert!(
            final_dist < initial_dist,
            "spring should still pull pad toward home: {initial_dist:.2} -> {final_dist:.2}",
        );
    }

    #[test]
    fn koi_wake_pushes_pad_in_swimming_direction() {
        let mut p = make_pad();
        let initial_x = p.x;
        let koi = [(18.0_f64, 15.0_f64, 10.0_f64, 0.0_f64)];
        for i in 0..40 {
            p.tick(0.05, i as f64 * 0.05, &koi);
        }
        assert!(
            p.x > initial_x,
            "wake should push pad east: {initial_x:.2} -> {:.2}",
            p.x
        );
    }

    #[test]
    fn ambient_current_produces_visible_drift() {
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.7, 0.0, 0.0, Some(default_wedge()));
        let mut max_excursion: f64 = 0.0;
        for i in 0..1000 {
            let t = i as f64 * 0.05;
            p.tick(0.05, t, &[]);
            let d = ((p.x - 20.0_f64).powi(2) + (p.y - 15.0_f64).powi(2)).sqrt();
            max_excursion = max_excursion.max(d);
        }
        assert!(
            max_excursion > 0.4,
            "ambient current + wind should drift pad at least 0.4 world, got {max_excursion}",
        );
    }

    // -- spawn_pads -------------------------------------------------------

    #[test]
    fn spawn_pads_is_deterministic() {
        let a = spawn_pads(80.0, 46.0);
        let b = spawn_pads(80.0, 46.0);
        assert_eq!(a.len(), b.len());
        for (p1, p2) in a.iter().zip(b.iter()) {
            assert!((p1.x - p2.x).abs() < 1e-10);
            assert!((p1.y - p2.y).abs() < 1e-10);
        }
    }

    #[test]
    fn spawn_pads_stays_inside_pond() {
        let (w, h) = (80.0, 46.0);
        for p in spawn_pads(w, h) {
            assert!(p.x > 0.0 && p.x < w);
            assert!(p.y > 0.0 && p.y < h);
        }
    }

    #[test]
    fn spawn_pads_vary_in_radius() {
        // The user explicitly asked for visibly varied sizes.
        let pads = spawn_pads(80.0, 46.0);
        let radii: Vec<f64> = pads.iter().map(|p| p.radius).collect();
        let lo = radii.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = radii.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            hi - lo > 1.5,
            "spawned pads should span a noticeable range of sizes: {lo:.2}..{hi:.2}",
        );
        assert!(lo >= RADIUS_MIN);
        assert!(hi <= RADIUS_MAX);
    }

    // -- wedge geometry invariants ----------------------------------------

    #[test]
    fn wedge_never_cuts_inside_its_inner_np() {
        let w = Wedge {
            centre: 0.0,
            hw_at_rim: 0.5,
            inner_np: 0.40,
        };
        for i in 0..72 {
            let angle = i as f64 / 72.0 * TAU - PI;
            for &np in &[0.0_f64, 0.10, 0.20, 0.30, 0.39, 0.40] {
                assert!(
                    !w.contains(angle, np),
                    "wedge cut at np={np}, angle={angle} (inner_np=0.40 should be safe)",
                );
            }
        }
    }

    #[test]
    fn wedge_tapers_to_a_point_at_inner_np() {
        let w = Wedge {
            centre: 0.0,
            hw_at_rim: 0.5,
            inner_np: 0.40,
        };
        assert!(
            !w.contains(0.30, 0.42),
            "wedge should taper — off-axis pixel near tip must stay painted",
        );
        assert!(
            w.contains(0.30, 0.99),
            "wedge should be at near-full width near the rim",
        );
    }

    #[test]
    fn wedge_only_cuts_around_its_centre_angle() {
        let hw = 0.4;
        let w = Wedge {
            centre: 0.0,
            hw_at_rim: hw,
            inner_np: 0.30,
        };
        for &probe in &[PI, PI / 2.0, -PI / 2.0, PI - 0.1, -PI + 0.1] {
            assert!(
                !w.contains(probe, 0.99),
                "angle {probe} should be outside the wedge (centre=0, hw={hw})",
            );
        }
    }

    #[test]
    fn angle_dist_handles_rotation_above_pi() {
        // The bug that made wedges eat half the pond: when local_angle
        // can be `world_angle - rotation` and rotation > PI, the
        // unnormalised difference falls outside [-PI, PI] and the old
        // .abs() trick broke. The current implementation must return
        // a non-negative value ≤ π for any inputs.
        for &a in &[-12.0_f64, -3.5, 0.0, 1.7, 8.4] {
            for &b in &[-4.0_f64, 0.0, 2.87, 6.0] {
                let d = angle_dist(a, b);
                assert!(
                    (0.0..=PI + 1e-12).contains(&d),
                    "angle_dist({a}, {b}) = {d}"
                );
            }
        }
    }

    // -- 40 % painted-area floor -----------------------------------------

    #[test]
    fn pad_with_no_wedge_paints_almost_the_whole_disc() {
        let p = pad_at(32.9, 10.9, 6.5, None);
        let ratio = painted_ratio(&p, 2.0);
        assert!(
            ratio > 0.99,
            "pad with no wedge should be fully painted: got {ratio:.3}",
        );
    }

    #[test]
    fn every_spawned_pad_keeps_at_least_40_percent_painted() {
        for (i, pad) in spawn_pads(80.0, 46.0).iter().enumerate() {
            let ratio = painted_ratio(pad, 2.0);
            assert!(
                ratio >= 0.40,
                "pad #{i} at ({:.1},{:.1}): only {:.1}% painted (must stay ≥ 40%)",
                pad.x,
                pad.y,
                ratio * 100.0,
            );
        }
    }

    #[test]
    fn worst_case_wedge_still_leaves_disc_majority_painted() {
        let w = Wedge {
            centre: 0.0,
            hw_at_rim: WEDGE_HW_MAX,
            inner_np: WEDGE_INNER_NP_MIN,
        };
        let p = pad_at(20.0, 15.0, 6.5, Some(w));
        let ratio = painted_ratio(&p, 2.0);
        assert!(
            ratio >= 0.60,
            "worst-case wedge cut too much: only {:.1}% painted",
            ratio * 100.0,
        );
    }

    #[test]
    fn many_random_pads_keep_at_least_40_percent_painted() {
        use crate::rng::pseudo_rand;
        let lerp = |t: f64, lo: f64, hi: f64| lo + t * (hi - lo);
        for i in 0..100u32 {
            let s = i as f64 * 7.13 + 0.5;
            let w = Wedge {
                centre: pseudo_rand(s + 2.0) * TAU,
                hw_at_rim: lerp(pseudo_rand(s), WEDGE_HW_MIN, WEDGE_HW_MAX),
                inner_np: lerp(pseudo_rand(s + 1.0), WEDGE_INNER_NP_MIN, WEDGE_INNER_NP_MAX),
            };
            let p = pad_at(20.0, 15.0, 6.5, Some(w));
            let ratio = painted_ratio(&p, 2.0);
            assert!(
                ratio >= 0.40,
                "iter {i} (hw={:.3}, inner={:.3}): only {:.1}% painted",
                w.hw_at_rim,
                w.inner_np,
                ratio * 100.0,
            );
        }
    }
}
