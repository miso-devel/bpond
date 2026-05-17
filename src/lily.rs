//! Lotus pads (ハスの葉) floating on the pond surface.
//!
//! Lotus leaves from above aren't perfect discs — they have a wavy rim,
//! visible radial veins, and a darker hub where the petiole attaches. We
//! reproduce all three with cheap per-pixel checks: an angle-dependent
//! radius for the rim, a sawtooth-style distance test for veins, and a
//! small dark center spot for the hub.
//!
//! Pads drift on the water:
//! - They spring back toward an anchor point ("home") so they don't wander
//!   off to the boundary.
//! - A slow ambient current oscillates the resting position so even an
//!   undisturbed pond looks alive.
//! - When a koi passes within `WAKE_RADIUS` the pad picks up a fraction
//!   of the koi's velocity, simulating the wake under the floating leaf.
//! - Linear damping bleeds energy off so things settle.

use crate::canvas::Canvas;
use std::f64::consts::TAU;

// ---------------------------------------------------------------------------
// Color bands
// ---------------------------------------------------------------------------

/// Bright fill through the middle of the leaf.
const FILL: (u8, u8, u8) = (60, 110, 45);
/// Mid-zone (toward the rim).
const MID: (u8, u8, u8) = (45, 85, 35);
/// Dark outline at the rim.
const EDGE: (u8, u8, u8) = (22, 50, 25);
/// Darker hub at the very center suggesting the petiole.
const HUB: (u8, u8, u8) = (30, 60, 28);
/// Veins radiating from the hub.
const VEIN: (u8, u8, u8) = (32, 62, 30);

// ---------------------------------------------------------------------------
// Shape parameters
// ---------------------------------------------------------------------------

/// Number of radial veins.
const VEIN_COUNT: f64 = 8.0;
/// How wide each vein appears in step-units (smaller = thinner).
const VEIN_HALF_WIDTH: f64 = 0.025;
/// Number of bumps around the rim (gentle waviness).
const RIM_BUMPS: f64 = 7.0;
/// Amplitude of the rim bumps as a fraction of the resting radius.
const RIM_BUMP_AMP: f64 = 0.05;
/// Amplitude of the slow whole-leaf breathing.
const BREATH_AMP: f64 = 0.025;

// ---------------------------------------------------------------------------
// Drift physics parameters
// ---------------------------------------------------------------------------

/// Spring constant pulling the pad back toward its home anchor.
const SPRING_K: f64 = 0.6;
/// Linear damping coefficient (1/s) — velocity decays as exp(-DAMPING·dt).
const DAMPING: f64 = 0.5;
/// Slow ambient water current amplitude (world units / s²).
const AMBIENT_AMP: f64 = 0.10;
/// Maximum distance at which a koi pushes the pad.
const WAKE_RADIUS: f64 = 6.5;
/// How strongly the koi's velocity feeds into the pad.
const WAKE_GAIN: f64 = 0.18;

// ---------------------------------------------------------------------------
// LilyPad
// ---------------------------------------------------------------------------

pub struct LilyPad {
    pub x: f64,
    pub y: f64,
    radius: f64,
    /// Anchor point — drift always tries to return here.
    home_x: f64,
    home_y: f64,
    /// Drift velocity.
    vx: f64,
    vy: f64,
    /// Phase offset for the rim and breathing oscillations.
    rim_phase: f64,
    /// Current rotation of the vein / rim pattern (radians).
    rotation: f64,
    /// Constant slow rotation rate (radians / s).
    rotation_rate: f64,
}

impl LilyPad {
    pub fn new(
        x: f64,
        y: f64,
        radius: f64,
        rim_phase: f64,
        rotation: f64,
        rotation_rate: f64,
    ) -> Self {
        LilyPad {
            x,
            y,
            radius,
            home_x: x,
            home_y: y,
            vx: 0.0,
            vy: 0.0,
            rim_phase,
            rotation,
            rotation_rate,
        }
    }

    #[cfg(test)]
    pub fn velocity(&self) -> (f64, f64) {
        (self.vx, self.vy)
    }

    /// Advance one frame of drift physics. `koi_data` is `(x, y, vx, vy)`
    /// per koi — vx/vy in world units / s.
    pub fn tick(&mut self, dt: f64, t: f64, koi_data: &[(f64, f64, f64, f64)]) {
        // Spring toward anchor.
        let spring_x = -SPRING_K * (self.x - self.home_x);
        let spring_y = -SPRING_K * (self.y - self.home_y);

        // Slow ambient current — different per pad via rim_phase.
        let cur_x = (t * 0.07 + self.rim_phase).sin() * AMBIENT_AMP;
        let cur_y = (t * 0.05 + self.rim_phase * 1.3).cos() * AMBIENT_AMP * 0.8;

        // Wake from any koi that's near. Falls off quadratically with
        // distance so a koi right under the pad pushes hardest.
        let mut wake_x = 0.0;
        let mut wake_y = 0.0;
        for &(kx, ky, kvx, kvy) in koi_data {
            let dx = self.x - kx;
            let dy = self.y - ky;
            let dist = (dx * dx + dy * dy).sqrt();
            if !(0.5..WAKE_RADIUS).contains(&dist) {
                continue;
            }
            let strength = (1.0 - dist / WAKE_RADIUS).powi(2) * WAKE_GAIN;
            wake_x += kvx * strength;
            wake_y += kvy * strength;
        }

        // Euler integrate forces into velocity, then damp.
        self.vx += (spring_x + cur_x + wake_x) * dt;
        self.vy += (spring_y + cur_y + wake_y) * dt;
        let damp = (-DAMPING * dt).exp();
        self.vx *= damp;
        self.vy *= damp;

        // Integrate position and rotation.
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.rotation += self.rotation_rate * dt;
    }

    /// Effective radius at a given angle — combines a slow breath with
    /// `RIM_BUMPS` periodic bumps so the silhouette isn't a flat circle.
    fn radius_at(&self, t: f64, angle: f64) -> f64 {
        let bumps = ((angle - self.rotation) * RIM_BUMPS + self.rim_phase).sin() * RIM_BUMP_AMP;
        let breath = (t * 0.25 + self.rim_phase).sin() * BREATH_AMP;
        self.radius * (1.0 + bumps + breath)
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64, t: f64) {
        let cx_px = self.x * scale;
        let cy_px = self.y * scale;
        // Worst-case radius for the bounding box scan (accounts for both
        // breath and bump amplitudes pushing the rim outward).
        let max_r_px = self.radius * (1.0 + RIM_BUMP_AMP + BREATH_AMP) * scale;
        let r_int = max_r_px.ceil() as i32;

        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let dxf = dx as f64;
                let dyf = dy as f64;
                let d = (dxf * dxf + dyf * dyf).sqrt();

                // Center pixel — give the very heart of the leaf the hub
                // colour straight away so the petiole spot reads cleanly.
                if d < 0.5 {
                    canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, HUB.0, HUB.1, HUB.2);
                    continue;
                }

                let angle = dyf.atan2(dxf);
                let r_local = self.radius_at(t, angle) * scale;
                if d > r_local {
                    continue;
                }

                // Normalised radial position: 0 at center, 1 at rim.
                let np = d / r_local;

                // Vein detection: 8 radial darker lines, with thin width
                // and excluded near the rim and the hub.
                let local_angle = angle - self.rotation;
                let vein_step = local_angle * VEIN_COUNT / TAU;
                let vein_d = (vein_step - vein_step.round()).abs();
                let near_vein = vein_d < VEIN_HALF_WIDTH && (0.12..0.92).contains(&np);

                let (r, g, b) = if np > 0.92 {
                    EDGE
                } else if near_vein {
                    VEIN
                } else if np > 0.70 {
                    MID
                } else if np < 0.12 {
                    HUB
                } else {
                    FILL
                };

                canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, r, g, b);
            }
        }
    }
}

/// Deterministic initial lily layout — same seed → same arrangement.
pub fn spawn_pads(w: f64, h: f64) -> Vec<LilyPad> {
    use crate::rng::pseudo_rand;
    const N: usize = 6;
    let mut pads = Vec::with_capacity(N);
    for i in 0..N {
        let seed = i as f64 * 13.7 + 4.2;
        let x = (0.1 + pseudo_rand(seed) * 0.8) * w;
        let y = (0.1 + pseudo_rand(seed + 1.0) * 0.8) * h;
        let radius = 3.0 + pseudo_rand(seed + 2.0) * 3.0; // 3–6 world units
        let rim_phase = pseudo_rand(seed + 3.0) * TAU;
        let rotation = pseudo_rand(seed + 4.0) * TAU;
        // Rotation rate: ±0.05 rad/s — slow turn over ~2 minutes.
        let rotation_rate = (pseudo_rand(seed + 5.0) - 0.5) * 0.1;
        pads.push(LilyPad::new(
            x,
            y,
            radius,
            rim_phase,
            rotation,
            rotation_rate,
        ));
    }
    pads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pad() -> LilyPad {
        LilyPad::new(20.0, 15.0, 4.0, 0.0, 0.0, 0.0)
    }

    #[test]
    fn lily_pad_new_holds_position() {
        let p = LilyPad::new(10.0, 20.0, 3.5, 0.0, 0.0, 0.0);
        assert!((p.x - 10.0).abs() < 1e-10);
        assert!((p.y - 20.0).abs() < 1e-10);
        assert_eq!(p.velocity(), (0.0, 0.0));
    }

    #[test]
    fn radius_stays_within_envelope() {
        // Combined breath + bumps shouldn't push the radius beyond ±10%
        // of the resting value at any (t, angle).
        let p = LilyPad::new(0.0, 0.0, 4.0, 1.3, 0.5, 0.0);
        for i in 0..200 {
            let t = i as f64 * 0.1;
            for j in 0..36 {
                let angle = j as f64 / 36.0 * TAU;
                let r = p.radius_at(t, angle);
                let bound = 4.0 * (RIM_BUMP_AMP + BREATH_AMP + 1e-6);
                assert!(
                    (r - 4.0).abs() <= bound,
                    "radius {r} outside envelope at t={t}, angle={angle}"
                );
            }
        }
    }

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
        // Look for at least one pixel that ended up in the VEIN bucket —
        // a value distinct from FILL / MID / HUB / EDGE.
        let p = LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0);
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let mut found = false;
        for x in 0..canvas.w {
            for y in 0..canvas.h {
                let (on, r, g, b) = canvas.get(x, y);
                if on && (r, g, b) == (VEIN.0, VEIN.1, VEIN.2) {
                    found = true;
                }
            }
        }
        assert!(found, "veins should be visible on a large pad");
    }

    // -- drift physics ------------------------------------------------------

    #[test]
    fn tick_returns_pad_toward_home_after_displacement() {
        let mut p = LilyPad::new(20.0, 15.0, 3.0, 0.0, 0.0, 0.0);
        // Manually shift away from home, then let it settle.
        p.x = 30.0;
        p.y = 25.0;
        let initial_dist = ((30.0_f64 - 20.0).powi(2) + (25.0_f64 - 15.0).powi(2)).sqrt();
        for i in 0..600 {
            let t = i as f64 * 0.05;
            p.tick(0.05, t, &[]);
        }
        let final_dist = ((p.x - 20.0_f64).powi(2) + (p.y - 15.0_f64).powi(2)).sqrt();
        assert!(
            final_dist < initial_dist * 0.5,
            "spring should pull pad most of the way home: {initial_dist:.2} -> {final_dist:.2}",
        );
    }

    #[test]
    fn koi_wake_pushes_pad_in_swimming_direction() {
        // Koi swimming east beside the pad should nudge it eastward.
        let mut p = LilyPad::new(20.0, 15.0, 3.0, 0.0, 0.0, 0.0);
        let initial_x = p.x;
        // Koi: position (18, 15), velocity (10, 0) → east, fast.
        let koi_data = [(18.0_f64, 15.0_f64, 10.0_f64, 0.0_f64)];
        for i in 0..40 {
            p.tick(0.05, i as f64 * 0.05, &koi_data);
        }
        assert!(
            p.x > initial_x,
            "wake should push pad east: {initial_x:.2} -> {:.2}",
            p.x
        );
    }

    #[test]
    fn ambient_current_nudges_resting_pad() {
        // No koi, no displacement — pure ambient field. Position should
        // wobble by some non-zero amount over time.
        let mut p = LilyPad::new(20.0, 15.0, 3.0, 0.7, 0.0, 0.0);
        let mut max_excursion: f64 = 0.0;
        for i in 0..500 {
            let t = i as f64 * 0.05;
            p.tick(0.05, t, &[]);
            let d = ((p.x - 20.0_f64).powi(2) + (p.y - 15.0_f64).powi(2)).sqrt();
            max_excursion = max_excursion.max(d);
        }
        assert!(
            max_excursion > 0.05,
            "ambient current should produce some drift, got max {max_excursion}",
        );
    }

    #[test]
    fn rotation_advances_at_configured_rate() {
        let mut p = LilyPad::new(20.0, 15.0, 3.0, 0.0, 0.0, 0.05);
        let initial = p.rotation;
        for _ in 0..200 {
            p.tick(0.05, 0.0, &[]);
        }
        // 200 * 0.05 * 0.05 = 0.5 rad over 10s.
        assert!(
            (p.rotation - initial - 0.5).abs() < 0.05,
            "rotation should advance ≈0.5 rad over 10 s, got {}",
            p.rotation - initial,
        );
    }

    // -- spawn --------------------------------------------------------------

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
            assert!(p.x > 0.0 && p.x < w, "pad x={} outside pond", p.x);
            assert!(p.y > 0.0 && p.y < h, "pad y={} outside pond", p.y);
        }
    }
}
