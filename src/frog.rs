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
const JUMP_DURATION: f64 = 0.55;
const LAND_DURATION: f64 = 0.32;

// Per-leap range
const JUMP_DISTANCE_MIN: f64 = 5.5;
const JUMP_DISTANCE_RANGE: f64 = 11.0;
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
            FrogState::Sit => {
                self.sit_timer -= dt;
                if self.sit_timer <= 0.0 {
                    let (tx, ty, hd) = self.choose_jump_target(w, h);
                    self.heading = hd;
                    (
                        FrogState::Crouch {
                            remaining: CROUCH_DURATION,
                            target: (tx, ty),
                        },
                        None,
                    )
                } else {
                    (FrogState::Sit, None)
                }
            }
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
                    self.sit_timer = SIT_DURATION_MIN + self.next_rand() * SIT_DURATION_RANGE;
                    (FrogState::Sit, None)
                } else {
                    (FrogState::Land { remaining }, None)
                }
            }
        };
        self.state = next;
        splash
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

        // Throat pulse only while sitting — too subtle to bother with
        // during the other states.
        if matches!(self.state, FrogState::Sit) {
            let bulge = (self.breath_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
            if bulge > 0.30 {
                self.draw_throat(canvas, &to_canvas, render_size, bulge);
            }
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
    fn full_cycle_passes_through_every_state() {
        // Force a transition by zeroing the sit timer then drive the
        // simulation long enough to observe each state in turn.
        let mut f = default_frog();
        f.sit_timer = 0.0;
        let mut saw_crouch = false;
        let mut saw_jump = false;
        let mut saw_land = false;
        let mut saw_sit_again = false;
        // ~3 seconds at 20 ms / tick. One full cycle is
        // crouch(0.18) + jump(0.55) + land(0.32) ≈ 1.05 s; the next
        // sit timer is 3–8 s, so we may or may not see a second cycle.
        let mut land_seen_tick = None;
        for i in 0..150 {
            f.update(0.02, 80.0, 46.0);
            match f.state() {
                FrogState::Crouch { .. } => saw_crouch = true,
                FrogState::Jump { .. } => saw_jump = true,
                FrogState::Land { .. } => {
                    saw_land = true;
                    land_seen_tick = Some(i);
                }
                FrogState::Sit => {
                    if land_seen_tick.is_some() {
                        saw_sit_again = true;
                    }
                }
            }
        }
        assert!(saw_crouch, "frog should enter Crouch");
        assert!(saw_jump, "frog should enter Jump");
        assert!(saw_land, "frog should enter Land");
        assert!(saw_sit_again, "frog should return to Sit after landing");
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
        // Start a frog near each corner and force jumps; landing
        // must stay inside [EDGE_MARGIN, w - EDGE_MARGIN].
        for &(sx, sy) in &[(2.0, 2.0), (78.0, 2.0), (2.0, 44.0), (78.0, 44.0)] {
            let mut f = Frog::new(sx, sy, 0.0, sx + sy);
            for _ in 0..30 {
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
}
