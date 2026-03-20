//! Koi rendering: shadow, tail, body, and fin pair drawing.

use super::*;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Fin geometry
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
        self.draw_shadow(canvas, scale);
        self.draw_tail(canvas, scale);
        self.draw_fin_pair(canvas, t, scale, &PECTORAL_FINS);
        self.draw_fin_pair(canvas, t, scale, &PELVIC_FINS);
        self.draw_body(canvas, scale);
    }

    fn draw_shadow(&self, canvas: &mut Canvas, scale: f64) {
        for i in (0..N_SPINE).step_by(2) {
            let s = i as f64 / N_SPINE as f64;
            let hw = physics::body_width(s) * 0.7;
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
            let hw = physics::body_width(s);
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
