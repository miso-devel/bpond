//! Lotus pads floating on the pond surface.
//!
//! Each pad is a clean disc with a single V-shaped wedge cut from
//! the rim, like a single notch out of a green coin. Other features:
//!
//! 1. **V-shaped wedge cut** — widest at the rim, tapering to a point
//!    at a random inner radius. Three per-pad parameters control the
//!    wedge: where it points, how wide it opens, and how deep it
//!    reaches. The wedge edge has a small deterministic wobble so it
//!    reads as a natural cut rather than a mathematical sector.
//! 2. **Radial veins** spreading out from the hub — typically 12-18
//!    visible primary veins.
//! 3. **Drift** — pads on a pond aren't static. Wind, currents, and
//!    fish brushing past keep them in continuous motion.

use crate::canvas::Canvas;
use std::f64::consts::{PI, TAU};

// ---------------------------------------------------------------------------
// Colour palette
// ---------------------------------------------------------------------------

/// Body greens, picked to stay clearly above the dark blue water
/// background so the leaf reads as a leaf, not as a few specks.
const FILL: (u8, u8, u8) = (95, 155, 70);
const MID: (u8, u8, u8) = (70, 125, 55);
const EDGE: (u8, u8, u8) = (45, 90, 40);
const HUB: (u8, u8, u8) = (55, 100, 50);
const VEIN: (u8, u8, u8) = (50, 95, 42);

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

/// V-wedge geometry. `WEDGE_HW_*` is the wedge half-width at the rim
/// (in radians); the wedge tapers linearly to a point at `inner_np`.
/// `WEDGE_INNER_NP_*` is the normalised radius at which the wedge tip
/// sits — some pads have a tip close to the centre (deep cut), some
/// further out (shallow notch).
///
/// With WEDGE_HW_MAX = 0.44 and INNER_NP_MIN = 0.30 the wedge
/// removes about 7.2% of the disc area (computed analytically). The
/// edge jitter adds at most ~5° to the rim half-width, pushing the
/// worst case to roughly 8% — well under the 60% upper bound implied
/// by the "leaf must stay at least 40% green" rule.
const WEDGE_HW_MIN: f64 = 0.22;
const WEDGE_HW_MAX: f64 = 0.42;
const WEDGE_INNER_NP_MIN: f64 = 0.30;
const WEDGE_INNER_NP_MAX: f64 = 0.65;

/// Peak deterministic edge jitter applied to the wedge half-width, in
/// radians. Bounded so the slice can never balloon by more than this.
const WEDGE_JITTER_AMP: f64 = 0.05;

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
    /// V-shaped wedges cut from the disc, in the pad-local frame.
    /// Each tuple is `(centre_angle, hw_at_rim, inner_np)`:
    ///   - `centre_angle` — angular direction the wedge points
    ///   - `hw_at_rim` — half the wedge's angular width at the rim
    ///   - `inner_np` — normalised radius at which the wedge tip sits
    ///
    /// Pixels with `np < inner_np` are never cut. From `inner_np` out
    /// to the rim the angular cut grows linearly from 0 to `hw_at_rim`.
    /// 1 wedge per pad in current designs, but the struct accepts a
    /// `Vec` to leave room for future variants without a breaking
    /// change.
    notches: Vec<(f64, f64, f64)>,
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
        notches: Vec<(f64, f64, f64)>,
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
        }
    }

    /// True if a pixel is inside any V-shaped wedge cut. `np` is the
    /// pixel's normalised radius (0 at centre, 1 at rim).
    ///
    /// The wedge is a true taper: at `np == inner_np` the angular
    /// half-width is 0 (a point), and it grows linearly to `hw_at_rim`
    /// as np climbs to 1. A bounded, position-dependent jitter is
    /// added to the half-width so the cut edge isn't a perfect line.
    /// The jitter is also scaled by the taper progress so the tip
    /// stays sharp.
    fn in_any_notch(&self, local_angle: f64, np: f64) -> bool {
        for &(center, hw_at_rim, inner_np) in &self.notches {
            if np <= inner_np {
                continue;
            }
            let progress = (np - inner_np) / (1.0 - inner_np);
            let phase = np * 6.0 + center * 3.1;
            let jitter_unit = phase.sin() * (WEDGE_JITTER_AMP * 0.6)
                + (phase * 2.3 + 0.7).cos() * (WEDGE_JITTER_AMP * 0.4);
            let effective_hw = (hw_at_rim + jitter_unit) * progress;
            if Self::angle_dist(local_angle, center) < effective_hw {
                return true;
            }
        }
        false
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

    /// Wraparound angular distance (shortest path), in `[0, π]`.
    ///
    /// `a` and `b` may be any finite angle — they don't need to be
    /// normalised first. We map `a - b` into `[0, TAU)` with
    /// `rem_euclid`, then fold values above π back through TAU.
    /// The previous version computed `(a - b).abs()` and could
    /// produce negative results for arguments outside `[-π, π]`,
    /// which silently broke the wedge cut for pads with large
    /// `rotation` values.
    fn angle_dist(a: f64, b: f64) -> f64 {
        let d = (a - b).rem_euclid(TAU);
        if d > PI {
            TAU - d
        } else {
            d
        }
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
                let np = d / r_local;

                // V-shaped wedge cut from the rim with a slightly
                // wobbled edge.
                if self.in_any_notch(local_angle, np) {
                    continue;
                }

                // Centre hub: paint the petiole attachment cleanly.
                if d < 0.5 {
                    canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, HUB.0, HUB.1, HUB.2);
                    continue;
                }

                // Vein detection: radial sawtooth distance test.
                let vein_step = local_angle * VEIN_COUNT / TAU;
                let vein_d = (vein_step - vein_step.round()).abs();
                let near_vein = vein_d < VEIN_HALF_WIDTH && (0.14..0.90).contains(&np);

                let (r, g, b) = if np > 0.92 {
                    EDGE
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
        // Vary radius per pad so the cluster doesn't read as a row of
        // identical coins. Small ones look like young leaves, larger
        // ones like mature pads.
        let radius = 4.5 + pseudo_rand(seed + 2.0) * 4.0; // ~4.5–8.5 world units
        let rim_phase = pseudo_rand(seed + 3.0) * TAU;
        let rotation = pseudo_rand(seed + 4.0) * TAU;
        let rate_mag = 0.10 + pseudo_rand(seed + 5.0) * 0.20;
        let rate_sign = if pseudo_rand(seed + 9.0) > 0.5 {
            1.0
        } else {
            -1.0
        };
        let rotation_rate = rate_mag * rate_sign;
        // One V-wedge cut per pad. Three independent rolls: where it
        // points, how wide it opens at the rim, and how deep its tip
        // sits inside the disc.
        let wedge_angle = pseudo_rand(seed + 11.0) * TAU;
        let wedge_hw = WEDGE_HW_MIN + pseudo_rand(seed + 12.0) * (WEDGE_HW_MAX - WEDGE_HW_MIN);
        let wedge_inner = WEDGE_INNER_NP_MIN
            + pseudo_rand(seed + 13.0) * (WEDGE_INNER_NP_MAX - WEDGE_INNER_NP_MIN);
        let notches = vec![(wedge_angle, wedge_hw, wedge_inner)];
        pads.push(LilyPad::new(
            x,
            y,
            radius,
            rim_phase,
            rotation,
            rotation_rate,
            notches,
        ));
    }
    pads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pad() -> LilyPad {
        LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4, 0.3)])
    }

    /// Empirical painted-pixel ratio against the full disc — used by
    /// the tests that bound how much of the leaf the wedge eats.
    fn painted_ratio(pad: &LilyPad, scale: f64) -> f64 {
        let r_sp = pad.radius * scale;
        let r_int = r_sp.ceil() as i32;
        // Canvas large enough to fully contain the pad on every side.
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

    #[test]
    fn lily_pad_new_holds_position() {
        let p = make_pad();
        assert!((p.x - 20.0).abs() < 1e-10);
        assert!((p.y - 15.0).abs() < 1e-10);
        assert_eq!(p.velocity(), (0.0, 0.0));
    }

    #[test]
    fn radius_stays_within_envelope() {
        let p = LilyPad::new(0.0, 0.0, 4.0, 1.3, 0.5, 0.0, vec![(0.0, 0.4, 0.3)]);
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
    fn notch_creates_a_gap_on_the_rim() {
        // Pad with a V-wedge cut pointing east, tip at np=0.30. The
        // rim pixel due east should be inside the wedge and therefore
        // unpainted.
        let p = LilyPad::new(40.0, 30.0, 6.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4, 0.3)]);
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
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4, 0.3)]);
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
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.0, 0.0, 0.0, vec![(0.0, 0.4, 0.3)]);
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
        let mut p = LilyPad::new(20.0, 15.0, 5.0, 0.7, 0.0, 0.0, vec![(0.0, 0.4, 0.3)]);
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

    // -- wedge geometry invariants ----------------------------------------

    #[test]
    fn wedge_never_cuts_inside_its_inner_np() {
        // A wedge at angle 0 with inner_np=0.40 must leave every pixel
        // with np <= 0.40 alone, regardless of angle.
        let p = LilyPad::new(0.0, 0.0, 10.0, 0.0, 0.0, 0.0, vec![(0.0, 0.5, 0.40)]);
        for i in 0..72 {
            let angle = i as f64 / 72.0 * TAU - PI;
            for &np in &[0.0_f64, 0.10, 0.20, 0.30, 0.39, 0.40] {
                assert!(
                    !p.in_any_notch(angle, np),
                    "wedge cut at np={np}, angle={angle} (inner_np=0.40 should be safe)",
                );
            }
        }
    }

    #[test]
    fn wedge_tapers_to_a_point_at_inner_np() {
        // Just inside inner_np the cut should be very narrow; near
        // the rim it should reach (close to) the full hw_at_rim.
        let p = LilyPad::new(0.0, 0.0, 10.0, 0.0, 0.0, 0.0, vec![(0.0, 0.5, 0.40)]);
        // 0.30 rad off-axis: that's well inside hw_at_rim=0.5 but
        // should NOT be inside the wedge when np barely exceeds the
        // inner threshold.
        assert!(
            !p.in_any_notch(0.30, 0.42),
            "wedge should taper — off-axis pixel near tip must stay painted",
        );
        // Same angle near the rim: now well inside the wedge.
        assert!(
            p.in_any_notch(0.30, 0.99),
            "wedge should be at near-full width near the rim",
        );
    }

    #[test]
    fn wedge_only_cuts_around_its_centre_angle() {
        // The wedge points east (angle 0). At the rim, anything more
        // than `hw_at_rim + jitter_amp` away from east must stay.
        let hw = 0.4;
        let p = LilyPad::new(0.0, 0.0, 10.0, 0.0, 0.0, 0.0, vec![(0.0, hw, 0.30)]);
        let safe_offset = hw + WEDGE_JITTER_AMP + 0.02;
        // Probe several angles well outside the wedge cone.
        for &probe in &[PI, PI / 2.0, -PI / 2.0, PI - 0.1, -PI + 0.1] {
            assert!(
                !p.in_any_notch(probe, 0.99),
                "angle {probe} should be outside the wedge (centre=0, hw={hw}, safe>{safe_offset})",
            );
        }
    }

    // -- 40% painted-area floor -------------------------------------------

    #[test]
    fn every_spawned_pad_keeps_at_least_40_percent_painted() {
        // The user-facing rule: the leaf must remain visibly a leaf.
        // We render each spawned pad and count how much of the full
        // disc is still painted green. With the parameter ranges
        // chosen, every pad should retain well over 40%.
        let pads = spawn_pads(80.0, 46.0);
        for (i, pad) in pads.iter().enumerate() {
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
    fn pad_without_any_wedge_paints_almost_the_whole_disc() {
        // Sanity check on painted_ratio itself: a pad with zero
        // wedges should paint ≥ 99% of its disc.
        let p = LilyPad::new(32.9, 10.9, 6.5, 0.0, 0.0, 0.0, vec![]);
        let ratio = painted_ratio(&p, 2.0);
        assert!(
            ratio > 0.99,
            "pad with no wedge should be fully painted: got {:.3}",
            ratio,
        );
    }

    #[test]
    fn worst_case_wedge_still_leaves_disc_majority_painted() {
        // Manually pick the most aggressive wedge inside the spawn
        // ranges and verify it still leaves > 60% of the disc.
        let p = LilyPad::new(
            20.0,
            15.0,
            6.5,
            0.0,
            0.0,
            0.0,
            vec![(0.0, WEDGE_HW_MAX, WEDGE_INNER_NP_MIN)],
        );
        let ratio = painted_ratio(&p, 2.0);
        assert!(
            ratio >= 0.60,
            "worst-case wedge cut too much: only {:.1}% painted",
            ratio * 100.0,
        );
    }

    #[test]
    fn many_random_pads_keep_at_least_40_percent_painted() {
        // Generate a hundred deterministic pads with varied wedge
        // parameters drawn from the same ranges as spawn_pads and
        // assert the floor holds for every one of them.
        use crate::rng::pseudo_rand;
        for i in 0..100u32 {
            let s = i as f64 * 7.13 + 0.5;
            let hw = WEDGE_HW_MIN + pseudo_rand(s) * (WEDGE_HW_MAX - WEDGE_HW_MIN);
            let inner = WEDGE_INNER_NP_MIN
                + pseudo_rand(s + 1.0) * (WEDGE_INNER_NP_MAX - WEDGE_INNER_NP_MIN);
            let centre = pseudo_rand(s + 2.0) * TAU;
            let p = LilyPad::new(20.0, 15.0, 6.5, 0.0, 0.0, 0.0, vec![(centre, hw, inner)]);
            let ratio = painted_ratio(&p, 2.0);
            assert!(
                ratio >= 0.40,
                "iter {i} (hw={hw:.3}, inner={inner:.3}): only {:.1}% painted",
                ratio * 100.0,
            );
        }
    }
}
