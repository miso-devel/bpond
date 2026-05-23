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

mod draw;

use crate::rng::pseudo_rand;
use std::f64::consts::{PI, TAU};

// ===========================================================================
// Simulation tuning constants
// ===========================================================================
//
// Visual constants (body geometry, eye / leg / throat positions,
// colour tints, etc.) live in `src/frog/draw.rs`; only physics,
// timing and behavioural probabilities live here.

const SIT_DURATION_MIN: f64 = 3.0;
const SIT_DURATION_RANGE: f64 = 5.0; // → 3-8 seconds
const CROUCH_DURATION: f64 = 0.18;
const SCARED_CROUCH_DURATION: f64 = 0.07; // scared frogs barely pause
const JUMP_DURATION: f64 = 0.55;
const LAND_DURATION: f64 = 0.32;
const CROAK_DURATION: f64 = 1.8;
const CROAK_PULSE_RATE: f64 = 2.6; // Hz — about 4-5 pulses per croak
const TONGUE_DURATION: f64 = 0.18; // fast — real frogs are even faster

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
// Float idles are very brief — frogs in water swim almost
// continuously, with only short glides between kicks.
const FLOAT_DURATION_MIN: f64 = 0.35;
const FLOAT_DURATION_RANGE: f64 = 0.90; // 0.35–1.25 s between actions
const SWIM_KICK_DURATION: f64 = 0.55;
const SWIM_STROKE_RATE: f64 = 1.7; // strokes per second
const SWIM_PEAK_SPEED: f64 = 8.0; // peak forward speed during recovery
const FLOAT_DRIFT_AMP: f64 = 0.30; // tiny vertical bob amplitude (wu)
const FLOAT_GLIDE_SPEED: f64 = 0.65; // gentle drift while idling
const FLOAT_HEADING_WANDER: f64 = 0.45; // rad/s peak heading drift while idling

/// Heading change applied at the start of every SwimKick. The turn
/// is uniform in `[-SWIM_KICK_TURN_RANGE/2, +SWIM_KICK_TURN_RANGE/2]`,
/// so individual kicks swerve up to about ±0.5 rad (≈ 29°). Chained
/// kicks compound into a clearly curved path, not a straight line.
const SWIM_KICK_TURN_RANGE: f64 = 1.0;
/// Continuous heading wander during a SwimKick. Smaller than the
/// per-kick turn so paths still read as deliberate strokes, but
/// large enough that even a single kick doesn't trace a perfect
/// line.
const SWIM_HEADING_WANDER: f64 = 0.55; // rad/s peak

// Action probabilities at the end of a Float period. Strongly biased
// toward kicking — a real pond frog in water swims actively rather
// than holding still for long stretches.
//
// A frog in water never jumps randomly. The "pad-jump" slot below is
// only spent if there is a reachable lily pad to aim at; otherwise
// it falls back to one more SwimKick.
const FLOAT_ACTION_KICK_THRESHOLD: f64 = 0.78; // [0, 0.78) → SwimKick
const FLOAT_ACTION_REST_THRESHOLD: f64 = 0.85; // [0.78, 0.85) → another Float
                                               //                                                  [0.85, 1.00) → try a pad-jump

/// Probability that a kick chains directly into another kick instead
/// of resting in Float first. Gives a "two-kick burst" feel that
/// real swimming frogs exhibit.
const SWIM_CHAIN_PROBABILITY: f64 = 0.45;

// Lily-pad preference for jump targeting.
const PAD_PREFERENCE: f64 = 0.65; // chance to aim at a pad if one is reachable
const PAD_LAND_THRESHOLD: f64 = 0.80; // fraction of pad radius counted as "on the pad"

// Per-leap range
const JUMP_DISTANCE_MIN: f64 = 5.5;
const JUMP_DISTANCE_RANGE: f64 = 11.0;
const SCARED_JUMP_DISTANCE: f64 = 18.0; // big jump away from the threat

// Breathing (throat pulse)
const BREATH_RATE: f64 = 1.4;

// Per-frog size variation. Sized so even the largest frog comfortably
// fits inside the smallest lily pad (see RADIUS_MIN in lily.rs).
const SIZE_MIN: f64 = 1.15;
const SIZE_MAX: f64 = 1.40;

// Bounds margin so frogs don't spawn or land on the literal edge.
const EDGE_MARGIN: f64 = 3.0;

// Splash impulse the pond layer uses to spawn ripples on landing.
const LANDING_SPLASH_FORCE: f64 = 1.0;

// Scare reaction range: frogs farther than this from the threat
// ignore it. Inside this radius they immediately launch.
const SCARE_RANGE: f64 = 14.0;

// Action probabilities at the end of a Sit period.
const ACTION_TONGUE_THRESHOLD: f64 = 0.10; // [0, 0.10) → flick
const ACTION_CROAK_THRESHOLD: f64 = 0.35; // [0.10, 0.35) → croak
                                          //                                                   [0.35, 1.00) → jump

// Eye blink — frogs blink occasionally as the nictitating membrane
// sweeps across. Each pad has its own period seeded from `seed`.
const BLINK_INTERVAL_MIN: f64 = 3.5;
const BLINK_INTERVAL_RANGE: f64 = 5.0;
const BLINK_DURATION: f64 = 0.16;

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
            Morph::Green => (95, 200, 70),
            Morph::Olive => (130, 150, 60),
            Morph::Brown => (135, 100, 60),
        }
    }
    fn back_mid(self) -> (u8, u8, u8) {
        match self {
            Morph::Green => (55, 145, 45),
            Morph::Olive => (85, 110, 45),
            Morph::Brown => (95, 70, 40),
        }
    }
    fn back_dark(self) -> (u8, u8, u8) {
        match self {
            Morph::Green => (25, 90, 30),
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
    /// Index of the lily pad this frog is currently perched on, or
    /// `None` if it's in open water / airborne. While set, the frog
    /// is "glued" to that pad — every tick its position is synced
    /// to the pad's centre, so if the pad drifts the frog drifts
    /// with it. The field is the single source of truth for "is
    /// this frog on a pad?".
    on_pad: Option<usize>,
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
            on_pad: None,
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

    /// Index of the pad the frog is perched on, if any. Pond uses
    /// this directly to tint that exact pad — no distance check,
    /// no chance of drift desync.
    pub fn perched_pad(&self) -> Option<usize> {
        self.on_pad
    }

    /// True when the frog is currently in water (floating or kicking).
    fn in_water(&self) -> bool {
        matches!(
            self.state,
            FrogState::Float { .. } | FrogState::SwimKick { .. }
        )
    }

    /// Lateral velocity (world units per second). Non-zero only during
    /// Jump. Pond reads this to push lily pads via wake forces.
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
        // Stay glued to whichever pad we're perched on. If that pad
        // has vanished (extremely unlikely but defensive), drop into
        // Float so we don't keep claiming to sit on nothing.
        if let Some(idx) = self.on_pad {
            match pads.get(idx) {
                Some(&(px, py, _)) => {
                    self.x = px;
                    self.y = py;
                }
                None => {
                    self.on_pad = None;
                    self.state = FrogState::Float {
                        remaining: self.float_duration(),
                    };
                }
            }
        }
        let mut events: Vec<FrogEvent> = Vec::new();
        let next = match self.state {
            FrogState::Sit => self.tick_sit(dt, w, h, pads),
            FrogState::Crouch {
                mut remaining,
                target,
            } => {
                remaining -= dt;
                if remaining <= 0.0 {
                    // Leaving the pad — clear the glue.
                    self.on_pad = None;
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
                    self.on_pad = pad_at(to.0, to.1, pads);
                    events.push(FrogEvent::Splash {
                        x: to.0,
                        y: to.1,
                        force: if self.on_pad.is_some() {
                            0.5
                        } else {
                            LANDING_SPLASH_FORCE
                        },
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
                    if self.on_pad.is_some() {
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
                // Subtle ambient motion: tiny vertical bob, a very
                // slow forward glide, and a sine heading wander so
                // the frog never sits perfectly still in the water.
                let bob = self.breath_phase.sin() * FLOAT_DRIFT_AMP * dt;
                let glide_dx = self.heading.cos() * FLOAT_GLIDE_SPEED * dt;
                let glide_dy = self.heading.sin() * FLOAT_GLIDE_SPEED * dt;
                let wander = (self.breath_phase * 0.5).cos() * FLOAT_HEADING_WANDER * dt;
                self.heading += wander;
                self.x = (self.x + glide_dx).clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
                self.y = (self.y + glide_dy + bob)
                    .clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
                if remaining <= 0.0 {
                    let next = self.tick_float_action(w, h, pads);
                    // Pushing off the water surface throws a wake.
                    if matches!(next, FrogState::Crouch { .. }) {
                        events.push(FrogEvent::Wake {
                            x: self.x,
                            y: self.y,
                        });
                    }
                    next
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
                // Smooth heading wander during the stroke so even a
                // single uninterrupted kick traces a slight curve.
                let wander = (self.breath_phase * 0.7 + 0.3).sin() * SWIM_HEADING_WANDER * dt;
                self.heading += wander;
                let thrust = (-stroke_phase.cos()).max(0.0);
                let speed = SWIM_PEAK_SPEED * thrust;
                let drift_dx = self.heading.cos() * speed * dt;
                let drift_dy = self.heading.sin() * speed * dt;
                self.x = (self.x + drift_dx).clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
                self.y = (self.y + drift_dy).clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
                // Emit two wake ripples per kick — a larger one as
                // the legs sweep back together (stroke_phase passes
                // π), and a small trailing one when the kick winds
                // down. Both are spawned about a body-length behind
                // the frog.
                let trail = 1.6 * self.size;
                let behind_x = self.x - self.heading.cos() * trail;
                let behind_y = self.y - self.heading.sin() * trail;
                if prev_phase < PI && stroke_phase >= PI {
                    events.push(FrogEvent::Wake {
                        x: behind_x,
                        y: behind_y,
                    });
                }
                if remaining <= 0.0 {
                    events.push(FrogEvent::Wake {
                        x: behind_x,
                        y: behind_y,
                    });
                    // Roughly half the time chain another kick
                    // directly — real swimming frogs often fire a
                    // two- or three-kick burst before they coast.
                    if self.next_rand() < SWIM_CHAIN_PROBABILITY {
                        self.swim_kick()
                    } else {
                        FrogState::Float {
                            remaining: self.float_duration(),
                        }
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
            self.swim_kick()
        } else if roll < FLOAT_ACTION_REST_THRESHOLD {
            FrogState::Float {
                remaining: self.float_duration(),
            }
        } else {
            // A frog in water never jumps somewhere random — it only
            // leaps when it can clearly land on a pad. If no pad is
            // reachable, fall back to another kick.
            let max_dist = JUMP_DISTANCE_MIN + JUMP_DISTANCE_RANGE;
            match self.pick_pad_target(pads, max_dist) {
                Some((tx, ty)) => {
                    let tx = tx.clamp(EDGE_MARGIN, (w - EDGE_MARGIN).max(EDGE_MARGIN));
                    let ty = ty.clamp(EDGE_MARGIN, (h - EDGE_MARGIN).max(EDGE_MARGIN));
                    self.heading = (ty - self.y).atan2(tx - self.x);
                    FrogState::Crouch {
                        remaining: CROUCH_DURATION,
                        target: (tx, ty),
                    }
                }
                None => self.swim_kick(),
            }
        }
    }

    /// Start a fresh SwimKick. Each kick begins with a small random
    /// turn — that's what gives swimming frogs their curved,
    /// non-straight paths through the water.
    fn swim_kick(&mut self) -> FrogState {
        let turn = (self.next_rand() - 0.5) * SWIM_KICK_TURN_RANGE;
        self.heading += turn;
        FrogState::SwimKick {
            remaining: SWIM_KICK_DURATION,
            stroke_phase: 0.0,
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
}

// ===========================================================================
// Free helpers
// ===========================================================================

/// Index of the lily pad `(x, y)` is currently inside, if any.
/// "Inside" means within `PAD_LAND_THRESHOLD * radius` — the frog
/// has to land near the centre, not just clip the rim.
fn pad_at(x: f64, y: f64, pads: &[(f64, f64, f64)]) -> Option<usize> {
    pads.iter().position(|&(px, py, pr)| {
        let dx = x - px;
        let dy = y - py;
        (dx * dx + dy * dy).sqrt() < pr * PAD_LAND_THRESHOLD
    })
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
                let mut f = Frog::new(px, py, heading, seed);
                // Glue the spawn-on-pad frog to that pad so it
                // follows it from frame zero.
                f.on_pad = Some(i);
                f
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

    pub(super) fn default_frog() -> Frog {
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

    // (Rendering tests live in `src/frog/draw.rs::tests`.)

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

    // (Vocal-sac rendering test lives in `frog/draw.rs`.)

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

    // (Tongue-flick rendering test lives in `frog/draw.rs`.)

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
    fn floating_frog_never_jumps_when_no_pad_is_reachable() {
        // A water frog must never jump into the void. With an empty
        // pad list the action roll's "pad-jump" slot has to fall
        // back to a SwimKick.
        let mut f = default_frog();
        f.state = FrogState::Float { remaining: 0.0 };
        f.x = 40.0;
        f.y = 23.0;
        for _ in 0..400 {
            f.update(0.02, 80.0, 46.0, &[]);
            assert!(
                !matches!(
                    f.state(),
                    FrogState::Crouch { .. } | FrogState::Jump { .. } | FrogState::Land { .. }
                ),
                "frog in water with no pad in range must not jump, got {:?}",
                f.state(),
            );
        }
    }

    #[test]
    fn floating_frog_with_a_pad_in_range_eventually_aims_at_it() {
        // Pin the frog at a known position each tick (otherwise
        // SwimKick drift will eventually take it out of pad-jump
        // range). With the float countdown re-armed every frame and
        // the ~15% pad-jump slot, the frog should converge quickly.
        let mut f = default_frog();
        let pads = [(42.0, 30.0, 3.5)];
        f.state = FrogState::Float { remaining: 0.0 };
        for _ in 0..2000 {
            f.x = 30.0;
            f.y = 30.0;
            f.heading = 0.0;
            f.update(0.02, 80.0, 46.0, &pads);
            if let FrogState::Crouch { target, .. } = f.state() {
                assert!(
                    (target.0 - pads[0].0).abs() < 0.5 && (target.1 - pads[0].1).abs() < 0.5,
                    "crouch target {target:?} should match the pad centre {:?}",
                    (pads[0].0, pads[0].1),
                );
                return;
            }
            // Re-arm the float countdown so the action roll fires
            // again immediately (kick or pad-jump).
            if let FrogState::Float { remaining } = &mut f.state {
                if *remaining > 0.0 {
                    *remaining = 0.0;
                }
            }
        }
        panic!("frog should eventually jump toward the in-range pad");
    }

    #[test]
    fn swimming_frog_traces_a_curved_path() {
        // After many kicks the spread of headings the frog has held
        // should be wide — a curving path, not a straight line.
        let mut f = default_frog();
        f.x = 30.0;
        f.y = 23.0;
        f.heading = 0.0;
        let mut min_h = f.heading;
        let mut max_h = f.heading;
        let mut kicks = 0;
        let mut was_kicking = matches!(f.state(), FrogState::SwimKick { .. });
        f.state = FrogState::Float { remaining: 0.0 };
        for _ in 0..600 {
            f.update(0.02, 80.0, 46.0, &[]);
            let is_kicking = matches!(f.state(), FrogState::SwimKick { .. });
            if is_kicking && !was_kicking {
                kicks += 1;
            }
            was_kicking = is_kicking;
            min_h = min_h.min(f.heading);
            max_h = max_h.max(f.heading);
            // Re-arm Float so kicks keep firing.
            if let FrogState::Float { remaining } = &mut f.state {
                if *remaining > 0.0 {
                    *remaining = 0.0;
                }
            }
            if kicks >= 10 {
                break;
            }
        }
        let spread = max_h - min_h;
        assert!(
            kicks >= 5,
            "the test should observe several kicks, got {kicks}",
        );
        assert!(
            spread > 0.5,
            "a swimming frog should curve — heading spread was only {spread} rad over {kicks} kicks",
        );
    }

    #[test]
    fn perched_frog_follows_pad_drift() {
        // A frog with `on_pad` set should track the pad's position
        // each frame, so detection stays accurate even if the pad
        // drifts.
        let mut f = default_frog();
        f.x = 20.0;
        f.y = 15.0;
        f.on_pad = Some(0);
        // First tick: pad at the original position.
        f.update(0.02, 80.0, 46.0, &[(20.0, 15.0, 6.0)]);
        assert!((f.position().0 - 20.0).abs() < 0.001);
        assert!((f.position().1 - 15.0).abs() < 0.001);
        // Pad drifts north — frog should follow.
        f.update(0.02, 80.0, 46.0, &[(20.0, 11.5, 6.0)]);
        assert!(
            (f.position().1 - 11.5).abs() < 0.001,
            "frog should follow drifted pad, got y={}",
            f.position().1
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

    // (Closed-eye render test lives in `frog/draw.rs`.)
}
