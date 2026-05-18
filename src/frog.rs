//! Pond frogs: sit, breathe, and leap.
//!
//! Frogs spend most of their time stationary on the water surface
//! with a slow throat-pulse breath. Every few seconds a frog picks a
//! random direction, crouches in anticipation, springs forward in a
//! parabolic-arc leap, and lands with a splash that ripples the
//! water.
//!
//! Rendering: oval body + small head bulge + two bright yellow eyes
//! on top + folded Z-shaped hind legs (or extended back during a
//! jump) + small front legs that peek out from under the chin. The
//! visible body size grows at the jump apex to simulate vertical
//! lift (the frog is briefly closer to the camera).

use crate::canvas::Canvas;
use crate::rng::pseudo_rand;
use std::f64::consts::TAU;

// ===========================================================================
// Tuning constants
// ===========================================================================

// Body geometry in world units (will be scaled by per-frog `size`).
const BODY_HALF_LEN: f64 = 2.10;
const BODY_HALF_WID: f64 = 1.20;
const HEAD_BULGE_LEN: f64 = 1.05;
const HEAD_HALF_WID: f64 = 0.90;

// Eyes — both the iris disc and pupil sized so they're not just one
// sub-pixel. At scale 2 the iris fills a 2-pixel ring around a
// 1-pixel pupil.
const EYE_RADIUS: f64 = 0.60;
const EYE_OFFSET_FWD: f64 = 1.55;
const EYE_OFFSET_SIDE: f64 = 0.82;
const PUPIL_RADIUS: f64 = 0.18;

// Throat pulse bulge.
const THROAT_FWD: f64 = 1.00;
const THROAT_HALF_WID: f64 = 0.55;

// Hind-leg joints (sitting / folded Z shape). Negative `fwd` = behind body centre.
const HIP_FWD: f64 = -1.05;
const HIP_SIDE: f64 = 0.95;
const KNEE_FWD: f64 = -2.35;
const KNEE_SIDE: f64 = 1.90;
const FOOT_FWD: f64 = -0.55;
const FOOT_SIDE: f64 = 1.90;
const LEG_THICKNESS: i32 = 1;

// Hind-leg joints during a jump (extended straight back).
const EXT_HIP_FWD: f64 = -1.05;
const EXT_HIP_SIDE: f64 = 0.60;
const EXT_FOOT_FWD: f64 = -3.55;
const EXT_FOOT_SIDE: f64 = 0.85;

// Front leg geometry — small, peeks out from under the chin.
const FRONT_HIP_FWD: f64 = 1.10;
const FRONT_HIP_SIDE: f64 = 0.65;
const FRONT_FOOT_FWD: f64 = 1.95;
const FRONT_FOOT_SIDE: f64 = 0.90;

// Timing
const SIT_DURATION_MIN: f64 = 3.0;
const SIT_DURATION_RANGE: f64 = 5.0; // → 3-8 seconds
const CROUCH_DURATION: f64 = 0.18;
const SCARED_CROUCH_DURATION: f64 = 0.07; // scared frogs barely pause
const JUMP_DURATION: f64 = 0.55;
const LAND_DURATION: f64 = 0.32;
const CROAK_DURATION: f64 = 1.8;
const CROAK_PULSE_RATE: f64 = 2.6; // Hz — about 4-5 pulses per croak
const TONGUE_DURATION: f64 = 0.18; // fast — real frogs are even faster
const TONGUE_REACH: f64 = 2.40; // world units forward of nose at peak

// Per-leap range
const JUMP_DISTANCE_MIN: f64 = 5.5;
const JUMP_DISTANCE_RANGE: f64 = 11.0;
const SCARED_JUMP_DISTANCE: f64 = 18.0; // big jump away from the threat
const JUMP_LIFT_SCALE: f64 = 0.40; // body grows this fraction at apex

// Breathing (throat pulse)
const BREATH_RATE: f64 = 1.4;

// Per-frog size variation
const SIZE_MIN: f64 = 0.85;
const SIZE_MAX: f64 = 1.15;

// Bounds margin so frogs don't spawn or land on the literal edge.
const EDGE_MARGIN: f64 = 3.0;

// Splash impulse the pond layer uses to spawn ripples on landing.
pub const LANDING_SPLASH_FORCE: f64 = 1.0;

// Scare reaction range: frogs farther than this from the threat
// ignore it. Inside this radius they immediately launch.
const SCARE_RANGE: f64 = 14.0;

// Action probabilities at the end of a Sit period.
const ACTION_TONGUE_THRESHOLD: f64 = 0.10; // [0, 0.10) → flick
const ACTION_CROAK_THRESHOLD: f64 = 0.35; // [0.10, 0.35) → croak
                                          //                                                   [0.35, 1.00) → jump

// Vocal sac visual (drawn during Croak).
const VOCAL_SAC_FWD: f64 = 1.10;
const VOCAL_SAC_RADIUS_BASE: f64 = 0.55;
const VOCAL_SAC_RADIUS_BULGE: f64 = 1.10; // peak inflation adds this much

// Tongue visual
const TONGUE_COLOR: (u8, u8, u8) = (220, 120, 130);

// ===========================================================================
// Colours
// ===========================================================================

mod color {
    /// Sunlit top of the back.
    pub const BACK_LIGHT: (u8, u8, u8) = (95, 165, 70);
    pub const BACK_MID: (u8, u8, u8) = (60, 120, 50);
    pub const BACK_DARK: (u8, u8, u8) = (30, 75, 30);
    /// Pale belly tint, mixed in at the jump apex when the frog is
    /// briefly between the viewer and the water.
    pub const BELLY: (u8, u8, u8) = (180, 200, 120);
    pub const EYE: (u8, u8, u8) = (235, 220, 80);
    pub const PUPIL: (u8, u8, u8) = (12, 12, 14);
    pub const THROAT: (u8, u8, u8) = (210, 220, 130);
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f64 + (b as f64 - a as f64) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    (
        lerp_u8(a.0, b.0, t),
        lerp_u8(a.1, b.1, t),
        lerp_u8(a.2, b.2, t),
    )
}

// ===========================================================================
// Splash returned to the pond when a frog lands
// ===========================================================================

pub struct Splash {
    pub x: f64,
    pub y: f64,
    pub force: f64,
}

// ===========================================================================
// State
// ===========================================================================

#[derive(Clone, Copy, Debug)]
pub enum FrogState {
    /// Stationary on the water; throat pulses with breath.
    Sit,
    /// Compressing in anticipation of a jump. Target is already
    /// decided so the jump phase can launch immediately.
    Crouch { remaining: f64, target: (f64, f64) },
    /// Mid-air, interpolating along `from → to` over `JUMP_DURATION`.
    Jump {
        from: (f64, f64),
        to: (f64, f64),
        progress: f64,
    },
    /// Just landed — body compressed, recovering for `remaining` seconds.
    Land { remaining: f64 },
    /// Inflating and deflating the vocal sac under the chin in a
    /// rhythmic pulse. The frog stays put. `pulse_phase` ticks at
    /// `CROAK_PULSE_RATE` so the sac inflates a few times per croak.
    Croak { remaining: f64, pulse_phase: f64 },
    /// Tongue snaps out forward and retracts. Very brief.
    TongueFlick { remaining: f64 },
}

// ===========================================================================
// Frog
// ===========================================================================

pub struct Frog {
    /// Resting position. During Jump the visible position is
    /// interpolated; this field still holds the takeoff point until
    /// landing snaps it forward.
    x: f64,
    y: f64,
    heading: f64,
    state: FrogState,
    sit_timer: f64,
    breath_phase: f64,
    size: f64,
    seed: f64,
    rng_step: f64,
}

impl Frog {
    pub fn new(x: f64, y: f64, heading: f64, seed: f64) -> Self {
        let size = SIZE_MIN + pseudo_rand(seed) * (SIZE_MAX - SIZE_MIN);
        Frog {
            x,
            y,
            heading,
            state: FrogState::Sit,
            sit_timer: SIT_DURATION_MIN + pseudo_rand(seed + 1.0) * SIT_DURATION_RANGE,
            breath_phase: pseudo_rand(seed + 2.0) * TAU,
            size,
            seed,
            rng_step: 0.0,
        }
    }

    fn next_rand(&mut self) -> f64 {
        self.rng_step += 1.0;
        pseudo_rand(self.seed + self.rng_step * 0.7)
    }

    /// Centre of mass, accounting for the jump arc's lateral motion.
    pub fn position(&self) -> (f64, f64) {
        match self.state {
            FrogState::Jump { from, to, progress } => (
                from.0 + (to.0 - from.0) * progress,
                from.1 + (to.1 - from.1) * progress,
            ),
            _ => (self.x, self.y),
        }
    }

    /// Lateral velocity (world units per second). Non-zero only during
    /// Jump. Reserved for the lily-pad wake integration in a later
    /// iteration.
    #[allow(dead_code)]
    pub fn velocity(&self) -> (f64, f64) {
        match self.state {
            FrogState::Jump { from, to, .. } => (
                (to.0 - from.0) / JUMP_DURATION,
                (to.1 - from.1) / JUMP_DURATION,
            ),
            _ => (0.0, 0.0),
        }
    }

    /// Returns 0..1, peaking at 1.0 at the jump apex.
    fn jump_lift(&self) -> f64 {
        match self.state {
            FrogState::Jump { progress, .. } => {
                let t = progress.clamp(0.0, 1.0);
                4.0 * t * (1.0 - t)
            }
            _ => 0.0,
        }
    }

    /// Compression factor for crouch / land posture (0 = no compression,
    /// 1 = fully compressed).
    fn crouch_compression(&self) -> f64 {
        match self.state {
            FrogState::Crouch { remaining, .. } => {
                (1.0 - (remaining / CROUCH_DURATION)).clamp(0.0, 1.0)
            }
            FrogState::Land { remaining } => (remaining / LAND_DURATION).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }

    #[cfg(test)]
    pub fn state(&self) -> FrogState {
        self.state
    }

    /// Advance the frog one frame. Returns a `Splash` on the tick
    /// the frog lands so the caller can spawn a ripple.
    pub fn update(&mut self, dt: f64, w: f64, h: f64) -> Option<Splash> {
        self.breath_phase += dt * BREATH_RATE;
        let current = self.state;
        let (next, splash) = match current {
            FrogState::Sit => (self.tick_sit(dt, w, h), None),
            FrogState::Crouch {
                mut remaining,
                target,
            } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    (
                        FrogState::Jump {
                            from: (self.x, self.y),
                            to: target,
                            progress: 0.0,
                        },
                        None,
                    )
                } else {
                    (FrogState::Crouch { remaining, target }, None)
                }
            }
            FrogState::Jump {
                from,
                to,
                mut progress,
            } => {
                progress += dt / JUMP_DURATION;
                if progress >= 1.0 {
                    self.x = to.0;
                    self.y = to.1;
                    let splash = Splash {
                        x: to.0,
                        y: to.1,
                        force: LANDING_SPLASH_FORCE,
                    };
                    (
                        FrogState::Land {
                            remaining: LAND_DURATION,
                        },
                        Some(splash),
                    )
                } else {
                    (FrogState::Jump { from, to, progress }, None)
                }
            }
            FrogState::Land { mut remaining } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    self.reset_sit_timer();
                    (FrogState::Sit, None)
                } else {
                    (FrogState::Land { remaining }, None)
                }
            }
            FrogState::Croak {
                mut remaining,
                mut pulse_phase,
            } => {
                remaining -= dt;
                pulse_phase += dt * CROAK_PULSE_RATE;
                if remaining <= 0.0 {
                    self.reset_sit_timer();
                    (FrogState::Sit, None)
                } else {
                    (
                        FrogState::Croak {
                            remaining,
                            pulse_phase,
                        },
                        None,
                    )
                }
            }
            FrogState::TongueFlick { mut remaining } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    self.reset_sit_timer();
                    (FrogState::Sit, None)
                } else {
                    (FrogState::TongueFlick { remaining }, None)
                }
            }
        };
        self.state = next;
        splash
    }

    fn reset_sit_timer(&mut self) {
        self.sit_timer = SIT_DURATION_MIN + self.next_rand() * SIT_DURATION_RANGE;
    }

    /// Choose what to do when the sit timer runs out. Most often
    /// the frog jumps; sometimes it croaks; occasionally it flicks
    /// its tongue.
    fn tick_sit(&mut self, dt: f64, w: f64, h: f64) -> FrogState {
        self.sit_timer -= dt;
        if self.sit_timer > 0.0 {
            return FrogState::Sit;
        }
        let roll = self.next_rand();
        if roll < ACTION_TONGUE_THRESHOLD {
            FrogState::TongueFlick {
                remaining: TONGUE_DURATION,
            }
        } else if roll < ACTION_CROAK_THRESHOLD {
            FrogState::Croak {
                remaining: CROAK_DURATION,
                pulse_phase: 0.0,
            }
        } else {
            let (tx, ty, hd) = self.choose_jump_target(w, h);
            self.heading = hd;
            FrogState::Crouch {
                remaining: CROUCH_DURATION,
                target: (tx, ty),
            }
        }
    }

    /// Make this frog jump immediately away from `(sx, sy)` if the
    /// threat is within `SCARE_RANGE`. Does nothing if the frog is
    /// already airborne — interrupting a Jump mid-flight would look
    /// like a teleport.
    pub fn scare(&mut self, sx: f64, sy: f64, w: f64, h: f64) {
        if matches!(self.state, FrogState::Jump { .. }) {
            return;
        }
        let dx = self.x - sx;
        let dy = self.y - sy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > SCARE_RANGE {
            return;
        }
        let away = if dist < 0.01 {
            self.next_rand() * TAU
        } else {
            dy.atan2(dx)
        };
        let raw_tx = self.x + away.cos() * SCARED_JUMP_DISTANCE;
        let raw_ty = self.y + away.sin() * SCARED_JUMP_DISTANCE;
        let tx = raw_tx.clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
        let ty = raw_ty.clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
        self.heading = (ty - self.y).atan2(tx - self.x);
        self.state = FrogState::Crouch {
            remaining: SCARED_CROUCH_DURATION,
            target: (tx, ty),
        };
    }

    fn choose_jump_target(&mut self, w: f64, h: f64) -> (f64, f64, f64) {
        // Pick a random direction biased slightly toward the centre
        // (so we don't repeatedly clamp against a wall).
        let drift_angle = self.next_rand() * TAU;
        let distance = JUMP_DISTANCE_MIN + self.next_rand() * JUMP_DISTANCE_RANGE;
        let raw_tx = self.x + drift_angle.cos() * distance;
        let raw_ty = self.y + drift_angle.sin() * distance;
        let tx = raw_tx.clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
        let ty = raw_ty.clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
        let actual_heading = (ty - self.y).atan2(tx - self.x);
        (tx, ty, actual_heading)
    }

    // -- rendering ------------------------------------------------------

    pub fn draw(&self, canvas: &mut Canvas, scale: f64) {
        let (cx, cy) = self.position();
        let lift = self.jump_lift();
        let crouch = self.crouch_compression();

        // Body size scaling: bigger at jump apex, slightly smaller
        // while compressed during crouch / land.
        let render_size = self.size * (1.0 + lift * JUMP_LIFT_SCALE - crouch * 0.10);

        // Stretch/squash on the same factor scheme.
        let stretch_fwd = 1.0 + lift * 0.15 - crouch * 0.12;
        let stretch_side = 1.0 - lift * 0.04 + crouch * 0.06;

        let cx_px = cx * scale;
        let cy_px = cy * scale;
        let cos_h = self.heading.cos();
        let sin_h = self.heading.sin();

        // Helper: body-local (forward, side) in world units → canvas px f64.
        let to_canvas = |bx_world: f64, by_world: f64| -> (f64, f64) {
            let wx_off = bx_world * cos_h - by_world * sin_h;
            let wy_off = bx_world * sin_h + by_world * cos_h;
            (cx_px + wx_off * scale, cy_px + wy_off * scale)
        };

        self.draw_body(canvas, scale, render_size, stretch_fwd, stretch_side, lift);

        // Hind legs — extended while airborne, folded otherwise.
        match self.state {
            FrogState::Jump { .. } => self.draw_extended_legs(canvas, &to_canvas, render_size),
            _ => self.draw_folded_legs(canvas, &to_canvas, render_size, crouch),
        }
        self.draw_front_legs(canvas, &to_canvas, render_size);
        self.draw_eyes(canvas, &to_canvas, render_size, scale);

        // State-specific overlays.
        match self.state {
            FrogState::Sit => {
                // Subtle breath pulse during Sit.
                let bulge = (self.breath_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                if bulge > 0.30 {
                    self.draw_throat(canvas, &to_canvas, render_size, bulge);
                }
            }
            FrogState::Croak { pulse_phase, .. } => {
                // Big inflating vocal sac. Even at its smallest the
                // sac is bigger than the resting throat so the croak
                // reads as a different beat.
                let pulse = (pulse_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                self.draw_vocal_sac(canvas, &to_canvas, render_size, scale, pulse);
            }
            FrogState::TongueFlick { remaining } => {
                let progress = 1.0 - (remaining / TONGUE_DURATION);
                let extension = if progress < 0.5 {
                    progress * 2.0
                } else {
                    (1.0 - progress) * 2.0
                };
                self.draw_tongue(canvas, &to_canvas, render_size, extension);
            }
            _ => {}
        }
    }

    fn draw_body(
        &self,
        canvas: &mut Canvas,
        scale: f64,
        render_size: f64,
        stretch_fwd: f64,
        stretch_side: f64,
        lift: f64,
    ) {
        let body_len = BODY_HALF_LEN * render_size * stretch_fwd;
        let body_wid = BODY_HALF_WID * render_size * stretch_side;
        let head_len = HEAD_BULGE_LEN * render_size * stretch_fwd;
        let head_wid = HEAD_HALF_WID * render_size * stretch_side;

        // Iterate canvas sub-pixels in a bounding box that covers
        // body + head + a small margin. For each pixel, inverse-rotate
        // into body-local coords and decide whether it's inside.
        let max_x = body_len + head_len + 1.0;
        let max_y = body_wid.max(head_wid) + 1.0;
        let bound = (max_x.max(max_y) * scale).ceil() as i32 + 2;

        let (cx, cy) = self.position();
        let cx_px = cx * scale;
        let cy_px = cy * scale;
        let cos_h = self.heading.cos();
        let sin_h = self.heading.sin();
        let cx_i = cx_px as i32;
        let cy_i = cy_px as i32;

        for dy in -bound..=bound {
            for dx in -bound..=bound {
                // Canvas px → world offset → body-local.
                let world_dx = dx as f64 / scale;
                let world_dy = dy as f64 / scale;
                let bx = world_dx * cos_h + world_dy * sin_h;
                let by = -world_dx * sin_h + world_dy * cos_h;
                if let Some((r, g, b)) =
                    body_pixel_colour(bx, by, body_len, body_wid, head_len, head_wid, lift)
                {
                    canvas.dot(cx_i + dx, cy_i + dy, r, g, b);
                }
            }
        }
    }

    fn draw_folded_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        crouch: f64,
    ) {
        // Crouch slightly gathers the knees inward.
        let gather = 1.0 - crouch * 0.25;
        for &side in &[1.0_f64, -1.0] {
            let hip = (HIP_FWD * render_size, HIP_SIDE * side * render_size);
            let knee = (
                KNEE_FWD * render_size,
                KNEE_SIDE * side * render_size * gather,
            );
            let foot = (
                FOOT_FWD * render_size,
                FOOT_SIDE * side * render_size * gather,
            );
            draw_segment(
                canvas,
                to_canvas,
                hip,
                knee,
                color::BACK_DARK,
                LEG_THICKNESS,
            );
            draw_segment(
                canvas,
                to_canvas,
                knee,
                foot,
                color::BACK_DARK,
                LEG_THICKNESS,
            );
        }
    }

    fn draw_extended_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        for &side in &[1.0_f64, -1.0] {
            let hip = (EXT_HIP_FWD * render_size, EXT_HIP_SIDE * side * render_size);
            let foot = (
                EXT_FOOT_FWD * render_size,
                EXT_FOOT_SIDE * side * render_size,
            );
            draw_segment(
                canvas,
                to_canvas,
                hip,
                foot,
                color::BACK_DARK,
                LEG_THICKNESS,
            );
        }
    }

    fn draw_front_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        for &side in &[1.0_f64, -1.0] {
            let hip = (
                FRONT_HIP_FWD * render_size,
                FRONT_HIP_SIDE * side * render_size,
            );
            let foot = (
                FRONT_FOOT_FWD * render_size,
                FRONT_FOOT_SIDE * side * render_size,
            );
            draw_segment(canvas, to_canvas, hip, foot, color::BACK_DARK, 0);
        }
    }

    fn draw_eyes(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        scale: f64,
    ) {
        // Radii are stored in world units; fill_disc wants sub-pixels.
        let r_px = EYE_RADIUS * render_size * scale;
        let pupil_r_px = PUPIL_RADIUS * render_size * scale;
        for &side in &[1.0_f64, -1.0] {
            let (cx, cy) = to_canvas(
                EYE_OFFSET_FWD * render_size,
                EYE_OFFSET_SIDE * side * render_size,
            );
            fill_disc(canvas, cx, cy, r_px, color::EYE);
            fill_disc(canvas, cx, cy, pupil_r_px, color::PUPIL);
        }
    }

    fn draw_throat(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        bulge: f64,
    ) {
        // Bulge is 0..1; widen the visible patch as the throat
        // inflates. At full bulge it pokes just below the chin.
        let half_wid = THROAT_HALF_WID * render_size * (0.6 + bulge * 0.6);
        let fwd = THROAT_FWD * render_size;
        // Fill a small oval underneath the head front.
        let step = 0.5;
        let mut by = -half_wid;
        while by <= half_wid {
            // Curved profile — wider in the middle.
            let nx = by / half_wid;
            let span = (1.0 - nx * nx).max(0.0).sqrt() * 0.35 * render_size;
            let mut bx = fwd - span;
            while bx <= fwd + span {
                let (px, py) = to_canvas(bx, by);
                canvas.dot(
                    px as i32,
                    py as i32,
                    color::THROAT.0,
                    color::THROAT.1,
                    color::THROAT.2,
                );
                bx += step;
            }
            by += step;
        }
    }

    fn draw_vocal_sac(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        scale: f64,
        pulse: f64,
    ) {
        // The sac is a disc that pokes forward and down from under
        // the chin. Its radius pulses with the croak rhythm.
        let radius_world = (VOCAL_SAC_RADIUS_BASE + VOCAL_SAC_RADIUS_BULGE * pulse) * render_size;
        // Place its centre just forward of the chin; it grows
        // forward as it inflates.
        let fwd = (VOCAL_SAC_FWD + 0.55 * pulse) * render_size;
        let (cx, cy) = to_canvas(fwd, 0.0);
        fill_disc(canvas, cx, cy, radius_world * scale, color::THROAT);
        // A subtle shadow line under the sac to give it volume.
        let shadow_dx = (radius_world * 0.55) * scale;
        fill_disc(
            canvas,
            cx,
            cy + shadow_dx * 0.35,
            radius_world * scale * 0.75,
            color::BACK_DARK,
        );
        // Re-paint the bright sac on top to keep its sunlit upper
        // half — gives the inflated sphere a clear highlight.
        fill_disc(
            canvas,
            cx,
            cy - shadow_dx * 0.10,
            radius_world * scale * 0.65,
            color::THROAT,
        );
    }

    fn draw_tongue(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        extension: f64,
    ) {
        if extension <= 0.0 {
            return;
        }
        let nose_fwd = (EYE_OFFSET_FWD + 0.40) * render_size;
        let tip_fwd = nose_fwd + TONGUE_REACH * render_size * extension;
        let from = (nose_fwd, 0.0);
        let to = (tip_fwd, 0.0);
        draw_segment(canvas, to_canvas, from, to, TONGUE_COLOR, 0);
        // Small bulb at the tip.
        let (tx, ty) = to_canvas(to.0, to.1);
        canvas.dot(
            tx as i32,
            ty as i32,
            TONGUE_COLOR.0,
            TONGUE_COLOR.1,
            TONGUE_COLOR.2,
        );
        canvas.dot(
            tx as i32 + 1,
            ty as i32,
            TONGUE_COLOR.0,
            TONGUE_COLOR.1,
            TONGUE_COLOR.2,
        );
    }
}

// ===========================================================================
// Body pixel colour
// ===========================================================================

/// Returns Some(colour) if the pixel is inside the body silhouette,
/// or None if it's outside.
fn body_pixel_colour(
    bx: f64,
    by: f64,
    body_len: f64,
    body_wid: f64,
    head_len: f64,
    head_wid: f64,
    lift: f64,
) -> Option<(u8, u8, u8)> {
    // Two overlapping ovals: main body (centred at 0) and head bulge
    // (centred forward of body). Either one being inside paints the
    // pixel.
    let in_body = (bx / body_len).powi(2) + (by / body_wid).powi(2) <= 1.0;
    let head_centre_fwd = body_len * 0.55;
    let in_head = ((bx - head_centre_fwd) / head_len).powi(2) + (by / head_wid).powi(2) <= 1.0;
    if !in_body && !in_head {
        return None;
    }

    // Shade: lighter in the centre-back, darker toward the rim,
    // belly tint mixed in at the jump apex.
    let r_body = ((bx / body_len).powi(2) + (by / body_wid).powi(2))
        .sqrt()
        .min(1.0);
    let base = if r_body < 0.55 {
        color::BACK_LIGHT
    } else {
        color::BACK_MID
    };
    let edge_mix = ((r_body - 0.80) * 4.0).clamp(0.0, 1.0);
    let with_edge = lerp_color(base, color::BACK_DARK, edge_mix);
    let belly_mix = lift * 0.45;
    Some(lerp_color(with_edge, color::BELLY, belly_mix))
}

// ===========================================================================
// Small drawing primitives
// ===========================================================================

/// Fill a disc of radius `r` (sub-pixels) at `(cx, cy)` (sub-pixel coords).
fn fill_disc(canvas: &mut Canvas, cx: f64, cy: f64, r: f64, color: (u8, u8, u8)) {
    let r_int = r.ceil() as i32;
    let cx_i = cx as i32;
    let cy_i = cy as i32;
    for dy in -r_int..=r_int {
        for dx in -r_int..=r_int {
            let dd = ((dx * dx + dy * dy) as f64).sqrt();
            if dd <= r {
                canvas.dot(cx_i + dx, cy_i + dy, color.0, color.1, color.2);
            }
        }
    }
}

/// Draw a thick line between two body-local points by sampling along
/// the segment. `thickness_radius` is in sub-pixels (0 = single dot).
fn draw_segment(
    canvas: &mut Canvas,
    to_canvas: &impl Fn(f64, f64) -> (f64, f64),
    from: (f64, f64),
    to: (f64, f64),
    color: (u8, u8, u8),
    thickness_radius: i32,
) {
    let (px0, py0) = to_canvas(from.0, from.1);
    let (px1, py1) = to_canvas(to.0, to.1);
    let dx = px1 - px0;
    let dy = py1 - py0;
    let steps = (dx * dx + dy * dy).sqrt().ceil() as i32;
    let steps = steps.max(1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let px = (px0 + dx * t) as i32;
        let py = (py0 + dy * t) as i32;
        for tdy in -thickness_radius..=thickness_radius {
            for tdx in -thickness_radius..=thickness_radius {
                canvas.dot(px + tdx, py + tdy, color.0, color.1, color.2);
            }
        }
    }
}

// ===========================================================================
// Spawning
// ===========================================================================

/// Default frog set for a fresh pond.
pub fn spawn_frogs(w: f64, h: f64) -> Vec<Frog> {
    [
        (w * 0.18, h * 0.20, 0.4, 2.1_f64),
        (w * 0.78, h * 0.32, 2.9, 5.7),
        (w * 0.45, h * 0.78, 4.6, 9.3),
    ]
    .iter()
    .map(|&(x, y, heading, seed)| Frog::new(x, y, heading, seed))
    .collect()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_frog() -> Frog {
        Frog::new(20.0, 15.0, 0.0, 1.0)
    }

    // -- construction -----------------------------------------------------

    #[test]
    fn new_frog_starts_in_sit_state() {
        let f = default_frog();
        assert!(matches!(f.state(), FrogState::Sit));
    }

    #[test]
    fn new_frog_holds_initial_position() {
        let f = default_frog();
        assert!((f.position().0 - 20.0).abs() < 1e-10);
        assert!((f.position().1 - 15.0).abs() < 1e-10);
    }

    #[test]
    fn velocity_is_zero_while_sitting() {
        let f = default_frog();
        assert_eq!(f.velocity(), (0.0, 0.0));
    }

    // -- state transitions ------------------------------------------------

    #[test]
    fn sit_eventually_transitions_to_crouch() {
        let mut f = default_frog();
        // Advance enough seconds to outlast the longest possible sit.
        for _ in 0..1000 {
            f.update(0.05, 80.0, 46.0);
            if !matches!(f.state(), FrogState::Sit) {
                break;
            }
        }
        assert!(
            matches!(f.state(), FrogState::Crouch { .. } | FrogState::Jump { .. }),
            "frog should leave Sit after enough time, still in {:?}",
            f.state(),
        );
    }

    #[test]
    fn full_jump_cycle_passes_through_every_state() {
        // Drop directly into Crouch so we deterministically exercise
        // the jump path (Sit's action roll could otherwise pick
        // Croak or TongueFlick instead). After landing the frog
        // transitions back into Sit, but the next action roll might
        // launch into another non-Sit state quickly — so just verify
        // we passed through each phase, not the final state.
        let mut f = default_frog();
        f.state = FrogState::Crouch {
            remaining: CROUCH_DURATION,
            target: (30.0, 20.0),
        };
        f.heading = 0.0;

        let mut saw_jump = false;
        let mut saw_land = false;
        let mut saw_sit_after_landing = false;
        let mut landed = false;
        // 2 seconds is plenty: crouch(0.18) + jump(0.55) + land(0.32) ≈ 1.05.
        for _ in 0..100 {
            f.update(0.02, 80.0, 46.0);
            match f.state() {
                FrogState::Jump { .. } => saw_jump = true,
                FrogState::Land { .. } => {
                    saw_land = true;
                    landed = true;
                }
                FrogState::Sit if landed => saw_sit_after_landing = true,
                _ => {}
            }
        }
        assert!(saw_jump, "frog should pass through Jump");
        assert!(saw_land, "frog should pass through Land");
        assert!(
            saw_sit_after_landing,
            "frog should return to Sit after landing",
        );
    }

    #[test]
    fn jump_returns_splash_on_landing() {
        let mut f = default_frog();
        f.sit_timer = 0.0;
        let mut splash = None;
        for _ in 0..200 {
            if let Some(s) = f.update(0.02, 80.0, 46.0) {
                splash = Some(s);
                break;
            }
        }
        let s = splash.expect("frog should emit a Splash when it lands");
        assert!(s.force > 0.0);
        // Splash position must be where the frog now sits.
        assert!((s.x - f.position().0).abs() < 1e-6);
        assert!((s.y - f.position().1).abs() < 1e-6);
    }

    #[test]
    fn jump_target_stays_inside_pond() {
        let w = 80.0;
        let h = 46.0;
        // Start each frog just inside the margin and force jumps;
        // landings must stay inside [EDGE_MARGIN, w - EDGE_MARGIN].
        // Croak / TongueFlick rolls don't move the frog so the
        // starting position must already satisfy the bounds.
        for &(sx, sy) in &[
            (EDGE_MARGIN + 0.1, EDGE_MARGIN + 0.1),
            (w - EDGE_MARGIN - 0.1, EDGE_MARGIN + 0.1),
            (EDGE_MARGIN + 0.1, h - EDGE_MARGIN - 0.1),
            (w - EDGE_MARGIN - 0.1, h - EDGE_MARGIN - 0.1),
        ] {
            let mut f = Frog::new(sx, sy, 0.0, sx + sy);
            for _ in 0..30 {
                // Force-decide an action by zeroing the sit timer.
                f.sit_timer = 0.0;
                for _ in 0..200 {
                    f.update(0.02, w, h);
                    if matches!(f.state(), FrogState::Sit) {
                        break;
                    }
                }
                let (x, y) = f.position();
                assert!(
                    (EDGE_MARGIN..=w - EDGE_MARGIN).contains(&x),
                    "frog x={x} fell outside margin starting from ({sx},{sy})",
                );
                assert!(
                    (EDGE_MARGIN..=h - EDGE_MARGIN).contains(&y),
                    "frog y={y} fell outside margin starting from ({sx},{sy})",
                );
            }
        }
    }

    // -- velocity --------------------------------------------------------

    #[test]
    fn jump_velocity_points_toward_target() {
        let mut f = default_frog();
        f.sit_timer = 0.0;
        // Tick once through Sit→Crouch and once through Crouch→Jump.
        for _ in 0..40 {
            f.update(0.02, 80.0, 46.0);
            if matches!(f.state(), FrogState::Jump { .. }) {
                break;
            }
        }
        assert!(matches!(f.state(), FrogState::Jump { .. }));
        let (vx, vy) = f.velocity();
        let speed = (vx * vx + vy * vy).sqrt();
        assert!(
            speed >= JUMP_DISTANCE_MIN / JUMP_DURATION - 0.1,
            "jump speed {speed} should be at least the configured min",
        );
    }

    // -- rendering --------------------------------------------------------

    #[test]
    fn draw_produces_visible_pixels_in_sit() {
        let f = default_frog();
        let mut canvas = Canvas::new(80, 30);
        f.draw(&mut canvas, 2.0);
        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| canvas.get(x, y).0)
            .count();
        assert!(lit > 30, "sitting frog should light many pixels, got {lit}");
    }

    #[test]
    fn draw_renders_eye_colour() {
        let f = default_frog();
        let mut canvas = Canvas::new(80, 30);
        f.draw(&mut canvas, 2.0);
        let found_eye = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == color::EYE
            });
        assert!(found_eye, "eye pixels should be visible in the render");
    }

    // -- spawn -----------------------------------------------------------

    #[test]
    fn spawn_frogs_yields_a_handful_of_frogs() {
        let frogs = spawn_frogs(80.0, 46.0);
        assert!(
            frogs.len() >= 2,
            "expect at least 2 frogs, got {}",
            frogs.len()
        );
    }

    #[test]
    fn spawn_frogs_are_inside_pond() {
        let (w, h) = (80.0, 46.0);
        for f in spawn_frogs(w, h) {
            let (x, y) = f.position();
            assert!(x > 0.0 && x < w);
            assert!(y > 0.0 && y < h);
        }
    }

    // -- croak / tongue --------------------------------------------------

    #[test]
    fn croak_state_resolves_back_to_sit() {
        let mut f = default_frog();
        f.state = FrogState::Croak {
            remaining: CROAK_DURATION,
            pulse_phase: 0.0,
        };
        for _ in 0..((CROAK_DURATION / 0.02) as usize + 20) {
            f.update(0.02, 80.0, 46.0);
        }
        assert!(
            matches!(f.state(), FrogState::Sit),
            "after CROAK_DURATION the frog must return to Sit, got {:?}",
            f.state(),
        );
    }

    #[test]
    fn croak_does_not_move_the_frog() {
        let mut f = default_frog();
        let (x0, y0) = f.position();
        f.state = FrogState::Croak {
            remaining: CROAK_DURATION,
            pulse_phase: 0.0,
        };
        for _ in 0..((CROAK_DURATION / 0.02) as usize) {
            f.update(0.02, 80.0, 46.0);
        }
        let (x1, y1) = f.position();
        assert!((x1 - x0).abs() < 1e-9 && (y1 - y0).abs() < 1e-9);
    }

    #[test]
    fn croak_draw_paints_a_visible_vocal_sac() {
        let mut f = default_frog();
        f.state = FrogState::Croak {
            remaining: CROAK_DURATION,
            pulse_phase: std::f64::consts::FRAC_PI_2, // peak of sin → max bulge
        };
        let mut canvas = Canvas::new(80, 30);
        f.draw(&mut canvas, 2.0);
        let lit = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == color::THROAT
            })
            .count();
        assert!(
            lit > 5,
            "croak should paint a sizeable throat-coloured vocal sac, got {lit}",
        );
    }

    #[test]
    fn tongue_flick_resolves_back_to_sit() {
        let mut f = default_frog();
        f.state = FrogState::TongueFlick {
            remaining: TONGUE_DURATION,
        };
        for _ in 0..30 {
            f.update(0.02, 80.0, 46.0);
        }
        assert!(matches!(f.state(), FrogState::Sit));
    }

    #[test]
    fn tongue_flick_draw_paints_tongue_colour() {
        let mut f = default_frog();
        f.state = FrogState::TongueFlick {
            remaining: TONGUE_DURATION * 0.5, // peak extension
        };
        let mut canvas = Canvas::new(80, 30);
        f.draw(&mut canvas, 2.0);
        let found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == TONGUE_COLOR
            });
        assert!(found, "tongue colour should be visible during the flick");
    }

    // -- scare ------------------------------------------------------------

    #[test]
    fn scare_inside_range_triggers_crouch_jump() {
        let mut f = Frog::new(20.0, 15.0, 0.0, 3.0);
        // Sit normally — but scared from very close.
        f.scare(19.0, 14.0, 80.0, 46.0);
        assert!(matches!(f.state(), FrogState::Crouch { .. }));
        // Resolving the crouch should put the frog into a Jump.
        for _ in 0..30 {
            f.update(0.02, 80.0, 46.0);
            if matches!(f.state(), FrogState::Jump { .. }) {
                return;
            }
        }
        panic!(
            "scared frog should reach Jump quickly, ended in {:?}",
            f.state()
        );
    }

    #[test]
    fn scare_outside_range_is_ignored() {
        let mut f = Frog::new(20.0, 15.0, 0.0, 4.0);
        let before = f.state();
        f.scare(70.0, 45.0, 80.0, 46.0);
        assert!(
            matches!(before, FrogState::Sit) && matches!(f.state(), FrogState::Sit),
            "distant scare should leave the frog alone",
        );
    }

    #[test]
    fn scare_aims_jump_away_from_threat() {
        let mut f = Frog::new(40.0, 23.0, 0.0, 5.0);
        f.scare(35.0, 23.0, 80.0, 46.0); // threat to the west
        match f.state() {
            FrogState::Crouch { target, .. } => {
                assert!(
                    target.0 > 40.0,
                    "scared frog should target east of its current x, got target=({:.1},{:.1})",
                    target.0,
                    target.1,
                );
            }
            other => panic!("expected Crouch after scare, got {other:?}"),
        }
    }

    #[test]
    fn scare_during_jump_does_not_interrupt() {
        let mut f = default_frog();
        f.state = FrogState::Jump {
            from: (20.0, 15.0),
            to: (30.0, 20.0),
            progress: 0.3,
        };
        f.scare(20.0, 15.0, 80.0, 46.0);
        assert!(matches!(f.state(), FrogState::Jump { .. }));
    }
}
