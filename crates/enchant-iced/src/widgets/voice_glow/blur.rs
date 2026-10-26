//! Separable gaussian blur on small float planes.
//!
//! CSS `blur(r)` and the canvas `blur(r)` filter are gaussians of standard
//! deviation `r`. Browsers approximate large radii with three box passes
//! (the SVG filter-effects spec recommends exactly that for `sigma >= 2`);
//! so do we, with running sums so the cost does not grow with the radius.
//! Small radii use the exact kernel. Everything outside the plane is
//! transparent.

/// Below this sigma (in samples) the exact kernel is used.
const BOX_FROM: f32 = 2.0;

/// The normalised gaussian kernel of standard deviation `sigma`, truncated at
/// three sigma.
fn kernel(sigma: f32) -> Vec<f32> {
    let radius = (sigma * 3.0).ceil().max(1.0) as usize;
    let denom = 2.0 * sigma * sigma;
    let mut k: Vec<f32> = (0..=2 * radius)
        .map(|i| {
            let d = i as f32 - radius as f32;
            (-(d * d) / denom).exp()
        })
        .collect();
    let sum: f32 = k.iter().sum();
    for v in &mut k {
        *v /= sum;
    }
    k
}

/// Geometry of one pass over a plane: `lines` lines of `len` samples, `along`
/// apart inside a line and `across` apart between lines.
#[derive(Clone, Copy)]
struct Lines {
    len: usize,
    lines: usize,
    along: usize,
    across: usize,
}

fn convolve<const N: usize>(src: &[[f32; N]], dst: &mut [[f32; N]], g: Lines, k: &[f32]) {
    let radius = (k.len() / 2) as isize;
    for line in 0..g.lines {
        let base = line * g.across;
        for i in 0..g.len {
            let mut acc = [0.0_f32; N];
            let lo = (i as isize - radius).max(0);
            let hi = (i as isize + radius).min(g.len as isize - 1);
            for j in lo..=hi {
                let w = k[(j - i as isize + radius) as usize];
                let s = &src[base + j as usize * g.along];
                for c in 0..N {
                    acc[c] += s[c] * w;
                }
            }
            dst[base + i * g.along] = acc;
        }
    }
}

/// A moving-window mean of width `2 * radius + 1` over one buffer, zero
/// outside it.
fn box_1d<const N: usize>(src: &[[f32; N]], dst: &mut [[f32; N]], radius: usize) {
    let len = src.len();
    let inv = 1.0 / (2 * radius + 1) as f32;
    let mut acc = [0.0_f32; N];
    // Window for i = 0 covers 0..=radius.
    for s in &src[..=radius.min(len - 1)] {
        for c in 0..N {
            acc[c] += s[c];
        }
    }
    for i in 0..len {
        for c in 0..N {
            dst[i][c] = acc[c] * inv;
        }
        // Slide: add i + radius + 1, drop i - radius.
        if i + radius + 1 < len {
            let s = &src[i + radius + 1];
            for c in 0..N {
                acc[c] += s[c];
            }
        }
        if i >= radius {
            let s = &src[i - radius];
            for c in 0..N {
                acc[c] -= s[c];
            }
        }
    }
}

/// Three box passes over every line of a plane. Each line is zero-extended by
/// the full support of the three boxes first, so the intermediate passes keep
/// the mass that spills past the edge (and lets it flow back in): that is the
/// transparent-black ("decal") edge of a true gaussian, whereas cropping
/// after every pass loses it and darkens the rim.
fn box_axis<const N: usize>(plane: &mut [[f32; N]], g: Lines, radii: [usize; 3]) {
    let pad: usize = radii.iter().sum();
    let n = g.len + 2 * pad;
    let mut a = vec![[0.0_f32; N]; n];
    let mut b = vec![[0.0_f32; N]; n];
    for line in 0..g.lines {
        let base = line * g.across;
        for (i, slot) in a[pad..pad + g.len].iter_mut().enumerate() {
            *slot = plane[base + i * g.along];
        }
        box_1d(&a, &mut b, radii[0]);
        box_1d(&b, &mut a, radii[1]);
        box_1d(&a, &mut b, radii[2]);
        for (i, v) in b[pad..pad + g.len].iter().enumerate() {
            plane[base + i * g.along] = *v;
        }
        a[..pad].fill([0.0; N]);
        a[pad + g.len..].fill([0.0; N]);
    }
}
/// Radii of three box passes whose composition approximates a gaussian of
/// `sigma` ("boxes for gauss").
fn box_radii(sigma: f32) -> [usize; 3] {
    let n = 3.0_f32;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m = ((12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n)
        / (-4.0 * wl as f32 - 4.0))
        .round() as i32;
    std::array::from_fn(|i| {
        let w = if (i as i32) < m { wl } else { wu };
        ((w - 1) / 2).max(0) as usize
    })
}

fn blur_axis<const N: usize>(src: &mut [[f32; N]], tmp: &mut [[f32; N]], g: Lines, sigma: f32) {
    if sigma < BOX_FROM {
        convolve(src, tmp, g, &kernel(sigma));
        src.copy_from_slice(tmp);
        return;
    }
    box_axis(src, g, box_radii(sigma));
}

/// Blurs a `w x h` plane in place with standard deviations `(sx, sy)` in
/// samples. `tmp` is scratch.
pub(crate) fn blur<const N: usize>(
    plane: &mut [[f32; N]],
    w: usize,
    h: usize,
    sx: f32,
    sy: f32,
    tmp: &mut Vec<[f32; N]>,
) {
    if w == 0 || h == 0 {
        return;
    }
    tmp.clear();
    tmp.resize(w * h, [0.0; N]);
    if sx > 0.2 {
        let rows = Lines {
            len: w,
            lines: h,
            along: 1,
            across: w,
        };
        blur_axis(plane, tmp, rows, sx);
    }
    if sy > 0.2 {
        let cols = Lines {
            len: h,
            lines: w,
            along: w,
            across: 1,
        };
        blur_axis(plane, tmp, cols, sy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(w: usize, h: usize) -> Vec<[f32; 1]> {
        let mut plane = vec![[0.0_f32; 1]; w * h];
        plane[(h / 2) * w + w / 2] = [1.0];
        plane
    }

    #[test]
    fn exact_kernel_preserves_energy_and_is_symmetric() {
        let (w, h) = (41, 41);
        let mut plane = impulse(w, h);
        let mut tmp = Vec::new();
        blur(&mut plane, w, h, 1.5, 1.5, &mut tmp);
        let sum: f32 = plane.iter().map(|p| p[0]).sum();
        assert!((sum - 1.0).abs() < 1e-3, "sum {sum}");
        assert!(plane[20 * w + 20][0] > plane[20 * w + 22][0]);
        assert!((plane[20 * w + 18][0] - plane[20 * w + 22][0]).abs() < 1e-6);
    }

    #[test]
    fn box_passes_approximate_a_gaussian() {
        let (w, h) = (101, 1);
        let mut boxed = impulse(w, h);
        let mut tmp = Vec::new();
        blur(&mut boxed, w, h, 6.0, 0.0, &mut tmp);
        let k = kernel(6.0);
        let centre = w / 2;
        let radius = k.len() / 2;
        for d in 0..radius.min(18) {
            let want = k[radius + d];
            let got = boxed[centre + d][0];
            assert!((got - want).abs() < 0.004, "d={d}: {got} vs {want}");
        }
        let sum: f32 = boxed.iter().map(|p| p[0]).sum();
        assert!((sum - 1.0).abs() < 1e-3);
    }

    #[test]
    fn box_blur_keeps_the_transparent_black_edge_of_a_true_gaussian() {
        // A uniform plane: at the rim a decal gaussian keeps about half of
        // the mass (the exact kernel gives Phi(0.5 / sigma)); cropping the
        // intermediate box passes would drop it to roughly 0.4 and below.
        let (w, h) = (200, 1);
        let mut boxed = vec![[1.0_f32]; w];
        let mut tmp = Vec::new();
        blur(&mut boxed, w, h, 10.0, 0.0, &mut tmp);
        let k = kernel(10.0);
        let r = k.len() / 2;
        let want: f32 = (0..=r).map(|d| k[r + d]).sum();
        assert!(
            (boxed[0][0] - want).abs() < 0.01,
            "rim {} vs {want}",
            boxed[0][0]
        );
        assert!((boxed[100][0] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn tiny_sigma_is_a_no_op() {
        let mut plane = vec![[0.5_f32; 1]; 9];
        let mut tmp = Vec::new();
        blur(&mut plane, 3, 3, 0.1, 0.1, &mut tmp);
        assert!(plane.iter().all(|p| p[0] == 0.5));
    }
}
