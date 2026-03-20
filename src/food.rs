//! Food pellet dropped by mouse click.

const DECAY_RATE: f64 = 0.04;
const EAT_RATE: f64 = 0.3;

pub const EAT_RANGE_SQ: f64 = 6.0;

pub struct Food {
    pub x: f64,
    pub y: f64,
    pub remaining: f64,
}

impl Food {
    pub fn new(x: f64, y: f64) -> Self {
        Food {
            x,
            y,
            remaining: 1.0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.remaining > 0.0
    }

    pub fn tick(&mut self, dt: f64, being_eaten: bool) {
        if being_eaten {
            self.remaining -= dt * EAT_RATE;
        } else {
            self.remaining -= dt * DECAY_RATE;
        }
    }

    pub fn fade(&self) -> f64 {
        self.remaining.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_food_starts_full() {
        let f = Food::new(10.0, 20.0);
        assert_eq!(f.x, 10.0);
        assert_eq!(f.y, 20.0);
        assert_eq!(f.remaining, 1.0);
        assert!(f.is_alive());
    }

    #[test]
    fn natural_decay_is_slow() {
        let mut f = Food::new(0.0, 0.0);
        f.tick(1.0, false);
        assert!((f.remaining - (1.0 - DECAY_RATE)).abs() < 1e-10);
        assert!(f.is_alive());
    }

    #[test]
    fn eating_consumes_faster() {
        let mut f = Food::new(0.0, 0.0);
        f.tick(1.0, true);
        assert!((f.remaining - (1.0 - EAT_RATE)).abs() < 1e-10);
    }

    #[test]
    fn dies_when_remaining_reaches_zero() {
        let mut f = Food::new(0.0, 0.0);
        f.tick(10.0, true);
        assert!(!f.is_alive());
    }

    #[test]
    fn fade_is_clamped() {
        let f = Food::new(0.0, 0.0);
        assert_eq!(f.fade(), 1.0);

        let mut f2 = Food::new(0.0, 0.0);
        f2.remaining = -0.5;
        assert_eq!(f2.fade(), 0.0);
    }
}
