//! Pure math shared by every thinking-orb mode.
//!
//! Everything here runs in `f64`, like the JavaScript reference, so the
//! sine-hash noise and the lattice maths stay bit-compatible with the golden
//! vectors. Values drop to `f32` only at paint time.

use std::f64::consts::PI;

/// Transcendentals from `libm` (a port of fdlibm, the same routines
/// JavaScript engines use), so frames are bit-for-bit reproducible across
/// platforms and match the reference engine at lattice boundaries, where a
/// last-bit difference in `sin` flips which band a dot belongs to.
pub(crate) trait Fdlibm {
    fn fsin(self) -> f64;
    fn fcos(self) -> f64;
    fn fexp(self) -> f64;
    fn fpowf(self, y: f64) -> f64;
    fn fatan2(self, x: f64) -> f64;
}

impl Fdlibm for f64 {
    fn fsin(self) -> f64 {
        libm::sin(self)
    }
    fn fcos(self) -> f64 {
        libm::cos(self)
    }
    fn fexp(self) -> f64 {
        libm::exp(self)
    }
    fn fpowf(self, y: f64) -> f64 {
        libm::pow(self, y)
    }
    fn fatan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
}

pub(crate) fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}

pub(crate) fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Deterministic hash in `[0, 1)`.
pub(crate) fn hash_d(a: f64, b: f64) -> f64 {
    let h = (a * 12.9898 + b * 78.233).fsin() * 43758.5453;
    h - h.floor()
}

/// Value noise on a 2D lattice: smooth, deterministic, cheap.
pub(crate) fn vnoise(x: f64, y: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let mut fx = x - xi;
    let mut fy = y - yi;
    fx = fx * fx * (3.0 - 2.0 * fx);
    fy = fy * fy * (3.0 - 2.0 * fy);
    let a = hash_d(xi, yi);
    let b = hash_d(xi + 1.0, yi);
    let c = hash_d(xi, yi + 1.0);
    let d = hash_d(xi + 1.0, yi + 1.0);
    a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy
}

/// Stable directions on a unit sphere (Fibonacci lattice).
pub(crate) fn fib_dir(i: f64, n: f64) -> [f64; 3] {
    let golden = PI * (3.0 - 5.0_f64.sqrt());
    let y = 1.0 - (2.0 * (i + 0.5)) / n;
    let rad = (1.0 - y * y).sqrt();
    let a = i * golden;
    [rad * a.fcos(), y, rad * a.fsin()]
}

/// Shortest signed angular distance, wrapped to `(-pi, pi]`.
pub(crate) fn angle_delta(a: f64, b: f64) -> f64 {
    (a - b).fsin().fatan2((a - b).fcos())
}

/// Dot radii were tuned for a 300pt frame; sub-linear scaling keeps small
/// spinners legible. A lower `pow` shrinks the dots less.
pub(crate) fn radius_scale(size: f64, pow: f64) -> f64 {
    (size / 300.0).fpowf(pow)
}

/// Spin, tilt and orthographic projection shared by every 3D mode.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Projector {
    st: f64,
    ct: f64,
    sy: f64,
    cyw: f64,
    cx: f64,
    cy: f64,
    scale: f64,
}

impl Projector {
    pub(crate) fn new(yaw: f64, tilt: f64, cx: f64, cy: f64, scale: f64) -> Self {
        Self {
            st: tilt.fsin(),
            ct: tilt.fcos(),
            sy: yaw.fsin(),
            cyw: yaw.fcos(),
            cx,
            cy,
            scale,
        }
    }

    /// Projects a unit-sphere point to `(x, y, depth)`.
    pub(crate) fn project(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let x1 = x * self.cyw + z * self.sy;
        let z1 = -x * self.sy + z * self.cyw;
        let y1 = y * self.ct - z1 * self.st;
        let z2 = y * self.st + z1 * self.ct;
        (self.cx + x1 * self.scale, self.cy - y1 * self.scale, z2)
    }
}

/// `0, 1, 2, ...` while `< n` (the JS `for (i = 0; i < n; i++)` loop).
pub(crate) fn below(n: f64) -> impl Iterator<Item = f64> {
    let mut i = 0.0;
    std::iter::from_fn(move || {
        (i < n).then(|| {
            let current = i;
            i += 1.0;
            current
        })
    })
}

/// `0, 1, 2, ...` while `<= n` (the JS `for (i = 0; i <= n; i++)` loop).
pub(crate) fn upto(n: f64) -> impl Iterator<Item = f64> {
    let mut i = 0.0;
    std::iter::from_fn(move || {
        (i <= n).then(|| {
            let current = i;
            i += 1.0;
            current
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_in_unit_interval_and_deterministic() {
        for i in 0..50 {
            let h = hash_d(f64::from(i), 1.7);
            assert!((0.0..1.0).contains(&h));
            assert_eq!(h, hash_d(f64::from(i), 1.7));
        }
    }

    #[test]
    fn fib_dirs_are_unit_vectors() {
        for i in 0..20 {
            let [x, y, z] = fib_dir(f64::from(i), 20.0);
            assert!((x * x + y * y + z * z - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn angle_delta_wraps() {
        assert!((angle_delta(0.1, 2.0 * PI - 0.1) - 0.2).abs() < 1e-12);
        assert!(angle_delta(PI, 0.0).abs() - PI < 1e-12);
    }

    #[test]
    fn loops_match_js_bounds() {
        assert_eq!(below(3.0).count(), 3);
        assert_eq!(upto(3.0).count(), 4);
        assert_eq!(below(0.0).count(), 0);
        assert_eq!(upto(2.5).count(), 3);
    }

    #[test]
    fn projector_centres_the_origin() {
        let p = Projector::new(0.4, 0.3, 10.0, 20.0, 5.0);
        let (x, y, z) = p.project(0.0, 0.0, 0.0);
        assert_eq!((x, y, z), (10.0, 20.0, 0.0));
    }

    #[test]
    fn vnoise_is_bounded() {
        for i in 0..40 {
            let v = vnoise(f64::from(i) * 0.37, 1.3);
            assert!((-0.01..=1.01).contains(&v));
        }
    }
}
