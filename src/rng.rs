//! Shared pseudo-random number generator used across pond simulation modules.

/// Deterministic pseudo-random value in [0, 1) derived from a seed.
pub(crate) fn pseudo_rand(seed: f64) -> f64 {
    ((seed * 127.1 + 311.7).sin() * 43758.5453).fract().abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pseudo_rand_output_in_unit_interval() {
        for i in 0..100 {
            let v = pseudo_rand(i as f64 * 1.7 + 0.3);
            assert!(v >= 0.0 && v < 1.0, "pseudo_rand({i}) = {v} not in [0, 1)");
        }
    }

    #[test]
    fn pseudo_rand_is_deterministic() {
        let a = pseudo_rand(42.0);
        let b = pseudo_rand(42.0);
        assert_eq!(a, b, "same seed must produce same value");
    }

    #[test]
    fn pseudo_rand_different_seeds_differ() {
        let a = pseudo_rand(1.0);
        let b = pseudo_rand(2.0);
        assert_ne!(
            a, b,
            "different seeds should generally produce different values"
        );
    }
}
