//! Lotus pads floating on the pond surface.
//!
//! Each pad is a clean disc with one or two pie-slice wedges removed,
//! like a cake with a slice taken out. Other features:
//!
//! 1. **Pie-slice cuts** from centre to rim — either one larger slice
//!    or two smaller ones, never removing more than ~40% of the disc.
//! 2. **Radial veins** spreading out from the hub — typically 12-18
//!    visible primary veins.
//! 3. A **sunlit crescent** on the rim suggesting the leaf's gentle
//!    saucer / cup shape and the angle of the light.
//! 4. **Drift** — pads on a pond aren't static. Wind, currents, and
//!    fish brushing past keep them in continuous motion.

use crate::canvas::Canvas;
use std::f64::consts::{PI, TAU};

// ---------------------------------------------------------------------------
// Colour palette
// ---------------------------------------------------------------------------

const FILL: (u8, u8, u8) = (60, 110, 45);
const MID: (u8, u8, u8) = (45, 85, 35);
const EDGE: (u8, u8, u8) = (20, 48, 24);
const HUB: (u8, u8, u8) = (28, 55, 30);
const VEIN: (u8, u8, u8) = (30, 60, 28);
/// Brighter green along the sun-lit rim crescent.
const HIGHLIGHT: (u8, u8, u8) = (110, 165, 80);

// ---------------------------------------------------------------------------
// Shape parameters
// ---------------------------------------------------------------------------

/// Radial vein count — real lotus leaves show 12-18 primary veins.
const VEIN_COUNT: f64 = 14.0;
const VEIN_HALF_WIDTH: f64 = 0.03;

/// Rim bumps and breath both disabled — the leaf must never extend
/// past its base circle. Variation between pads comes from the notch
/// alone; the silhouette is otherwise a strict, stable circle.
const RIM_BUMPS: f64 = 7.0;
const RIM_BUMP_AMP: f64 = 0.0;
const BREATH_AMP: f64 = 0.0;

/// Pie-slice notch geometry. A "notch" is an angular wedge cut from
/// centre to rim — the missing slice of cake. Per-pad variation is
/// limited to: 1 slice or 2 slices, big or small. Sizes are tuned to
/// read as a single missing wedge, not a half-eaten disc.
const SINGLE_SLICE_HW_MIN: f64 = 0.18; // ~20° total
const SINGLE_SLICE_HW_RANGE: f64 = 0.18; // up to ~40° total
const TWIN_SLICE_HW_MIN: f64 = 0.12; // ~14° total each
const TWIN_SLICE_HW_RANGE: f64 = 0.10; // up to ~25° total each

/// Sun-lit crescent on the rim.
const HIGHLIGHT_HALF_WIDTH: f64 = 0.6;
const HIGHLIGHT_INNER_NP: f64 = 0.75;

// ---------------------------------------------------------------------------
// Drift physics
// ---------------------------------------------------------------------------

const SPRING_K: f64 = 0.18;
const DAMPING: f64 = 0.28;
const AMBIENT_AMP: f64 = 0.32;
const WAKE_RADIUS: f64 = 9.0;
const WAKE_GAIN: f64 = 0.55;

/// Global wind drift — every pad feels this in addition to its own
/// ambient sinusoid. Direction slowly rotates so the pond's overall
/// flow shifts over a couple of minutes.
fn global_wind(t: f64) -> (f64, f64) {
    let wind_angle = t * 0.025;
    let wind_mag = (0.6 + 0.4 * (t * 0.011).sin()) * 0.22;
    (wind_angle.cos() * wind_mag, wind_angle.sin() * wind_mag)
}

// ---------------------------------------------------------------------------
// LilyPad
// ---------------------------------------------------------------------------

pub struct LilyPad {
    pub x: f64,
    pub y: f64,
    radius: f64,
    home_x: f64,
    home_y: f64,
    vx: f64,
    vy: f64,
    rim_phase: f64,
    rotation: f64,
    rotation_rate: f64,
    /// Pie-slice cuts in the pad-local frame: each entry is
    /// `(centre_angle, half_width)` in radians. 1 or 2 slices per pad.
    notches: Vec<(f64, f64)>,
    /// Angle of the sun-lit highlight crescent (pad-local frame).
    highlight_angle: f64,
}

impl LilyPad {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f64,
        y: f64,
        radius: f64,
        rim_phase: f64,
        rotation: f64,
        rotation_rate: f64,
        notches: Vec<(f64, f64)>,
        highlight_angle: f64,
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
            notches,
            highlight_angle,
        }
    }

    fn in_any_notch(&self, local_angle: f64) -> bool {
        self.notches
            .iter()
            .any(|&(c, hw)| Self::angle_dist(local_angle, c) < hw)
    }

    #[cfg(test)]
    pub fn velocity(&self) -> (f64, f64) {
        (self.vx, self.vy)
    }

    pub fn tick(&mut self, dt: f64, t: f64, koi_data: &[(f64, f64, f64, f64)]) {
        let spring_x = -SPRING_K * (self.x - self.home_x);
        let spring_y = -SPRING_K * (self.y - self.home_y);

        // Per-pad ambient sinusoid (different per pad through rim_phase).
        let cur_x = (t * 0.07 + self.rim_phase).sin() * AMBIENT_AMP;
        let cur_y = (t * 0.05 + self.rim_phase * 1.3).cos() * AMBIENT_AMP * 0.8;

        // Shared global wind on top of the per-pad sinusoid — gives the
        // whole pond a coherent direction at any moment.
        let (gw_x, gw_y) = global_wind(t);

        // Wake from any koi nearby.
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

        self.vx += (spring_x + cur_x + gw_x + wake_x) * dt;
        self.vy += (spring_y + cur_y + gw_y + wake_y) * dt;
        let damp = (-DAMPING * dt).exp();
        self.vx *= damp;
        self.vy *= damp;

        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.rotation += self.rotation_rate * dt;
    }

    fn radius_at(&self, t: f64, angle: f64) -> f64 {
        let bumps = ((angle - self.rotation) * RIM_BUMPS + self.rim_phase).sin() * RIM_BUMP_AMP;
        let breath = (t * 0.25 + self.rim_phase).sin() * BREATH_AMP;
        self.radius * (1.0 + bumps + breath)
    }

    /// Wraparound angular distance (shortest path).
    fn angle_dist(a: f64, b: f64) -> f64 {
        let mut d = (a - b).abs();
        if d > PI {
            d = TAU - d;
        }
        d
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64, t: f64) {
        let cx_px = self.x * scale;
        let cy_px = self.y * scale;
        let max_r_px = self.radius * (1.0 + RIM_BUMP_AMP + BREATH_AMP) * scale;
        let r_int = max_r_px.ceil() as i32;

        // 1) Paint the leaf body pixel by pixel.
        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let dxf = dx as f64;
                let dyf = dy as f64;
                let d = (dxf * dxf + dyf * dyf).sqrt();

                let angle = dyf.atan2(dxf);
                let r_local = self.radius_at(t, angle) * scale;
                if d > r_local {
                    continue;
                }
                let local_angle = angle - self.rotation;

                // Pie-slice cut: removes everything inside the angular
                // wedge from centre to rim, like a slice of cake taken
                // out of the disc.
                if self.in_any_notch(local_angle) {
                    continue;
                }

                // Centre hub: paint the petiole attachment cleanly.
                if d < 0.5 {
                    canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, HUB.0, HUB.1, HUB.2);
                    continue;
                }

                let np = d / r_local;

                // Vein detection: radial sawtooth distance test.
                let vein_step = local_angle * VEIN_COUNT / TAU;
                let vein_d = (vein_step - vein_step.round()).abs();
                let near_vein = vein_d < VEIN_HALF_WIDTH && (0.14..0.90).contains(&np);

                // Sun-lit crescent on the outer rim band.
                let near_highlight = (HIGHLIGHT_INNER_NP..=0.92).contains(&np)
                    && Self::angle_dist(local_angle, self.highlight_angle) < HIGHLIGHT_HALF_WIDTH;

                let (r, g, b) = if np > 0.92 {
                    EDGE
                } else if near_highlight {
                    HIGHLIGHT
                } else if near_vein {
                    VEIN
                } else if np > 0.72 {
                    MID
                } else if np < 0.14 {
                    HUB
                } else {
                    FILL
                };

                canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, r, g, b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

/// Deterministic initial lily layout — same pond dimensions yield the
/// same arrangement of pads, notches, highlights, and droplets.
pub fn spawn_pads(w: f64, h: f64) -> Vec<LilyPad> {
    use crate::rng::pseudo_rand;
    const N: usize = 10;
    let mut pads = Vec::with_capacity(N);
    for i in 0..N {
        let seed = i as f64 * 13.7 + 4.2;
        let x = (0.1 + pseudo_rand(seed) * 0.8) * w;
        let y = (0.1 + pseudo_rand(seed + 1.0) * 0.8) * h;
        // Uniform radius — the silhouette is the same circle for every
        // pad; only the notch differs.
        let radius = 6.5;
        let rim_phase = pseudo_rand(seed + 3.0) * TAU;
        let rotation = pseudo_rand(seed + 4.0) * TAU;
        let rate_mag = 0.10 + pseudo_rand(seed + 5.0) * 0.20;
        let rate_sign = if pseudo_rand(seed + 9.0) > 0.5 {
            1.0
        } else {
            -1.0
        };
        let rotation_rate = rate_mag * rate_sign;
        // Per-pad variation: either one bigger slice removed or two
        // smaller slices on opposite sides. Each slice is a pie wedge
        // from centre to rim, like a piece of cake taken out.
        let twin = pseudo_rand(seed + 10.0) > 0.5;
        let notches = if twin {
            let a1 = pseudo_rand(seed + 11.0) * TAU;
            let hw1 = TWIN_SLICE_HW_MIN + pseudo_rand(seed + 12.0) * TWIN_SLICE_HW_RANGE;
            let hw2 = TWIN_SLICE_HW_MIN + pseudo_rand(seed + 14.0) * TWIN_SLICE_HW_RANGE;
            // Put the second slice on the opposite side with jitter so
            // the two cuts never overlap.
            let sep = PI * (0.6 + pseudo_rand(seed + 15.0) * 0.4);
            vec![(a1, hw1), (a1 + sep, hw2)]
        } else {
            let a = pseudo_rand(seed + 11.0) * TAU;
            let hw = SINGLE_SLICE_HW_MIN + pseudo_rand(seed + 12.0) * SINGLE_SLICE_HW_RANGE;
            vec![(a, hw)]
        };
        let highlight_angle = pseudo_rand(seed + 7.0) * TAU;
        pads.push(LilyPad::new(
            x,
            y,
            radius,
            rim_phase,
            rotation,
            rotation_rate,
            notches,
            highlight_angle,
        ));
    }
    pads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pad() -> LilyPad {
        LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4)], PI)
    }

    #[test]
    fn lily_pad_new_holds_position() {
        let p = make_pad();
        assert!((p.x - 20.0).abs() < 1e-10);
        assert!((p.y - 15.0).abs() < 1e-10);
        assert_eq!(p.velocity(), (0.0, 0.0));
    }

    #[test]
    fn radius_stays_within_envelope() {
        let p = LilyPad::new(0.0, 0.0, 4.0, 1.3, 0.5, 0.0, vec![(0.0, 0.4)], PI);
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
        let p = make_pad();
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == (VEIN.0, VEIN.1, VEIN.2)
            });
        assert!(found, "veins should be visible");
    }

    #[test]
    fn draw_renders_highlight_band() {
        // Highlight is brighter than any other green band — should be
        // present on a pad with a non-zero radius.
        let p = make_pad();
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == (HIGHLIGHT.0, HIGHLIGHT.1, HIGHLIGHT.2)
            });
        assert!(found, "sun-lit crescent should be visible");
    }

    #[test]
    fn notch_creates_a_gap_on_the_rim() {
        // Pad with a pie-slice cut pointing east. The rim pixel due
        // east should be inside the slice and therefore unpainted.
        let p = LilyPad::new(40.0, 30.0, 6.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4)], PI);
        let mut canvas = Canvas::new(160, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        // Center of canvas approx (80, 60) (pad center px = 40*2=80, 30*2=60).
        let cx = 80usize;
        let cy = 60usize;
        // Point ~1 pixel inside the rim, directly east of center.
        // r_local ≈ 6 world × 2 scale = 12 sub-pixels.
        let probe_x = cx + 11; // just inside rim
        let probe_y = cy;
        let (on, _, _, _) = canvas.get(probe_x, probe_y);
        assert!(
            !on,
            "the notch should leave the rim pixel directly east unpainted"
        );
    }

    #[test]
    fn tick_returns_pad_toward_home_after_displacement() {
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4)], PI);
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
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4)], PI);
        let initial_x = p.x;
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
    fn ambient_current_produces_visible_drift() {
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.7, 0.0, 0.0, vec![(0.0, 0.4)], PI);
        let mut max_excursion: f64 = 0.0;
        for i in 0..1000 {
            let t = i as f64 * 0.05;
            p.tick(0.05, t, &[]);
            let d = ((p.x - 20.0_f64).powi(2) + (p.y - 15.0_f64).powi(2)).sqrt();
            max_excursion = max_excursion.max(d);
        }
        // With the new stronger ambient + global wind, drift should
        // reach at least ~0.4 world units (1+ cell) from home.
        assert!(
            max_excursion > 0.4,
            "ambient current + wind should drift pad at least 0.4 world, got {max_excursion}",
        );
    }

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
}
