//! Koi rendering: tail, fins, body, and eyes drawn into the braille canvas.

use super::*;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Body color shader
// ---------------------------------------------------------------------------

/// Smooth color along the perpendicular axis of the body. `np` is the
/// normalized distance from the spine: 0 at centerline, 1 at silhouette
/// edge. Three bands meet at 0.65 / 0.78 / 0.92 with linear blends so
/// edges anti-alias naturally through canvas-level color averaging.
fn body_color(np: f64, red: bool) -> (u8, u8, u8) {
    const SILHOUETTE: (f64, f64, f64) = (8.0, 6.0, 5.0);
    const OUTLINE: (f64, f64, f64) = (32.0, 26.0, 20.0);
    const WHITE: (f64, f64, f64) = (255.0, 252.0, 242.0);
    const RED: (f64, f64, f64) = (245.0, 35.0, 15.0);

    let interior = if red && np < 0.72 { RED } else { WHITE };

    let (r, g, b) = if np > 0.92 {
        SILHOUETTE
    } else if np > 0.78 {
        let t = ((np - 0.78) / 0.14).clamp(0.0, 1.0);
        lerp3(OUTLINE, SILHOUETTE, t)
    } else if np > 0.65 {
        let t = ((np - 0.65) / 0.13).clamp(0.0, 1.0);
        lerp3(interior, OUTLINE, t)
    } else {
        interior
    };
    (r as u8, g as u8, b as u8)
}

fn lerp3(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

// ---------------------------------------------------------------------------
// Fin geometry
// ---------------------------------------------------------------------------

struct FinParams {
    spine_pos: f64,
    rest_deg: f64,
    amp_deg: f64,
    len_frac: f64,
    radius: f64,
    /// Pectoral fins steer; the inside fin extends to brake during turns.
    /// Pelvic fins are stabilizers and don't asymmetrically deflect.
    steering: bool,
}

const PECTORAL_FINS: FinParams = FinParams {
    spine_pos: 0.20,
    rest_deg: 15.0,
    amp_deg: 30.0,
    len_frac: 0.12,
    radius: 1.5,
    steering: true,
};

const PELVIC_FINS: FinParams = FinParams {
    spine_pos: 0.45,
    rest_deg: 10.0,
    amp_deg: 20.0,
    len_frac: 0.08,
    radius: 1.0,
    steering: false,
};

// ---------------------------------------------------------------------------
// Draw impl
// ---------------------------------------------------------------------------

impl Koi {
    pub(super) fn tangent_at(&self, i: usize) -> (f64, f64) {
        let i2 = (i + 1).min(N_SPINE - 1);
        let i1 = i.saturating_sub(1);
        let dx = self.spine_x[i1] - self.spine_x[i2];
        let dy = self.spine_y[i1] - self.spine_y[i2];
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        (dx / l, dy / l)
    }

    pub(super) fn normal_at(&self, i: usize) -> (f64, f64) {
        let (tx, ty) = self.tangent_at(i);
        (-ty, tx)
    }

    fn to_px(wx: f64, wy: f64, scale: f64) -> (i32, i32) {
        ((wx * scale) as i32, (wy * scale) as i32)
    }

    pub fn draw(&self, canvas: &mut Canvas, t: f64, scale: f64) {
        self.draw_tail(canvas, scale);
        self.draw_fin_pair(canvas, t, scale, &PECTORAL_FINS);
        self.draw_fin_pair(canvas, t, scale, &PELVIC_FINS);
        self.draw_body(canvas, scale);
        self.draw_eyes(canvas, scale);
    }

    fn draw_eyes(&self, canvas: &mut Canvas, scale: f64) {
        // Eyes sit on each side of the head, ~10% back from the snout.
        // 2×2 dark pupil with a single bright catchlight on top.
        let i = 3;
        let s = i as f64 / N_SPINE as f64;
        let hw = physics::body_width(s);
        let (nx, ny) = self.normal_at(i);
        let off = hw * 0.55;
        for side in [-1.0f64, 1.0] {
            let ex = self.spine_x[i] + nx * off * side;
            let ey = self.spine_y[i] + ny * off * side;
            let (px, py) = Self::to_px(ex, ey, scale);
            canvas.fat(px, py, 8, 5, 12);
            // Highlight overwrites the pupil's top-left dot.
            canvas.dot(px, py, 240, 235, 215);
        }
    }

    fn draw_tail(&self, canvas: &mut Canvas, scale: f64) {
        // Real koi caudal fin is roughly as wide as the body itself —
        // ~2.5 world units per lobe at peak. The previous (0.3 + ft*2.8)
        // setting was nearly double that.
        let spread_scale = 0.85 + 0.20 * self.burst.clamp(0.2, 2.5);
        for lobe in [-1.0f64, 1.0] {
            for ray_offset in [-0.35_f64, 0.0, 0.35] {
                for ti in 0..22 {
                    let ft = ti as f64 / 22.0;
                    let idx = (N_SPINE - 7 + (ft * 6.0) as usize).min(N_SPINE - 1);
                    let (nx, ny) = self.normal_at(idx);
                    let spread = lobe * (0.2 + ft * 1.4) * spread_scale * (1.0 + ray_offset * ft);
                    let (px, py) = Self::to_px(
                        self.spine_x[idx] + nx * spread,
                        self.spine_y[idx] + ny * spread,
                        scale,
                    );
                    let a = (1.0 - ft * 0.35) * 0.5;
                    let r = (220.0 * a) as u8;
                    let g = (210.0 * a) as u8;
                    let b = (190.0 * a) as u8;
                    if ft < 0.3 {
                        canvas.fat(px, py, r, g, b);
                    } else {
                        canvas.dot(px, py, r, g, b);
                    }
                }
            }
        }
    }

    fn draw_body(&self, canvas: &mut Canvas, scale: f64) {
        // Sample twice along the spine and use single sub-pixel dots so
        // the body shape comes out smooth instead of in chunky 2×2 blocks.
        const SUB: i32 = 2;
        let n_along = (N_SPINE as i32) * SUB;
        for ss in 0..n_along {
            let i_f = ss as f64 / SUB as f64;
            let i0 = (i_f as usize).min(N_SPINE - 1);
            let i1 = (i0 + 1).min(N_SPINE - 1);
            let frac = i_f - i0 as f64;
            let cx = self.spine_x[i0] * (1.0 - frac) + self.spine_x[i1] * frac;
            let cy = self.spine_y[i0] * (1.0 - frac) + self.spine_y[i1] * frac;
            let (nx, ny) = self.normal_at(i0);

            let s = i_f / N_SPINE as f64;
            let hw = physics::body_width(s);
            let steps = (hw * scale * 3.0) as i32 + 1;
            let red_here = self.red_mask[i0];
            for pi in -steps..=steps {
                let p = pi as f64 / (steps as f64 / hw);
                if p.abs() > hw {
                    continue;
                }
                let np = (p / hw).abs();
                let (px, py) = Self::to_px(cx + nx * p, cy + ny * p, scale);
                let (r, g, b) = body_color(np, red_here);
                canvas.dot(px, py, r, g, b);
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

        // Beat amplitude scales with current effort: a sprinting koi rows
        // harder, a hovering one barely moves its fins.
        let beat_scale = 0.65 + 0.45 * self.burst.clamp(0.2, 2.5);

        for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
            let phase = if is_left { 0.0 } else { PI };
            // For steering (pectoral) fins the inside fin (in the direction
            // of the turn) extends more to act as a brake; the outside fin
            // tucks streamlined. tanh keeps it smooth and bounded.
            let asym = if params.steering {
                1.0 + side * (self.turn_rate * 1.6).tanh() * 0.55
            } else {
                1.0
            };
            let angle = rest + amp * beat_scale * asym * (2.0 * PI * FREQ * t + phase).sin();
            // Three rays per fin form a fan: front ray extends laterally,
            // rear rays sweep back.
            for ray_offset in [-0.45_f64, 0.0, 0.45] {
                for fi in 0..14 {
                    let ft = fi as f64 / 14.0;
                    let spread = side
                        * (angle.sin() * (1.0 - ft * 0.5))
                        * params.radius
                        * (1.0 + ray_offset * ft * 0.7);
                    let along = -ft * fin_len * (1.0 + ray_offset * 0.4);
                    let wx = self.spine_x[idx] + nx * spread + tx * along;
                    let wy = self.spine_y[idx] + ny * spread + ty * along;
                    let (px, py) = Self::to_px(wx, wy, scale);
                    let a = (1.0 - ft * 0.7) * 0.55;
                    let r = (210.0 * a) as u8;
                    let g = (200.0 * a) as u8;
                    let b = (182.0 * a) as u8;
                    if ft < 0.18 {
                        canvas.fat(px, py, r, g, b);
                    } else {
                        canvas.dot(px, py, r, g, b);
                    }
                }
            }
        }
    }
}
