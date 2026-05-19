//! Pond frogs: sit, breathe, leap, and swim.
//!
//! The state machine carries two distinct rest postures so the frog
//! matches its surroundings:
//!
//! * On a lily pad — `Sit` uses the dry posture (folded Z hind
//!   legs, visible front legs, throat pulse). Off the timer, the
//!   frog rolls into `Crouch → Jump → Land`, or occasionally
//!   `Croak` (vocal sac inflation) or `TongueFlick`.
//! * In open water — `Float` shows a submerged-tinted body with
//!   hind legs trailing back (no front legs, no throat). Float
//!   cycles with `SwimKick` (one propulsive breaststroke kick that
//!   emits a wake ripple behind the frog), and Float occasionally
//!   rolls a jump that aims at a lily pad.
//!
//! Landing on a pad after a jump transitions to `Sit`; landing in
//! open water transitions to `Float`. The same crouch / jump / land
//! pipeline serves both surfaces.
//!
//! Rendering: a wide shoulder bulge + tapered rear oval (the frog
//! is broadest near the head, like real pond frogs from above) +
//! two large yellow eyes poking out sideways past the silhouette.
//! The visible body size grows at the jump apex to read as vertical
//! lift; the lift curve `(27/4) t (1-t)^2` peaks early to match a
//! real frog's explosive push-off. SwimKick drift is pulse-based:
//! velocity spikes during the recovery sweep, drops to near-zero
//! between strokes.

use crate::canvas::Canvas;
use crate::rng::pseudo_rand;
use std::f64::consts::{PI, TAU};

// ===========================================================================
// Tuning constants
// ===========================================================================

// Body geometry in world units (will be scaled by per-frog `size`).
//
// The silhouette is built from two overlapping ovals: a wide
// shoulder / head bulge up front and a tapered rear oval behind it.
// The shoulder oval is INTENTIONALLY wider than the rear so the
// frog reads with the broadest part near the head — matching the
// way pond frogs look from above.
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

// Hind-leg joints (sitting / folded Z shape). Negative `fwd` = behind
// body centre. The femur swings out wide and the tibia comes forward
// — the unmistakable bunched-up "Z" of a sitting frog.
const HIP_FWD: f64 = -1.20;
const HIP_SIDE: f64 = 1.05;
const KNEE_FWD: f64 = -3.10;
const KNEE_SIDE: f64 = 2.55;
const FOOT_FWD: f64 = 0.10;
const FOOT_SIDE: f64 = 2.55;
const LEG_THICKNESS: i32 = 1;

// Hind-leg joints during a jump (extended straight back). The toes
// reach about a body length behind the hip so the leap looks like a
// real hind-leg-driven launch.
const EXT_HIP_FWD: f64 = -1.20;
const EXT_HIP_SIDE: f64 = 0.70;
const EXT_FOOT_FWD: f64 = -5.40;
const EXT_FOOT_SIDE: f64 = 1.10;

// Front leg geometry — small, peeks out from under the shoulder and
// rests visibly forward of the chin when the frog is sitting.
const FRONT_HIP_FWD: f64 = 1.20;
const FRONT_HIP_SIDE: f64 = 0.85;
const FRONT_FOOT_FWD: f64 = 2.30;
const FRONT_FOOT_SIDE: f64 = 1.10;

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

// Water residency.
//
// A frog that lands off-pad enters `Float` — it idles in the water,
// body submerged, legs trailing. Float's timer eventually rolls one
// of three actions:
//
//   • SwimKick — one propulsive breaststroke kick; pulse-based
//     drift peaks during the recovery sweep, then the frog returns
//     to Float.
//   • Float — keep idling (reset the timer).
//   • Crouch → Jump — try to leave the water, usually onto a pad.
//
// Crouch / Jump from water reuse the existing jump pipeline.
const FLOAT_DURATION_MIN: f64 = 2.0;
const FLOAT_DURATION_RANGE: f64 = 4.0; // 2-6 seconds of idle floating
const SWIM_KICK_DURATION: f64 = 0.62; // one full stroke cycle
const SWIM_STROKE_RATE: f64 = 1.5; // strokes per second
const SWIM_PEAK_SPEED: f64 = 5.0; // peak forward speed during recovery
const FLOAT_DRIFT_AMP: f64 = 0.30; // tiny bob amplitude (wu) for idle Float

// Action probabilities at the end of a Float period.
const FLOAT_ACTION_KICK_THRESHOLD: f64 = 0.45; // [0, 0.45) → SwimKick
const FLOAT_ACTION_REST_THRESHOLD: f64 = 0.80; // [0.45, 0.80) → another Float
                                               //                                                  [0.80, 1.00) → Crouch + Jump

// Lily-pad preference for jump targeting.
const PAD_PREFERENCE: f64 = 0.65; // chance to aim at a pad if one is reachable
const PAD_LAND_THRESHOLD: f64 = 0.80; // fraction of pad radius counted as "on the pad"

// Per-leap range
const JUMP_DISTANCE_MIN: f64 = 5.5;
const JUMP_DISTANCE_RANGE: f64 = 11.0;
const SCARED_JUMP_DISTANCE: f64 = 18.0; // big jump away from the threat
const JUMP_LIFT_SCALE: f64 = 0.55; // body grows this fraction at apex

// Breathing (throat pulse)
const BREATH_RATE: f64 = 1.4;

// Per-frog size variation. Sized so even the largest frog comfortably
// fits inside the smallest lily pad (see RADIUS_MIN in lily.rs).
const SIZE_MIN: f64 = 1.15;
const SIZE_MAX: f64 = 1.40;

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

// Eye blink — frogs blink occasionally as the nictitating membrane
// sweeps across. Each pad has its own period seeded from `seed`.
const BLINK_INTERVAL_MIN: f64 = 3.5;
const BLINK_INTERVAL_RANGE: f64 = 5.0;
const BLINK_DURATION: f64 = 0.16;

// ===========================================================================
// Colours
// ===========================================================================

mod color {
    /// Pale belly tint, mixed in at the jump apex when the frog is
    /// briefly between the viewer and the water.
    pub const BELLY: (u8, u8, u8) = (180, 200, 120);
    pub const EYE: (u8, u8, u8) = (235, 220, 80);
    pub const PUPIL: (u8, u8, u8) = (12, 12, 14);
    pub const THROAT: (u8, u8, u8) = (210, 220, 130);
}

/// Colour morph picked per-frog at spawn so the pond hosts a mix of
/// looks rather than a herd of identical green frogs.
#[derive(Clone, Copy, Debug)]
pub enum Morph {
    /// Bright pond-frog green.
    Green,
    /// Olive / camo — common in green frogs and bullfrogs.
    Olive,
    /// Brown / wood-frog colouring.
    Brown,
}

impl Morph {
    fn back_light(self) -> (u8, u8, u8) {
        match self {
            Morph::Green => (95, 165, 70),
            Morph::Olive => (115, 130, 65),
            Morph::Brown => (135, 100, 60),
        }
    }
    fn back_mid(self) -> (u8, u8, u8) {
        match self {
            Morph::Green => (60, 120, 50),
            Morph::Olive => (75, 95, 45),
            Morph::Brown => (95, 70, 40),
        }
    }
    fn back_dark(self) -> (u8, u8, u8) {
        match self {
            Morph::Green => (30, 75, 30),
            Morph::Olive => (45, 60, 25),
            Morph::Brown => (60, 40, 22),
        }
    }
    fn pick(seed: f64) -> Self {
        let r = pseudo_rand(seed);
        if r < 0.55 {
            Morph::Green
        } else if r < 0.80 {
            Morph::Olive
        } else {
            Morph::Brown
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
// Events emitted by a frog tick
// ===========================================================================

/// Anything a single `Frog::update` tick can ask the pond to do.
/// Today: spawn ripples for splashes and swim wakes.
pub enum FrogEvent {
    /// The frog hit the water (or a pad). `force` is 1.0 for an
    /// open-water belly-flop and 0.5 for a pad landing.
    Splash { x: f64, y: f64, force: f64 },
    /// A swimming kick threw water back behind the frog. The
    /// caller should spawn one small ripple at `(x, y)`.
    Wake { x: f64, y: f64 },
}

// ===========================================================================
// State
// ===========================================================================

#[derive(Clone, Copy, Debug)]
pub enum FrogState {
    /// Resting on a lily pad. Dry posture (folded Z hind legs,
    /// visible front legs) and throat pulses with breath.
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
    /// Idling in open water — body submerged (drawn in a darker,
    /// water-tinted palette), hind legs trailing back relaxed. The
    /// `remaining` timer drives the next action roll.
    Float { remaining: f64 },
    /// One propulsive breaststroke kick. After it finishes the
    /// frog returns to `Float`.
    SwimKick { remaining: f64, stroke_phase: f64 },
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
    morph: Morph,
    seed: f64,
    rng_step: f64,
    /// Counts down until the next blink starts.
    blink_timer: f64,
    /// > 0 while the eye is mid-blink.
    blink_remaining: f64,
}

impl Frog {
    pub fn new(x: f64, y: f64, heading: f64, seed: f64) -> Self {
        let size = SIZE_MIN + pseudo_rand(seed) * (SIZE_MAX - SIZE_MIN);
        let morph = Morph::pick(seed + 7.0);
        Frog {
            x,
            y,
            heading,
            state: FrogState::Sit,
            sit_timer: SIT_DURATION_MIN + pseudo_rand(seed + 1.0) * SIT_DURATION_RANGE,
            breath_phase: pseudo_rand(seed + 2.0) * TAU,
            size,
            morph,
            seed,
            rng_step: 0.0,
            blink_timer: BLINK_INTERVAL_MIN + pseudo_rand(seed + 5.0) * BLINK_INTERVAL_RANGE,
            blink_remaining: 0.0,
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

    /// True when the frog is at rest on a solid surface — i.e.
    /// on a lily pad, not floating in water and not airborne.
    /// Pond uses this together with a position check to decide
    /// which pads to tint with the "occupied" palette.
    pub fn is_resting(&self) -> bool {
        matches!(
            self.state,
            FrogState::Sit
                | FrogState::Crouch { .. }
                | FrogState::Land { .. }
                | FrogState::Croak { .. }
                | FrogState::TongueFlick { .. }
        )
    }

    /// True when the frog is currently in water (floating or kicking).
    pub fn in_water(&self) -> bool {
        matches!(
            self.state,
            FrogState::Float { .. } | FrogState::SwimKick { .. }
        )
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
    ///
    /// The trajectory is asymmetric: a real frog jump rises quickly
    /// off the powerful hind-leg push, then falls back to the water
    /// under gravity. We approximate that with `27/4 * t * (1-t)^2`
    /// which peaks at exactly 1.0 at t = 1/3, then decays gently
    /// to 0 at t = 1.
    fn jump_lift(&self) -> f64 {
        match self.state {
            FrogState::Jump { progress, .. } => {
                let t = progress.clamp(0.0, 1.0);
                (27.0 / 4.0) * t * (1.0 - t).powi(2)
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

    /// Advance the frog one frame. Returns a list of side effects
    /// (splashes, wakes) the pond should turn into ripples.
    ///
    /// `pads` is an immutable snapshot list of `(x, y, radius)` for
    /// every lily pad in the pond. Frogs use it to bias their jump
    /// targets toward landing on a pad, and to decide whether a
    /// landing went into open water (→ Float) or onto a pad (→ Sit).
    pub fn update(&mut self, dt: f64, w: f64, h: f64, pads: &[(f64, f64, f64)]) -> Vec<FrogEvent> {
        self.breath_phase += dt * BREATH_RATE;
        self.tick_blink(dt);
        let mut events: Vec<FrogEvent> = Vec::new();
        let next = match self.state {
            FrogState::Sit => self.tick_sit(dt, w, h, pads),
            FrogState::Crouch {
                mut remaining,
                target,
            } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    FrogState::Jump {
                        from: (self.x, self.y),
                        to: target,
                        progress: 0.0,
                    }
                } else {
                    FrogState::Crouch { remaining, target }
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
                    let on_pad = landed_on_pad(to.0, to.1, pads);
                    events.push(FrogEvent::Splash {
                        x: to.0,
                        y: to.1,
                        force: if on_pad { 0.5 } else { LANDING_SPLASH_FORCE },
                    });
                    FrogState::Land {
                        remaining: LAND_DURATION,
                    }
                } else {
                    FrogState::Jump { from, to, progress }
                }
            }
            FrogState::Land { mut remaining } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    // Pad landings settle into the dry Sit posture
                    // immediately. Water landings drop the frog into
                    // Float so it idles with submerged posture and
                    // can choose to swim or jump out later.
                    if landed_on_pad(self.x, self.y, pads) {
                        self.reset_sit_timer();
                        FrogState::Sit
                    } else {
                        FrogState::Float {
                            remaining: self.float_duration(),
                        }
                    }
                } else {
                    FrogState::Land { remaining }
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
                    FrogState::Sit
                } else {
                    FrogState::Croak {
                        remaining,
                        pulse_phase,
                    }
                }
            }
            FrogState::TongueFlick { mut remaining } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    self.reset_sit_timer();
                    FrogState::Sit
                } else {
                    FrogState::TongueFlick { remaining }
                }
            }
            FrogState::Float { mut remaining } => {
                remaining -= dt;
                // Tiny ambient bob so the frog isn't perfectly
                // glued to the water.
                let bob = self.breath_phase.sin() * FLOAT_DRIFT_AMP * dt;
                self.y = (self.y + bob).clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
                if remaining <= 0.0 {
                    self.tick_float_action(w, h, pads)
                } else {
                    FrogState::Float { remaining }
                }
            }
            FrogState::SwimKick {
                mut remaining,
                mut stroke_phase,
            } => {
                let prev_phase = stroke_phase;
                remaining -= dt;
                stroke_phase += dt * SWIM_STROKE_RATE * TAU;
                let thrust = (-stroke_phase.cos()).max(0.0);
                let speed = SWIM_PEAK_SPEED * thrust;
                let drift_dx = self.heading.cos() * speed * dt;
                let drift_dy = self.heading.sin() * speed * dt;
                self.x = (self.x + drift_dx).clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
                self.y = (self.y + drift_dy).clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
                // Emit one wake ripple per stroke as the legs sweep
                // back together (stroke_phase passes π). Spawn it
                // about one body-length behind the frog.
                if prev_phase < PI && stroke_phase >= PI {
                    let trail = 1.6 * self.size;
                    events.push(FrogEvent::Wake {
                        x: self.x - self.heading.cos() * trail,
                        y: self.y - self.heading.sin() * trail,
                    });
                }
                if remaining <= 0.0 {
                    FrogState::Float {
                        remaining: self.float_duration(),
                    }
                } else {
                    FrogState::SwimKick {
                        remaining,
                        stroke_phase,
                    }
                }
            }
        };
        self.state = next;
        events
    }

    fn float_duration(&mut self) -> f64 {
        FLOAT_DURATION_MIN + self.next_rand() * FLOAT_DURATION_RANGE
    }

    /// When the Float timer runs out, decide what to do next.
    fn tick_float_action(&mut self, w: f64, h: f64, pads: &[(f64, f64, f64)]) -> FrogState {
        let roll = self.next_rand();
        if roll < FLOAT_ACTION_KICK_THRESHOLD {
            FrogState::SwimKick {
                remaining: SWIM_KICK_DURATION,
                stroke_phase: 0.0,
            }
        } else if roll < FLOAT_ACTION_REST_THRESHOLD {
            FrogState::Float {
                remaining: self.float_duration(),
            }
        } else {
            let (tx, ty, hd) = self.choose_jump_target(w, h, pads);
            self.heading = hd;
            FrogState::Crouch {
                remaining: CROUCH_DURATION,
                target: (tx, ty),
            }
        }
    }

    fn reset_sit_timer(&mut self) {
        self.sit_timer = SIT_DURATION_MIN + self.next_rand() * SIT_DURATION_RANGE;
    }

    fn tick_blink(&mut self, dt: f64) {
        if self.blink_remaining > 0.0 {
            self.blink_remaining -= dt;
            if self.blink_remaining < 0.0 {
                self.blink_remaining = 0.0;
                self.blink_timer = BLINK_INTERVAL_MIN + self.next_rand() * BLINK_INTERVAL_RANGE;
            }
        } else {
            self.blink_timer -= dt;
            if self.blink_timer <= 0.0 {
                self.blink_remaining = BLINK_DURATION;
            }
        }
    }

    fn is_blinking(&self) -> bool {
        self.blink_remaining > 0.0
    }

    /// Choose what to do when the sit timer runs out. Most often
    /// the frog jumps; sometimes it croaks; occasionally it flicks
    /// its tongue.
    fn tick_sit(&mut self, dt: f64, w: f64, h: f64, pads: &[(f64, f64, f64)]) -> FrogState {
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
            let (tx, ty, hd) = self.choose_jump_target(w, h, pads);
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

    fn choose_jump_target(&mut self, w: f64, h: f64, pads: &[(f64, f64, f64)]) -> (f64, f64, f64) {
        let max_dist = JUMP_DISTANCE_MIN + JUMP_DISTANCE_RANGE;
        let prefer_pad = self.next_rand() < PAD_PREFERENCE;
        let target = if prefer_pad {
            self.pick_pad_target(pads, max_dist)
        } else {
            None
        };
        let (raw_tx, raw_ty) = target.unwrap_or_else(|| {
            // Random direction fallback.
            let drift_angle = self.next_rand() * TAU;
            let distance = JUMP_DISTANCE_MIN + self.next_rand() * JUMP_DISTANCE_RANGE;
            (
                self.x + drift_angle.cos() * distance,
                self.y + drift_angle.sin() * distance,
            )
        });
        let tx = raw_tx.clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
        let ty = raw_ty.clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
        let heading = (ty - self.y).atan2(tx - self.x);
        (tx, ty, heading)
    }

    /// Pick the closest pad within `max_dist` of the frog (but not
    /// the pad it's already sitting on) and return its centre. None
    /// if no pad qualifies.
    fn pick_pad_target(&self, pads: &[(f64, f64, f64)], max_dist: f64) -> Option<(f64, f64)> {
        let mut best: Option<(f64, (f64, f64))> = None;
        for &(px, py, pr) in pads {
            let dx = px - self.x;
            let dy = py - self.y;
            let dist = (dx * dx + dy * dy).sqrt();
            // Ignore pads we're already sitting on (close enough that
            // jumping to the centre wouldn't move us).
            if dist < pr * PAD_LAND_THRESHOLD {
                continue;
            }
            if dist > max_dist {
                continue;
            }
            let beat_best = match best {
                None => true,
                Some((b, _)) => dist < b,
            };
            if beat_best {
                best = Some((dist, (px, py)));
            }
        }
        best.map(|(_, p)| p)
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
        // Front legs only show when the frog is sitting on dry land
        // — in water they're tucked beneath the chest.
        if !submerged {
            self.draw_front_legs(canvas, &to_canvas, render_size);
        }
        self.draw_eyes(canvas, &to_canvas, render_size, scale);

        // Throat / vocal sac / tongue all assume an above-water
        // posture and only show on dry land.
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
                self.morph.back_dark(),
                LEG_THICKNESS,
            );
            draw_segment(
                canvas,
                to_canvas,
                knee,
                foot,
                self.morph.back_dark(),
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
                self.morph.back_dark(),
                LEG_THICKNESS,
            );
        }
    }

    /// Breaststroke kick: both legs synchronously sweep out and
    /// back in time with `stroke_phase` (which ticks at
    /// `SWIM_STROKE_RATE * TAU`). Knee and foot positions
    /// interpolate between folded and extended.
    /// Relaxed posture for a floating frog: legs trail back roughly
    /// halfway between the folded Z and the fully extended jump
    /// pose. Used while the frog idles in water.
    fn draw_trailing_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
    ) {
        // Interpolate halfway between EXT_* and the folded knee/foot
        // positions for a "relaxed, drifting" posture.
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

    fn draw_swim_legs(
        &self,
        canvas: &mut Canvas,
        to_canvas: &impl Fn(f64, f64) -> (f64, f64),
        render_size: f64,
        stroke_phase: f64,
    ) {
        // 0 = legs gathered to the body, 1 = legs splayed out behind.
        let splay = (stroke_phase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
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
            draw_segment(
                canvas,
                to_canvas,
                hip,
                knee,
                self.morph.back_dark(),
                LEG_THICKNESS,
            );
            draw_segment(
                canvas,
                to_canvas,
                knee,
                foot,
                self.morph.back_dark(),
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
            draw_segment(canvas, to_canvas, hip, foot, self.morph.back_dark(), 0);
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
        let blinking = self.is_blinking();
        for &side in &[1.0_f64, -1.0] {
            let (cx, cy) = to_canvas(
                EYE_OFFSET_FWD * render_size,
                EYE_OFFSET_SIDE * side * render_size,
            );
            if blinking {
                // Nictitating membrane: replace the eye with a short
                // horizontal slit of the back's darkest colour.
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
            self.morph.back_dark(),
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
// Free helpers
// ===========================================================================

/// True if `(x, y)` falls inside any of the lily pads listed in
/// `pads`. "Inside" means within `PAD_LAND_THRESHOLD * radius`, so
/// the frog has to land near the centre — not just clip the rim.
fn landed_on_pad(x: f64, y: f64, pads: &[(f64, f64, f64)]) -> bool {
    pads.iter().any(|&(px, py, pr)| {
        let dx = x - px;
        let dy = y - py;
        (dx * dx + dy * dy).sqrt() < pr * PAD_LAND_THRESHOLD
    })
}

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

/// Underwater body tint: pixels mix toward this when the frog is
/// floating or kicking, so the body reads as submerged below the
/// water's mirror surface.
const SUBMERGED_TINT: (u8, u8, u8) = (18, 32, 48);
const SUBMERGED_MIX: f64 = 0.62;

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
    // pixel, darker toward the rim. Belly tint mixed in at the
    // jump apex when the underside briefly faces the camera.
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

// ===========================================================================
// Spawning
// ===========================================================================

/// Default frog set for a fresh pond. We try to place each frog
/// directly on a different lily pad (so the very first frame shows
/// frogs at rest in their natural perches); any frog that runs out
/// of available pads falls back to a sensible spot in open water.
///
/// Seeds are picked so the trio rolls a mix of colour morphs
/// (green / olive / brown).
pub fn spawn_frogs(pads: &[(f64, f64, f64)]) -> Vec<Frog> {
    const STARTS: [(f64, f64); 3] = [(0.4, 1.7), (2.9, 5.2), (4.6, 3.1)];
    STARTS
        .iter()
        .enumerate()
        .map(|(i, &(heading, seed))| {
            if let Some(&(px, py, _)) = pads.get(i) {
                Frog::new(px, py, heading, seed)
            } else {
                // No pad to perch on — drop the frog in water near
                // the centre and let it start in Float.
                let x = 20.0 + i as f64 * 7.0;
                let y = 15.0 + i as f64 * 5.0;
                let mut f = Frog::new(x, y, heading, seed);
                f.state = FrogState::Float {
                    remaining: FLOAT_DURATION_MIN,
                };
                f
            }
        })
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
    fn sit_eventually_transitions_to_another_state() {
        let mut f = default_frog();
        // Advance enough seconds to outlast the longest possible sit.
        for _ in 0..1000 {
            f.update(0.05, 80.0, 46.0, &[]);
            if !matches!(f.state(), FrogState::Sit) {
                break;
            }
        }
        assert!(
            !matches!(f.state(), FrogState::Sit),
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

        // Place a pad right at the landing target so the post-land
        // transition is Land → Sit rather than Land → Swim.
        let pads = [(30.0, 20.0, 4.0)];

        let mut saw_jump = false;
        let mut saw_land = false;
        let mut saw_sit_after_landing = false;
        let mut landed = false;
        for _ in 0..100 {
            f.update(0.02, 80.0, 46.0, &pads);
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
            "frog should return to Sit after landing on a pad",
        );
    }

    #[test]
    fn jump_returns_splash_on_landing() {
        let mut f = default_frog();
        f.sit_timer = 0.0;
        let mut splash = None;
        for _ in 0..200 {
            for ev in f.update(0.02, 80.0, 46.0, &[]) {
                if let FrogEvent::Splash { x, y, force } = ev {
                    splash = Some((x, y, force));
                }
            }
            if splash.is_some() {
                break;
            }
        }
        let (sx, sy, force) = splash.expect("frog should emit a Splash when it lands");
        assert!(force > 0.0);
        // Splash position must be where the frog now sits.
        assert!((sx - f.position().0).abs() < 1e-6);
        assert!((sy - f.position().1).abs() < 1e-6);
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
                    f.update(0.02, w, h, &[]);
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
            f.update(0.02, 80.0, 46.0, &[]);
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

    fn dummy_pads() -> Vec<(f64, f64, f64)> {
        vec![(20.0, 15.0, 6.0), (60.0, 30.0, 5.5), (40.0, 35.0, 7.0)]
    }

    #[test]
    fn spawn_frogs_yields_a_handful_of_frogs() {
        let frogs = spawn_frogs(&dummy_pads());
        assert!(
            frogs.len() >= 2,
            "expect at least 2 frogs, got {}",
            frogs.len()
        );
    }

    #[test]
    fn spawn_frogs_perch_on_each_provided_pad() {
        let pads = dummy_pads();
        let frogs = spawn_frogs(&pads);
        for (frog, pad) in frogs.iter().zip(pads.iter()) {
            let (fx, fy) = frog.position();
            assert!((fx - pad.0).abs() < 1e-9);
            assert!((fy - pad.1).abs() < 1e-9);
            assert!(matches!(frog.state(), FrogState::Sit));
        }
    }

    #[test]
    fn spawn_frogs_with_no_pads_drops_them_into_float() {
        let frogs = spawn_frogs(&[]);
        assert!(!frogs.is_empty());
        for f in &frogs {
            assert!(matches!(f.state(), FrogState::Float { .. }));
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
            f.update(0.02, 80.0, 46.0, &[]);
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
            f.update(0.02, 80.0, 46.0, &[]);
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
            f.update(0.02, 80.0, 46.0, &[]);
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
            f.update(0.02, 80.0, 46.0, &[]);
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

    // -- pad preference + swim ----------------------------------------

    #[test]
    fn landing_on_pad_skips_swim() {
        let mut f = default_frog();
        f.state = FrogState::Crouch {
            remaining: CROUCH_DURATION,
            target: (50.0, 30.0),
        };
        f.heading = 0.0;
        let pads = [(50.0, 30.0, 5.5)];
        let mut saw_water = false;
        for _ in 0..120 {
            f.update(0.02, 80.0, 46.0, &pads);
            if matches!(
                f.state(),
                FrogState::Float { .. } | FrogState::SwimKick { .. }
            ) {
                saw_water = true;
            }
        }
        assert!(
            !saw_water,
            "landing on a pad should NOT enter Float or SwimKick",
        );
    }

    #[test]
    fn swim_kick_resolves_back_to_float() {
        let mut f = default_frog();
        f.state = FrogState::SwimKick {
            remaining: SWIM_KICK_DURATION,
            stroke_phase: 0.0,
        };
        for _ in 0..((SWIM_KICK_DURATION / 0.02) as usize + 10) {
            f.update(0.02, 80.0, 46.0, &[]);
        }
        assert!(
            matches!(f.state(), FrogState::Float { .. }),
            "after one kick the frog should idle in Float, got {:?}",
            f.state(),
        );
    }

    #[test]
    fn swim_kick_drifts_the_frog_forward() {
        let mut f = default_frog();
        f.heading = 0.0; // east
        let (x0, _) = f.position();
        f.state = FrogState::SwimKick {
            remaining: SWIM_KICK_DURATION,
            stroke_phase: 0.0,
        };
        for _ in 0..((SWIM_KICK_DURATION / 0.02) as usize) {
            f.update(0.02, 80.0, 46.0, &[]);
        }
        let (x1, _) = f.position();
        assert!(
            x1 > x0 + 0.5,
            "kicking frog should drift east; went from {x0:.2} to {x1:.2}",
        );
    }

    #[test]
    fn swim_kick_emits_a_wake_ripple() {
        let mut f = default_frog();
        f.heading = 0.0;
        f.state = FrogState::SwimKick {
            remaining: SWIM_KICK_DURATION,
            stroke_phase: 0.0,
        };
        let mut wake_seen = false;
        for _ in 0..((SWIM_KICK_DURATION / 0.02) as usize + 5) {
            for ev in f.update(0.02, 80.0, 46.0, &[]) {
                if let FrogEvent::Wake { x, .. } = ev {
                    // Wake is behind the frog (heading east → west of x).
                    assert!(
                        x < f.position().0,
                        "wake should appear behind the swimming frog",
                    );
                    wake_seen = true;
                }
            }
        }
        assert!(wake_seen, "one kick should emit at least one Wake event");
    }

    #[test]
    fn landing_in_water_enters_float() {
        let mut f = default_frog();
        f.state = FrogState::Crouch {
            remaining: CROUCH_DURATION,
            target: (50.0, 30.0),
        };
        f.heading = 0.0;
        for _ in 0..120 {
            f.update(0.02, 80.0, 46.0, &[]);
            if matches!(f.state(), FrogState::Float { .. }) {
                return;
            }
        }
        panic!(
            "frog should be floating after landing in open water, got {:?}",
            f.state()
        );
    }

    #[test]
    fn float_state_eventually_resolves() {
        let mut f = default_frog();
        f.state = FrogState::Float {
            remaining: 0.0, // force an immediate action roll
        };
        // After enough ticks the Float should have transitioned out
        // (to SwimKick, another Float, or Crouch).
        for _ in 0..20 {
            f.update(0.02, 80.0, 46.0, &[]);
        }
        // Just verify the state is one of the allowed water/action
        // states (Float again, SwimKick, Crouch) — never Sit.
        assert!(
            !matches!(f.state(), FrogState::Sit),
            "Float should not return to Sit, got {:?}",
            f.state(),
        );
    }

    #[test]
    fn jump_target_prefers_a_pad_when_one_is_in_range() {
        // Force every action roll to land in the "jump" bucket by
        // skipping the action threshold check — we call the
        // choose_jump_target helper directly.
        let pads = [(30.0, 23.0, 4.5)];
        let f = Frog::new(20.0, 23.0, 0.0, 12.0);
        // pick_pad_target should return the pad's centre (it's the
        // only one within range).
        let target = f.pick_pad_target(&pads, JUMP_DISTANCE_MIN + JUMP_DISTANCE_RANGE);
        assert_eq!(target, Some((30.0, 23.0)));
    }

    #[test]
    fn pick_pad_target_ignores_the_pad_we_are_already_sitting_on() {
        // Frog sits at (20, 23) and there's a pad right under it.
        // We should not pick that pad as a target.
        let pads = [(20.0, 23.0, 5.0), (35.0, 23.0, 4.0)];
        let f = Frog::new(20.0, 23.0, 0.0, 13.0);
        let target = f.pick_pad_target(&pads, JUMP_DISTANCE_MIN + JUMP_DISTANCE_RANGE);
        assert_eq!(target, Some((35.0, 23.0)));
    }

    // -- snapshot used by Pond for pad wake --------------------------

    #[test]
    fn velocity_is_zero_outside_of_jump_state() {
        let mut f = default_frog();
        assert_eq!(f.velocity(), (0.0, 0.0));
        f.state = FrogState::SwimKick {
            remaining: SWIM_KICK_DURATION,
            stroke_phase: 0.0,
        };
        assert_eq!(f.velocity(), (0.0, 0.0));
        f.state = FrogState::Croak {
            remaining: CROAK_DURATION,
            pulse_phase: 0.0,
        };
        assert_eq!(f.velocity(), (0.0, 0.0));
    }

    #[test]
    fn velocity_is_nonzero_during_jump() {
        let mut f = default_frog();
        f.state = FrogState::Jump {
            from: (20.0, 15.0),
            to: (35.0, 15.0),
            progress: 0.4,
        };
        let (vx, vy) = f.velocity();
        let speed = (vx * vx + vy * vy).sqrt();
        assert!(speed > 1.0);
    }

    // -- colour morph + blink --------------------------------------------

    #[test]
    fn morph_picker_covers_all_three_variants() {
        let mut seen_green = false;
        let mut seen_olive = false;
        let mut seen_brown = false;
        for i in 0..200 {
            match Morph::pick(i as f64 * 0.97 + 0.5) {
                Morph::Green => seen_green = true,
                Morph::Olive => seen_olive = true,
                Morph::Brown => seen_brown = true,
            }
        }
        assert!(seen_green && seen_olive && seen_brown);
    }

    #[test]
    fn spawn_frogs_eventually_produces_more_than_one_morph() {
        // Use Morph::pick directly across many seeds — the morph
        // picker itself is what we care about.
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let s = i as f64 * 0.7 + 0.5;
            match Morph::pick(s) {
                Morph::Green => seen.insert("green"),
                Morph::Olive => seen.insert("olive"),
                Morph::Brown => seen.insert("brown"),
            };
        }
        assert_eq!(seen.len(), 3, "morph picker must produce all 3 variants");
    }

    #[test]
    fn blink_eventually_closes_the_eye() {
        let mut f = default_frog();
        let mut saw_blink = false;
        // 10 seconds is more than the longest blink interval.
        for _ in 0..500 {
            f.update(0.02, 80.0, 46.0, &[]);
            if f.is_blinking() {
                saw_blink = true;
                break;
            }
        }
        assert!(saw_blink, "frog should blink at least once within 10s");
    }

    #[test]
    fn blink_clears_iris_paint() {
        let mut f = default_frog();
        f.blink_remaining = BLINK_DURATION; // force a blink right now
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
