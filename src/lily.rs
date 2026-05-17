//! Lotus pads (ハスの葉) floating on the pond surface.
//!
//! Visually they're round dark-green discs scattered across the pond.
//! For a first-pass implementation they're static — placed deterministically
//! at Pond::new and drawn each frame with a tiny wobble in the radius so
//! the silhouette feels alive rather than stamped.

use crate::canvas::Canvas;
use std::f64::consts::TAU;

/// Inner / mid / edge color bands. Lotus leaves are darker at the rim
/// and slightly lighter through the body; values picked to read clearly
/// against the dark-blue pond water.
const FILL: (u8, u8, u8) = (60, 110, 45);
const MID: (u8, u8, u8) = (45, 85, 35);
const EDGE: (u8, u8, u8) = (22, 50, 25);
/// Tiny darker dot in the center suggesting the petiole attachment point.
const HUB: (u8, u8, u8) = (30, 60, 28);

pub struct LilyPad {
    pub x: f64,
    pub y: f64,
    /// Resting radius in world units.
    radius: f64,
    /// Phase offset for the gentle radius wobble so multiple pads don't
    /// breathe in sync.
    phase: f64,
}

impl LilyPad {
    pub fn new(x: f64, y: f64, radius: f64, phase: f64) -> Self {
        LilyPad {
            x,
            y,
            radius,
            phase,
        }
    }

    /// Sample the current radius — extremely subtle wobble (≤ 3% of the
    /// resting radius) so the pads don't look perfectly static.
    fn radius_at(&self, t: f64) -> f64 {
        let wobble = (t * 0.25 + self.phase).sin() * 0.03;
        self.radius * (1.0 + wobble)
    }

    pub fn draw(&self, canvas: &mut Canvas, scale: f64, t: f64) {
        let cx_px = self.x * scale;
        let cy_px = self.y * scale;
        let r_px = self.radius_at(t) * scale;
        let r_int = r_px.ceil() as i32;

        for dy in -r_int..=r_int {
            for dx in -r_int..=r_int {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                if d > r_px {
                    continue;
                }
                // Normalised radial position: 0 at center, 1 at rim.
                let np = d / r_px;
                let (r, g, b) = if np > 0.92 {
                    EDGE
                } else if np > 0.70 {
                    MID
                } else if np < 0.08 {
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
/// Pads are spread across the pond avoiding the rim, with varied sizes
/// and individual wobble phases.
pub fn spawn_pads(w: f64, h: f64) -> Vec<LilyPad> {
    use crate::rng::pseudo_rand;
    const N: usize = 6;
    let mut pads = Vec::with_capacity(N);
    for i in 0..N {
        let seed = i as f64 * 13.7 + 4.2;
        // Keep pads inside the central 80% of the pond.
        let x = (0.1 + pseudo_rand(seed) * 0.8) * w;
        let y = (0.1 + pseudo_rand(seed + 1.0) * 0.8) * h;
        // Radius 2.5–5.0 world units → ~3-5 cells across visually.
        let radius = 2.5 + pseudo_rand(seed + 2.0) * 2.5;
        let phase = pseudo_rand(seed + 3.0) * TAU;
        pads.push(LilyPad::new(x, y, radius, phase));
    }
    pads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lily_pad_new_holds_position() {
        let p = LilyPad::new(10.0, 20.0, 3.5, 0.0);
        assert!((p.x - 10.0).abs() < 1e-10);
        assert!((p.y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn radius_wobbles_within_3_percent() {
        let p = LilyPad::new(0.0, 0.0, 4.0, 1.3);
        for i in 0..200 {
            let t = i as f64 * 0.1;
            let r = p.radius_at(t);
            assert!(
                r >= 4.0 * 0.97 && r <= 4.0 * 1.03,
                "radius must wobble within ±3% of 4.0, got {r}"
            );
        }
    }

    #[test]
    fn draw_produces_visible_pixels() {
        let p = LilyPad::new(20.0, 15.0, 3.0, 0.0);
        let mut canvas = Canvas::new(80, 60);
        p.draw(&mut canvas, 2.0, 0.0);
        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(
            lit > 50,
            "lily pad disc should light many pixels, got {lit}"
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
            assert!(p.x > 0.0 && p.x < w, "pad x={} outside pond", p.x);
            assert!(p.y > 0.0 && p.y < h, "pad y={} outside pond", p.y);
        }
    }
}
