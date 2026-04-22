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
        const UNIT_F32: f32 = 1.0 / 16_777_216.0;
        ((self.next_u32() >> 8) as f32) * UNIT_F32
    }

    pub fn bool(&mut self, threshold: f32) -> bool {
        if !threshold.is_finite() {
            return false;
        }

        let threshold = threshold.clamp(0.0, 1.0);
        if threshold <= 0.0 {
            return false;
        }
        if threshold >= 1.0 {
            return true;
        }

        self.next_f32() < threshold
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        if !min.is_finite() || !max.is_finite() {
            return min;
        }

        if max <= min {
            return min;
        }

        min + self.next_f32() * (max - min)
    }

    pub fn range_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }

        let span = (max - min) as u128;
        let zone = ((u64::MAX as u128 + 1) / span) * span;

        loop {
            let value = self.next_u64() as u128;
            if value < zone {
                return min + (value % span) as usize;
            }
        }
    }
}

pub fn stable_noise(seed: u64, index: u64) -> f32 {
    let value = mix(seed ^ mix(index));
    const UNIT_F32: f32 = 1.0 / 16_777_216.0;
    ((value >> 40) as u32 as f32) * UNIT_F32
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

    const NEXT_F32_EDGE_STATE: u64 = 0x86D9_D51B_B3DC_B845;
    const RANGE_USIZE_EDGE_STATE: u64 = 0x932B_113C_FBD6_B596;
    const STABLE_NOISE_EDGE_SEED: u64 = 0x3162_8AF6_7B21_31AB;

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
    fn next_f32_stays_below_one_even_on_highest_raw_sample() {
        let mut rng = StableRng {
            state: NEXT_F32_EDGE_STATE,
        };

        assert!(rng.next_f32() < 1.0);
    }

    #[test]
    fn range_f32_handles_zero_and_reversed_spans() {
        let mut rng = StableRng::new(99);

        assert_eq!(rng.range_f32(3.0, 3.0), 3.0);
        assert_eq!(rng.range_f32(5.0, 4.0), 5.0);
    }

    #[test]
    fn range_usize_rejects_top_of_range_bias() {
        let mut rng = StableRng {
            state: RANGE_USIZE_EDGE_STATE,
        };

        assert_eq!(rng.range_usize(0, 5), 4);
    }

    #[test]
    fn stable_noise_stays_below_one_even_on_highest_raw_sample() {
        let first = stable_noise(STABLE_NOISE_EDGE_SEED, 0);
        let second = stable_noise(STABLE_NOISE_EDGE_SEED, 0);

        assert_eq!(first, second);
        assert!(first < 1.0);
    }
}
