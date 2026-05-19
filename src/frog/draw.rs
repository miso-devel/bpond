//! Frog rendering — body silhouette, legs, eyes, throat, vocal sac,
//! tongue, and the submerged-tint pass for in-water postures.
//!
//! Everything here is purely visual. The parent module owns the
//! simulation; this module turns a frog's current state into pixels
//! on a [`crate::canvas::Canvas`].

use super::*;
use crate::canvas::Canvas;

// ===========================================================================
// Visual constants
// ===========================================================================

// Body geometry: two overlapping ovals — wide shoulder up front,
// tapered rear behind. Coordinates are in world units in the frog's
// local frame (forward, side).
const REAR_HALF_LEN: f64 = 2.25;
const REAR_HALF_WID: f64 = 1.15;
const REAR_CENTRE_FWD: f64 = -0.40;
const SHOULDER_HALF_LEN: f64 = 1.65;
const SHOULDER_HALF_WID: f64 = 1.55;
const SHOULDER_CENTRE_FWD: f64 = 0.95;

// Eyes sit on top of the shoulder bulge, poking sideways past the
// silhouette so they read like the prominent eye-bumps of a real frog.
const EYE_RADIUS: f64 = 0.70;
const EYE_OFFSET_FWD: f64 = 1.55;
const EYE_OFFSET_SIDE: f64 = 1.20;
const PUPIL_RADIUS: f64 = 0.22;

// Throat pulse bulge.
const THROAT_FWD: f64 = 1.20;
const THROAT_HALF_WID: f64 = 0.70;

// Hind-leg joints (sitting / folded Z shape).
const HIP_FWD: f64 = -1.20;
const HIP_SIDE: f64 = 1.05;
const KNEE_FWD: f64 = -3.10;
const KNEE_SIDE: f64 = 2.55;
const FOOT_FWD: f64 = 0.10;
const FOOT_SIDE: f64 = 2.55;
const LEG_THICKNESS: i32 = 1;

// Hind-leg joints during a jump (extended straight back).
const EXT_HIP_FWD: f64 = -1.20;
const EXT_HIP_SIDE: f64 = 0.70;
const EXT_FOOT_FWD: f64 = -5.40;
const EXT_FOOT_SIDE: f64 = 1.10;

// Front legs — short, peek out from under the shoulder while sitting.
const FRONT_HIP_FWD: f64 = 1.20;
const FRONT_HIP_SIDE: f64 = 0.85;
const FRONT_FOOT_FWD: f64 = 2.30;
const FRONT_FOOT_SIDE: f64 = 1.10;

/// Body grows by this fraction at the jump apex to read as vertical lift.
const JUMP_LIFT_SCALE: f64 = 0.55;

// Vocal sac visual (drawn during Croak).
const VOCAL_SAC_FWD: f64 = 1.10;
const VOCAL_SAC_RADIUS_BASE: f64 = 0.55;
const VOCAL_SAC_RADIUS_BULGE: f64 = 1.10;

// Tongue
const TONGUE_COLOR: (u8, u8, u8) = (220, 120, 130);
const TONGUE_REACH: f64 = 2.40; // world units forward of nose at peak

/// Cool tint applied to the frog body while it is in water. Kept
/// deliberately subtle: just enough that the swimming frog reads
/// slightly cooler than a pad-sitting frog, while preserving its
/// natural green so it still stands out against the water.
const SUBMERGED_TINT: (u8, u8, u8) = (40, 70, 110);
const SUBMERGED_MIX: f64 = 0.20;

mod color {
    /// Pale belly tint, mixed in at the jump apex when the frog is
    /// briefly between the viewer and the water.
    pub const BELLY: (u8, u8, u8) = (180, 200, 120);
    pub const EYE: (u8, u8, u8) = (235, 220, 80);
    pub const PUPIL: (u8, u8, u8) = (12, 12, 14);
    pub const THROAT: (u8, u8, u8) = (210, 220, 130);
}

// ===========================================================================
// Frog rendering
// ===========================================================================

impl super::Frog {
    pub fn draw(&self, canvas: &mut Canvas, scale: f64) {
        let (cx, cy) = self.position();
        let lift = self.jump_lift();
        let crouch = self.crouch_compression();

        // Body size scaling: bigger at the jump apex, slightly
        // smaller while compressed during crouch / land.
        let render_size = self.size * (1.0 + lift * JUMP_LIFT_SCALE - crouch * 0.10);

        // Stretch/squash on the same scheme.
        let stretch_fwd = 1.0 + lift * 0.15 - crouch * 0.12;
        let stretch_side = 1.0 - lift * 0.04 + crouch * 0.06;

        let cx_px = cx * scale;
        let cy_px = cy * scale;
        let cos_h = self.heading.cos();
        let sin_h = self.heading.sin();

        // Helper: body-local (forward, side) in world units → canvas
        // sub-pixel coordinates.
        let to_canvas = |bx_world: f64, by_world: f64| -> (f64, f64) {
            let wx_off = bx_world * cos_h - by_world * sin_h;
            let wy_off = bx_world * sin_h + by_world * cos_h;
            (cx_px + wx_off * scale, cy_px + wy_off * scale)
        };

        let submerged = self.in_water();
        self.draw_body(
            canvas,
            scale,
            render_size,
            stretch_fwd,
            stretch_side,
            lift,
            submerged,
        );

        // Hind-leg posture depends on what the frog is doing. While
        // in water the legs trail behind (Float) or actively stroke
        // (SwimKick); on land they fold into the classic Z unless
        // the frog is mid-jump.
        match self.state {
            FrogState::Jump { .. } => self.draw_extended_legs(canvas, &to_canvas, render_size),
            FrogState::SwimKick { stroke_phase, .. } => {
                self.draw_swim_legs(canvas, &to_canvas, render_size, stroke_phase)
            }
            FrogState::Float { .. } => self.draw_trailing_legs(canvas, &to_canvas, render_size),
            _ => self.draw_folded_legs(canvas, &to_canvas, render_size, crouch),
        }
        // Front legs / throat / tongue overlays only fire when the
        // frog is on dry land. In water they'd be under the surface.
        if !submerged {
            self.draw_front_legs(canvas, &to_canvas, render_size);
        }
        self.draw_eyes(canvas, &to_canvas, render_size, scale);
        if !submerged {
            match self.state {
                FrogState::Sit => {
                    let bulge = (self.breath_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    if bulge > 0.30 {
                        self.draw_throat(canvas, &to_canvas, render_size, bulge);
                    }
                }
                FrogState::Croak { pulse_phase, .. } => {
                    let pulse = (pulse_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                    self.draw_vocal_sac(canvas, &to_canvas, render_size, scale, pulse);
                }
                FrogState::TongueFlick { remaining } => {
                    let progress = 1.0 - (remaining / super::TONGUE_DURATION);
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
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_body(
        &self,
        canvas: &mut Canvas,
        scale: f64,
        render_size: f64,
        stretch_fwd: f64,
        stretch_side: f64,
        lift: f64,
        submerged: bool,
    ) {
        let rear = Oval {
            cx: REAR_CENTRE_FWD * render_size * stretch_fwd,
            half_len: REAR_HALF_LEN * render_size * stretch_fwd,
            half_wid: REAR_HALF_WID * render_size * stretch_side,
        };
        let shoulder = Oval {
            cx: SHOULDER_CENTRE_FWD * render_size * stretch_fwd,
            half_len: SHOULDER_HALF_LEN * render_size * stretch_fwd,
            half_wid: SHOULDER_HALF_WID * render_size * stretch_side,
        };

        // Bounding box covers both ovals plus a small margin.
        let max_x = (rear.cx + rear.half_len).max(shoulder.cx + shoulder.half_len);
        let min_x = (rear.cx - rear.half_len).min(shoulder.cx - shoulder.half_len);
        let max_y = rear.half_wid.max(shoulder.half_wid);
        let extent = max_x.max(-min_x).max(max_y) + 1.0;
        let bound = (extent * scale).ceil() as i32 + 2;

        let (cx, cy) = self.position();
        let cx_px = cx * scale;
        let cy_px = cy * scale;
        let cos_h = self.heading.cos();
        let sin_h = self.heading.sin();
        let cx_i = cx_px as i32;
        let cy_i = cy_px as i32;

        for dy in -bound..=bound {
            for dx in -bound..=bound {
                let world_dx = dx as f64 / scale;
                let world_dy = dy as f64 / scale;
                let bx = world_dx * cos_h + world_dy * sin_h;
                let by = -world_dx * sin_h + world_dy * cos_h;
                if let Some((r, g, b)) =
                    body_pixel_colour(bx, by, rear, shoulder, lift, self.morph, submerged)
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
        let dark = self.morph.back_dark();
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
            draw_segment(canvas, to_canvas, hip, knee, dark, LEG_THICKNESS);
            draw_segment(canvas, to_canvas, knee, foot, dark, LEG_THICKNESS);
        }
    }

    fn draw_extended_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        let dark = self.morph.back_dark();
        for &side in &[1.0_f64, -1.0] {
            let hip = (EXT_HIP_FWD * render_size, EXT_HIP_SIDE * side * render_size);
            let foot = (
                EXT_FOOT_FWD * render_size,
                EXT_FOOT_SIDE * side * render_size,
            );
            draw_segment(canvas, to_canvas, hip, foot, dark, LEG_THICKNESS);
        }
    }

    /// Relaxed posture for a floating frog: legs trail back roughly
    /// halfway between the folded Z and the fully extended jump
    /// pose. Used while the frog idles in water.
    fn draw_trailing_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        let splay = 0.55;
        let dark = self.morph.back_dark();
        for &side in &[1.0_f64, -1.0] {
            let hip = (EXT_HIP_FWD * render_size, EXT_HIP_SIDE * side * render_size);
            let knee = (
                (KNEE_FWD + (-1.0 - KNEE_FWD) * splay) * render_size,
                (KNEE_SIDE + (1.0 - KNEE_SIDE) * splay) * side * render_size,
            );
            let foot = (
                (FOOT_FWD + (EXT_FOOT_FWD - FOOT_FWD) * splay) * render_size,
                (FOOT_SIDE + (EXT_FOOT_SIDE - FOOT_SIDE) * splay) * side * render_size,
            );
            draw_segment(canvas, to_canvas, hip, knee, dark, LEG_THICKNESS);
            draw_segment(canvas, to_canvas, knee, foot, dark, LEG_THICKNESS);
        }
    }

    /// Breaststroke kick: legs sweep out and back in time with
    /// `stroke_phase`. Knee and foot positions interpolate between
    /// folded and extended.
    fn draw_swim_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        stroke_phase: f64,
    ) {
        let splay = (stroke_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let dark = self.morph.back_dark();
        for &side in &[1.0_f64, -1.0] {
            let hip = (HIP_FWD * render_size, HIP_SIDE * side * render_size);
            let knee = (
                (KNEE_FWD + (-1.0 - KNEE_FWD) * splay) * render_size,
                (KNEE_SIDE + (1.0 - KNEE_SIDE) * splay) * side * render_size,
            );
            let foot = (
                (FOOT_FWD + (EXT_FOOT_FWD - FOOT_FWD) * splay) * render_size,
                (FOOT_SIDE + (EXT_FOOT_SIDE - FOOT_SIDE) * splay) * side * render_size,
            );
            draw_segment(canvas, to_canvas, hip, knee, dark, LEG_THICKNESS);
            draw_segment(canvas, to_canvas, knee, foot, dark, LEG_THICKNESS);
        }
    }

    fn draw_front_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        let dark = self.morph.back_dark();
        for &side in &[1.0_f64, -1.0] {
            let hip = (
                FRONT_HIP_FWD * render_size,
                FRONT_HIP_SIDE * side * render_size,
            );
            let foot = (
                FRONT_FOOT_FWD * render_size,
                FRONT_FOOT_SIDE * side * render_size,
            );
            draw_segment(canvas, to_canvas, hip, foot, dark, 0);
        }
    }

    fn draw_eyes(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        scale: f64,
    ) {
        let r_px = EYE_RADIUS * render_size * scale;
        let pupil_r_px = PUPIL_RADIUS * render_size * scale;
        let blinking = self.is_blinking();
        for &side in &[1.0_f64, -1.0] {
            let (cx, cy) = to_canvas(
                EYE_OFFSET_FWD * render_size,
                EYE_OFFSET_SIDE * side * render_size,
            );
            if blinking {
                // Nictitating membrane: a short horizontal slit of
                // the morph's darkest back colour replaces the eye.
                let slit_half = r_px.max(1.5);
                let slit_y = cy as i32;
                let cx_i = cx as i32;
                let dark = self.morph.back_dark();
                for dx in -slit_half as i32..=slit_half as i32 {
                    canvas.dot(cx_i + dx, slit_y, dark.0, dark.1, dark.2);
                }
            } else {
                fill_disc(canvas, cx, cy, r_px, color::EYE);
                fill_disc(canvas, cx, cy, pupil_r_px, color::PUPIL);
            }
        }
    }

    fn draw_throat(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        bulge: f64,
    ) {
        let half_wid = THROAT_HALF_WID * render_size * (0.6 + bulge * 0.6);
        let fwd = THROAT_FWD * render_size;
        let step = 0.5;
        let mut by = -half_wid;
        while by <= half_wid {
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
        let radius_world = (VOCAL_SAC_RADIUS_BASE + VOCAL_SAC_RADIUS_BULGE * pulse) * render_size;
        let fwd = (VOCAL_SAC_FWD + 0.55 * pulse) * render_size;
        let (cx, cy) = to_canvas(fwd, 0.0);
        fill_disc(canvas, cx, cy, radius_world * scale, color::THROAT);
        // Soft shadow underneath for a sense of volume.
        let shadow_dx = (radius_world * 0.55) * scale;
        fill_disc(
            canvas,
            cx,
            cy + shadow_dx * 0.35,
            radius_world * scale * 0.75,
            self.morph.back_dark(),
        );
        // Re-paint the bright sac on top to keep a sunlit highlight.
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
// Body silhouette helpers
// ===========================================================================

/// An oval lobe used to compose the frog silhouette.
#[derive(Clone, Copy)]
struct Oval {
    cx: f64,
    half_len: f64,
    half_wid: f64,
}

impl Oval {
    fn contains(self, bx: f64, by: f64) -> bool {
        let nx = (bx - self.cx) / self.half_len;
        let ny = by / self.half_wid;
        nx * nx + ny * ny <= 1.0
    }
    fn distance_to_centre(self, bx: f64, by: f64) -> f64 {
        let nx = (bx - self.cx) / self.half_len;
        let ny = by / self.half_wid;
        (nx * nx + ny * ny).sqrt()
    }
}

/// Returns Some(colour) if the pixel is inside the body silhouette,
/// or None if it's outside.
#[allow(clippy::too_many_arguments)]
fn body_pixel_colour(
    bx: f64,
    by: f64,
    rear: Oval,
    shoulder: Oval,
    lift: f64,
    morph: Morph,
    submerged: bool,
) -> Option<(u8, u8, u8)> {
    let in_rear = rear.contains(bx, by);
    let in_shoulder = shoulder.contains(bx, by);
    if !in_rear && !in_shoulder {
        return None;
    }

    // Shade: lighter near the centre of whichever lobe owns this
    // pixel, darker toward the rim. Belly tint mixed in at the jump
    // apex when the underside briefly faces the camera.
    let r_lobe = if in_shoulder && in_rear {
        rear.distance_to_centre(bx, by)
            .min(shoulder.distance_to_centre(bx, by))
    } else if in_shoulder {
        shoulder.distance_to_centre(bx, by)
    } else {
        rear.distance_to_centre(bx, by)
    }
    .min(1.0);

    let base = if r_lobe < 0.55 {
        morph.back_light()
    } else {
        morph.back_mid()
    };
    let edge_mix = ((r_lobe - 0.80) * 4.0).clamp(0.0, 1.0);
    let with_edge = lerp_color(base, morph.back_dark(), edge_mix);
    let belly_mix = lift * 0.45;
    let surface = lerp_color(with_edge, color::BELLY, belly_mix);
    Some(if submerged {
        lerp_color(surface, SUBMERGED_TINT, SUBMERGED_MIX)
    } else {
        surface
    })
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
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::super::tests as parent_tests;
    use super::*;

    #[test]
    fn draw_produces_visible_pixels_in_sit() {
        let f = parent_tests::default_frog();
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
        let f = parent_tests::default_frog();
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

    #[test]
    fn croak_draw_paints_a_visible_vocal_sac() {
        let mut f = parent_tests::default_frog();
        f.state = FrogState::Croak {
            remaining: super::super::CROAK_DURATION,
            pulse_phase: std::f64::consts::FRAC_PI_2, // sin peak → full bulge
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
            "croak should paint a sizeable throat-coloured sac, got {lit}",
        );
    }

    #[test]
    fn tongue_flick_draw_paints_tongue_colour() {
        let mut f = parent_tests::default_frog();
        f.state = FrogState::TongueFlick {
            remaining: super::super::TONGUE_DURATION * 0.5, // peak extension
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

    #[test]
    fn blink_clears_iris_paint() {
        let mut f = parent_tests::default_frog();
        f.blink_remaining = super::super::BLINK_DURATION; // force a blink
        let mut canvas = Canvas::new(80, 30);
        f.draw(&mut canvas, 2.0);
        let iris_found = (0..canvas.w)
            .flat_map(|x| (0..canvas.h).map(move |y| (x, y)))
            .any(|(x, y)| {
                let (on, r, g, b) = canvas.get(x, y);
                on && (r, g, b) == color::EYE
            });
        assert!(!iris_found, "closed eye should not paint the iris colour");
    }
}
