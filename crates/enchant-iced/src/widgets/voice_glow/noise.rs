//! `feTurbulence type="fractalNoise"`, red channel: the displacement map of
//! the distortion filter.
//!
//! A port of the reference implementation in the SVG 1.1 filter-effects
//! specification (Ken Perlin's improved noise with the spec's seeded lattice).
//! The filter pins the map's green channel to 0.5, so only the red channel
//! (index 0) is needed; the other three channels' gradients are still drawn
//! from the generator, since they consume the same random sequence.

const B_SIZE: usize = 0x100;
const B_MASK: i64 = 0xff;
const PERLIN_N: f64 = 4096.0;
const RAND_M: i64 = 2_147_483_647;
const RAND_A: i64 = 16_807;
const RAND_Q: i64 = 127_773;
const RAND_R: i64 = 2_836;

fn random(seed: i64) -> i64 {
    let r = RAND_A * (seed % RAND_Q) - RAND_R * (seed / RAND_Q);
    if r <= 0 { r + RAND_M } else { r }
}

/// The seeded lattice of one turbulence filter.
#[derive(Clone, Debug)]
pub(crate) struct Turbulence {
    selector: [usize; B_SIZE + B_SIZE + 2],
    gradient: [[f64; 2]; B_SIZE + B_SIZE + 2],
}

impl Turbulence {
    /// The lattice for `seed` (the filter's `seed` attribute, 7 upstream).
    pub(crate) fn new(seed: i64) -> Self {
        let mut seed = if seed <= 0 {
            -(seed % (RAND_M - 1)) + 1
        } else {
            seed
        };
        if seed > RAND_M - 1 {
            seed = RAND_M - 1;
        }
        let mut selector = [0_usize; B_SIZE + B_SIZE + 2];
        let mut gradient = [[0.0_f64; 2]; B_SIZE + B_SIZE + 2];
        for channel in 0..4 {
            for (i, sel) in selector.iter_mut().take(B_SIZE).enumerate() {
                *sel = i;
                let mut g = [0.0_f64; 2];
                for v in &mut g {
                    seed = random(seed);
                    *v = (((seed % (2 * B_SIZE as i64)) - B_SIZE as i64) as f64) / B_SIZE as f64;
                }
                let len = (g[0] * g[0] + g[1] * g[1]).sqrt();
                if len > 0.0 {
                    g[0] /= len;
                    g[1] /= len;
                }
                if channel == 0 {
                    gradient[i] = g;
                }
            }
        }
        for i in (1..B_SIZE).rev() {
            let k = selector[i];
            seed = random(seed);
            let j = (seed % B_SIZE as i64) as usize;
            selector[i] = selector[j];
            selector[j] = k;
        }
        for i in 0..B_SIZE + 2 {
            selector[B_SIZE + i] = selector[i];
            gradient[B_SIZE + i] = gradient[i];
        }
        Self { selector, gradient }
    }

    fn noise2(&self, x: f64, y: f64) -> f64 {
        let t = x + PERLIN_N;
        let bx0 = (t as i64) & B_MASK;
        let bx1 = (bx0 + 1) & B_MASK;
        let rx0 = t - t.trunc();
        let rx1 = rx0 - 1.0;
        let t = y + PERLIN_N;
        let by0 = (t as i64) & B_MASK;
        let by1 = (by0 + 1) & B_MASK;
        let ry0 = t - t.trunc();
        let ry1 = ry0 - 1.0;

        let i = self.selector[bx0 as usize];
        let j = self.selector[bx1 as usize];
        let b00 = self.selector[i + by0 as usize];
        let b10 = self.selector[j + by0 as usize];
        let b01 = self.selector[i + by1 as usize];
        let b11 = self.selector[j + by1 as usize];

        let s_curve = |t: f64| t * t * (3.0 - 2.0 * t);
        let lerp = |t: f64, a: f64, b: f64| a + t * (b - a);
        let (sx, sy) = (s_curve(rx0), s_curve(ry0));

        let q = self.gradient[b00];
        let u = rx0 * q[0] + ry0 * q[1];
        let q = self.gradient[b10];
        let v = rx1 * q[0] + ry0 * q[1];
        let a = lerp(sx, u, v);
        let q = self.gradient[b01];
        let u = rx0 * q[0] + ry1 * q[1];
        let q = self.gradient[b11];
        let v = rx1 * q[0] + ry1 * q[1];
        let b = lerp(sx, u, v);
        lerp(sy, a, b)
    }

    /// The red channel of fractal noise at `(x, y)`, in `0..=1`: the sum of
    /// `octaves` octaves of `noise / 2^octave`, mapped from `-1..1`.
    pub(crate) fn fractal_red(&self, x: f64, y: f64, freq: (f64, f64), octaves: u32) -> f32 {
        let mut sum = 0.0;
        let (mut vx, mut vy) = (x * freq.0, y * freq.1);
        let mut ratio = 1.0;
        for _ in 0..octaves {
            sum += self.noise2(vx, vy) / ratio;
            vx *= 2.0;
            vy *= 2.0;
            ratio *= 2.0;
        }
        ((sum + 1.0) / 2.0).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic_bounded_and_varies() {
        let t = Turbulence::new(7);
        let a = t.fractal_red(10.3, 4.2, (0.012, 0.05), 2);
        assert_eq!(
            a,
            Turbulence::new(7).fractal_red(10.3, 4.2, (0.012, 0.05), 2)
        );
        let samples: Vec<f32> = (0..200)
            .map(|i| t.fractal_red(i as f64 * 1.7, i as f64 * 0.9, (0.012, 0.05), 2))
            .collect();
        assert!(samples.iter().all(|v| (0.0..=1.0).contains(v)));
        let (lo, hi) = samples
            .iter()
            .fold((1.0_f32, 0.0_f32), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        assert!(hi - lo > 0.1, "noise should vary, got {lo}..{hi}");
        // Different seeds give different fields.
        assert_ne!(
            a,
            Turbulence::new(8).fractal_red(10.3, 4.2, (0.012, 0.05), 2)
        );
    }

    #[test]
    fn noise_is_zero_mean_ish_at_lattice_points() {
        // Perlin noise is exactly 0 on integer lattice points -> 0.5 mapped.
        let t = Turbulence::new(7);
        let v = t.fractal_red(3.0, 5.0, (1.0, 1.0), 1);
        assert!((v - 0.5).abs() < 1e-6);
    }
}
