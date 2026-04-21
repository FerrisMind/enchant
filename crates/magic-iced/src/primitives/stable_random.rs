#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StableRng {
    state: u64,
}

impl StableRng {
    pub fn new(seed: u64) -> Self {
        Self { state: mix(seed) }
    }

    pub fn fork(self, salt: u64) -> Self {
        Self::new(self.state ^ mix(salt.wrapping_add(0x9E37_79B9_7F4A_7C15)))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = mix(self.state.wrapping_add(0x9E37_79B9_7F4A_7C15));
        self.state
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    pub fn next_f32(&mut self) -> f32 {
        let value = self.next_u32() as f64 / u32::MAX as f64;
        value as f32
    }

    pub fn bool(&mut self, threshold: f32) -> bool {
        self.next_f32() < threshold.clamp(0.0, 1.0)
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        if !min.is_finite() || !max.is_finite() {
            return min;
        }

        if (max - min).abs() <= f32::EPSILON {
            return min;
        }

        min + self.next_f32() * (max - min)
    }

    pub fn range_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }

        let span = max - min;
        min + (self.next_u64() as usize % span)
    }
}

pub fn stable_noise(seed: u64, index: u64) -> f32 {
    let value = mix(seed ^ mix(index));
    (value as f64 / u64::MAX as f64) as f32
}

fn mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_rng_is_repeatable_for_same_seed() {
        let mut left = StableRng::new(42);
        let mut right = StableRng::new(42);

        assert_eq!(left.next_u64(), right.next_u64());
        assert_eq!(left.next_u64(), right.next_u64());
        assert_eq!(left.next_u32(), right.next_u32());
    }

    #[test]
    fn fork_creates_independent_but_repeatable_streams() {
        let base = StableRng::new(7);
        let mut left = base.fork(1);
        let mut right = base.fork(1);
        let mut other = base.fork(2);

        assert_eq!(left.next_u64(), right.next_u64());
        assert_ne!(left.next_u64(), other.next_u64());
    }

    #[test]
    fn stable_noise_is_bounded_and_repeatable() {
        let first = stable_noise(99, 1234);
        let second = stable_noise(99, 1234);

        assert_eq!(first, second);
        assert!((0.0..=1.0).contains(&first));
    }
}
