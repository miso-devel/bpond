//! Koi fish: chain-based spine physics, biomechanics-accurate fins,
//! and braille rendering.

use crate::canvas::Canvas;
use std::f64::consts::PI;

/// Number of segments in the spine chain.
pub const N_SPINE: usize = 40;
/// Distance between spine points (world units = terminal cells).
const SEG_LEN: f64 = 0.55;
/// Tail-beat frequency (Hz).
const FREQ: f64 = 1.2;
/// Total body length in world units.
const BODY_TOTAL: f64 = N_SPINE as f64 * SEG_LEN;

/// Body half-width at a given normalized position (0=head, 1=tail).
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

/// A single koi fish with chain-dynamics spine.
pub struct Koi {
    pub spine_x: [f64; N_SPINE],
    pub spine_y: [f64; N_SPINE],
    pub heading: f64,
    pub speed: f64,
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

        // Initialize spine as a straight line behind the head
        for i in 0..N_SPINE {
            koi.spine_x[i] = x - (i as f64) * SEG_LEN * heading.cos();
            koi.spine_y[i] = y - (i as f64) * SEG_LEN * heading.sin();
        }

        // Generate pseudo-random red patches (2–5 patches of varying size)
        let n_patches = 2 + ((id * 3.7).sin().abs() * 3.5) as usize;
        for p in 0..n_patches {
            let center =
                ((id * (p as f64 + 1.0) * 2.3 + 0.7).sin().abs() * 0.7 + 0.08) * N_SPINE as f64;
            let half_w =
                ((id * (p as f64 + 1.0) * 1.7 + 1.3).cos().abs() * 0.12 + 0.04) * N_SPINE as f64;
            for i in 0..N_SPINE {
                if (i as f64 - center).abs() < half_w {
                    koi.red_mask[i] = true;
                }
            }
        }
        koi
    }

    /// Advance the koi by one time step.
    pub fn update(&mut self, dt: f64, t: f64, w: f64, h: f64) {
        // Periodic turn decisions
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

        // Smoothly approach target turn rate
        let approach = 0.6 * dt;
        if (self.target_turn - self.turn_rate).abs() < approach {
            self.turn_rate = self.target_turn;
        } else if self.target_turn > self.turn_rate {
            self.turn_rate += approach;
        } else {
            self.turn_rate -= approach;
        }
        self.turn_rate = self.turn_rate.clamp(-0.45, 0.45);

        // Swimming undulation added to heading
        let swim_wave = (t * 2.0 * PI * FREQ).sin() * 0.10;
        self.heading += (self.turn_rate + swim_wave) * dt;

        // Steer back only when fully off-screen
        let margin = 5.0;
        let fully_out = self.spine_x[0] < -margin
            || self.spine_x[0] > w + margin
            || self.spine_y[0] < -margin
            || self.spine_y[0] > h + margin;
        if fully_out {
            let toward =
                (h / 2.0 - self.spine_y[0]).atan2(w / 2.0 - self.spine_x[0]);
            let diff = (toward - self.heading + PI).rem_euclid(2.0 * PI) - PI;
            self.heading += diff * 0.3 * dt;
        }

        // Move head forward
        let burst = if (t * 0.1 + self.id).sin() > 0.97 { 1.5 } else { 1.0 };
        self.spine_x[0] += self.heading.cos() * self.speed * burst * dt;
        self.spine_y[0] += self.heading.sin() * self.speed * burst * dt;

        // Chain dynamics: each segment follows the one ahead
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

    /// Draw the koi onto the braille canvas.
    pub fn draw(&self, canvas: &mut Canvas, t: f64, scale: f64) {
        let to_px = |wx: f64, wy: f64| -> (i32, i32) {
            ((wx * scale) as i32, (wy * scale) as i32)
        };

        let tangent_at = |i: usize| -> (f64, f64) {
            let i2 = (i + 1).min(N_SPINE - 1);
            let i1 = i.saturating_sub(1);
            let dx = self.spine_x[i1] - self.spine_x[i2];
            let dy = self.spine_y[i1] - self.spine_y[i2];
            let l = (dx * dx + dy * dy).sqrt().max(0.001);
            (dx / l, dy / l)
        };

        let normal_at = |i: usize| -> (f64, f64) {
            let (tx, ty) = tangent_at(i);
            (-ty, tx)
        };

        // Shadow
        for i in (0..N_SPINE).step_by(2) {
            let s = i as f64 / N_SPINE as f64;
            let hw = body_width(s) * 0.7;
            let (nx, ny) = normal_at(i);
            let steps = (hw * scale * 1.2) as i32 + 1;
            for pi in -steps..=steps {
                let p = pi as f64 / (steps as f64 / hw);
                if p.abs() > hw {
                    continue;
                }
                let (px, py) = to_px(self.spine_x[i] + nx * p, self.spine_y[i] + ny * p);
                canvas.dot(px + 3, py + 5, 3, 6, 12);
            }
        }

        // Tail fin — fixed fan shape, rides on spine's natural sway
        for lobe in [-1.0f64, 1.0] {
            for ti in 0..20 {
                let ft = ti as f64 / 20.0;
                let idx = (N_SPINE - 7 + (ft * 6.0) as usize).min(N_SPINE - 1);
                let (nx, ny) = normal_at(idx);
                let spread = lobe * (0.3 + ft * 2.8);
                let (px, py) =
                    to_px(self.spine_x[idx] + nx * spread, self.spine_y[idx] + ny * spread);
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

        // Pectoral fins (angle-based, left/right alternate)
        self.draw_fin_pair(canvas, t, scale, 0.20, 15.0, 30.0, 0.12, 1.5, &normal_at, &tangent_at);

        // Pelvic fins (smaller, slight phase offset)
        self.draw_fin_pair(canvas, t, scale, 0.45, 10.0, 20.0, 0.08, 1.0, &normal_at, &tangent_at);

        // Body
        for i in 0..N_SPINE {
            let s = i as f64 / N_SPINE as f64;
            let hw = body_width(s);
            let (nx, ny) = normal_at(i);
            let steps = (hw * scale * 2.0) as i32 + 1;
            for pi in -steps..=steps {
                let p = pi as f64 / (steps as f64 / hw);
                if p.abs() > hw {
                    continue;
                }
                let np = (p / hw).abs();
                let (px, py) =
                    to_px(self.spine_x[i] + nx * p, self.spine_y[i] + ny * p);
                let outline = np > 0.78;
                let is_red = self.red_mask[i] && np < 0.72;
                let (r, g, b) = if outline {
                    (30, 25, 18)
                } else if is_red {
                    (235, 45, 25)
                } else {
                    (255, 252, 242)
                };
                canvas.fat(px, py, r, g, b);
            }
        }
    }

    fn draw_fin_pair(
        &self,
        canvas: &mut Canvas,
        t: f64,
        scale: f64,
        spine_pos: f64,
        rest_deg: f64,
        amp_deg: f64,
        len_frac: f64,
        radius: f64,
        normal_at: &dyn Fn(usize) -> (f64, f64),
        tangent_at: &dyn Fn(usize) -> (f64, f64),
    ) {
        let idx = (N_SPINE as f64 * spine_pos) as usize;
        if idx >= N_SPINE {
            return;
        }
        let (nx, ny) = normal_at(idx);
        let (tx, ty) = tangent_at(idx);
        let rest = rest_deg.to_radians();
        let amp = amp_deg.to_radians();
        let fin_len = BODY_TOTAL * len_frac;

        for (side, is_left) in [(-1.0f64, true), (1.0, false)] {
            let phase = if is_left { 0.0 } else { PI };
            let angle = rest + amp * (2.0 * PI * FREQ * t + phase).sin();
            for fi in 0..12 {
                let ft = fi as f64 / 12.0;
                let spread = side * (angle.sin() * (1.0 - ft * 0.5)) * radius;
                let along = -ft * fin_len;
                let wx = self.spine_x[idx] + nx * spread + tx * along;
                let wy = self.spine_y[idx] + ny * spread + ty * along;
                let px = (wx * scale) as i32;
                let py = (wy * scale) as i32;
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
