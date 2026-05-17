//! Lotus pads (ハスの葉) floating on the pond surface.
//!
//! A real water-lily / lotus leaf seen from above has a distinctive set
//! of features beyond "round green disc":
//!
//! 1. A **V-shaped notch** cut from the rim toward (but not reaching)
//!    the petiole — the single most recognisable silhouette feature.
//! 2. **Radial veins** spreading out from the hub — typically 12-18
//!    visible primary veins.
//! 3. **Water droplets** beading up on the leaf surface (the famous
//!    "lotus effect" from the leaf's nano-textured wax coating).
//! 4. A **sunlit crescent** on the rim suggesting the leaf's gentle
//!    saucer / cup shape and the angle of the light.
//! 5. **Drift** — pads on a pond aren't static. Wind, currents, and
//!    fish brushing past keep them in continuous motion.
//!
//! All five are implemented here.

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
/// Lotus-effect water droplets: pale, slightly bluish.
const DROPLET: (u8, u8, u8) = (175, 220, 200);
/// Small darker pixel under each droplet for a touch of contact shadow.
const DROPLET_SHADOW: (u8, u8, u8) = (40, 75, 40);

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

/// Notch geometry midpoints, only used by tests. Spawned pads pick a
/// per-pad "notch size" in [0, 1] which drives both depth and width
/// together — the only visible difference between pads is whether
/// their く-notch is big or small.
#[cfg(test)]
const NOTCH_INNER_NP: f64 = 0.65;
#[cfg(test)]
const NOTCH_HALF_WIDTH_MAX: f64 = 0.15;

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
    /// Angle (radians, pad-local frame) where the V-notch points outward.
    notch_angle: f64,
    /// Per-pad notch depth as a normalized-radius threshold. The V
    /// extends inward from the rim to this np value; varies slightly
    /// per pad so each leaf has a slightly different bite.
    notch_inner_np: f64,
    /// Per-pad notch half-width at the rim, in radians.
    notch_half_width: f64,
    /// Angle of the sun-lit highlight crescent (pad-local frame).
    highlight_angle: f64,
    /// Water droplets in pad-local polar coords: `(r_frac, angle)`.
    droplets: Vec<(f64, f64)>,
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
        notch_angle: f64,
        notch_inner_np: f64,
        notch_half_width: f64,
        highlight_angle: f64,
        droplets: Vec<(f64, f64)>,
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
            notch_angle,
            notch_inner_np,
            notch_half_width,
            highlight_angle,
            droplets,
        }
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

                // Force the very center to hub colour so the petiole
                // attachment reads cleanly.
                if d < 0.5 {
                    canvas.dot(cx_px as i32 + dx, cy_px as i32 + dy, HUB.0, HUB.1, HUB.2);
                    continue;
                }

                let angle = dyf.atan2(dxf);
                let r_local = self.radius_at(t, angle) * scale;
                if d > r_local {
                    continue;
                }
                let np = d / r_local;
                let local_angle = angle - self.rotation;

                // V-notch cut from the rim toward (but not reaching)
                // the hub. Widest at the rim, narrowing inward. Per-pad
                // depth and width let each leaf have its own bite.
                if np > self.notch_inner_np {
                    let progress = (np - self.notch_inner_np) / (1.0 - self.notch_inner_np);
                    let half_w = self.notch_half_width * progress;
                    if Self::angle_dist(local_angle, self.notch_angle) < half_w {
                        continue;
                    }
                }

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

        // 2) Paint water droplets last so they overlay the leaf body.
        for &(r_frac, drop_angle) in &self.droplets {
            let abs_angle = drop_angle + self.rotation;
            let drop_world_x = self.x + r_frac * self.radius * abs_angle.cos();
            let drop_world_y = self.y + r_frac * self.radius * abs_angle.sin();
            let px = (drop_world_x * scale) as i32;
            let py = (drop_world_y * scale) as i32;
            // Subtle 2-pixel droplet with a touch of shadow underneath
            // for a hint of 3-D bead.
            canvas.dot(px, py, DROPLET.0, DROPLET.1, DROPLET.2);
            canvas.dot(px + 1, py, DROPLET.0, DROPLET.1, DROPLET.2);
            canvas.dot(
                px,
                py + 1,
                DROPLET_SHADOW.0,
                DROPLET_SHADOW.1,
                DROPLET_SHADOW.2,
            );
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
        let notch_angle = pseudo_rand(seed + 6.0) * TAU;
        // The only meaningful per-pad variation: how big the く-notch
        // is. A single random "size" parameter drives both depth and
        // width so the bite scales coherently — some pads have a big
        // く wedge bitten out of them, others a small one.
        let notch_size = pseudo_rand(seed + 10.0);
        // Conservative range — even the "big" bite stays a disciplined
        // く shape. Anything deeper or wider stops reading as a lotus
        // leaf and starts looking broken.
        let notch_inner_np = 0.85 - notch_size * 0.23; // 0.85 (small) → 0.62 (big)
        let notch_half_width = 0.08 + notch_size * 0.10; // 0.08 rad (small) → 0.18 rad (big)
        let highlight_angle = pseudo_rand(seed + 7.0) * TAU;
        let droplet_count = 3 + (pseudo_rand(seed + 8.0) * 4.0) as usize; // 3-6
        let mut droplets = Vec::with_capacity(droplet_count);
        for j in 0..droplet_count {
            let ds = seed + 100.0 + j as f64 * 2.3;
            // Keep droplets in the body of the leaf — not in the notch
            // wedge. Re-roll if too close to it.
            let mut a = pseudo_rand(ds) * TAU;
            for _ in 0..4 {
                if LilyPad::angle_dist(a, notch_angle) > notch_half_width * 1.2 {
                    break;
                }
                a = pseudo_rand(ds + a) * TAU;
            }
            let r_frac = 0.20 + pseudo_rand(ds + 1.0) * 0.50;
            droplets.push((r_frac, a));
        }
        pads.push(LilyPad::new(
            x,
            y,
            radius,
            rim_phase,
            rotation,
            rotation_rate,
            notch_angle,
            notch_inner_np,
            notch_half_width,
            highlight_angle,
            droplets,
        ));
    }
    pads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pad() -> LilyPad {
        LilyPad::new(
            20.0,
            15.0,
            5.0,
            0.0,
            0.0,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![(0.4, 1.0), (0.5, 3.0)],
        )
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
        let p = LilyPad::new(
            0.0,
            0.0,
            4.0,
            1.3,
            0.5,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![],
        );
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
    fn draw_renders_droplets() {
        let p = make_pad();
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == (DROPLET.0, DROPLET.1, DROPLET.2)
            });
        assert!(found, "water droplets should be visible");
    }

    #[test]
    fn notch_creates_a_gap_on_the_rim() {
        // Pad with notch pointing east. Sample the rim near that angle —
        // pixels in the V wedge should NOT be painted.
        let p = LilyPad::new(
            40.0,
            30.0,
            6.0,
            0.0,
            0.0,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![],
        );
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
        let mut p = LilyPad::new(
            20.0,
            15.0,
            5.0,
            0.0,
            0.0,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![],
        );
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
        let mut p = LilyPad::new(
            20.0,
            15.0,
            5.0,
            0.0,
            0.0,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![],
        );
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
        let mut p = LilyPad::new(
            20.0,
            15.0,
            5.0,
            0.7,
            0.0,
            0.0,
            0.0,
            NOTCH_INNER_NP,
            NOTCH_HALF_WIDTH_MAX,
            PI,
            vec![],
        );
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
