//! Separable gaussian blur on small float planes.
//!
//! CSS `blur(r)` and the canvas `blur(r)` filter are gaussians of standard
//! deviation `r`. Browsers approximate large radii with three box passes
//! (the SVG filter-effects spec recommends exactly that for `sigma >= 2`);
//! so do we, with running sums so the cost does not grow with the radius.
//! Small radii use the exact kernel. Everything outside the plane is
//! transparent.
//!
//! Rows blur in parallel; the vertical pass transposes, blurs rows and
//! transposes back, which is the same arithmetic as blurring columns but lets
//! every worker touch contiguous memory. This is the proven `voice_glow`
//! implementation, optimised for the full-card layers.

use rayon::prelude::*;

use super::paint::Px;

/// Below this sigma (in samples) the exact kernel is used.
const BOX_FROM: f32 = 2.0;

/// Retained scratch for [`blur`]. Keep one next to the plane so the transpose
/// buffer is not reallocated per frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct BlurScratch {
    plane: Vec<Px>,
}

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

fn convolve_line(src: &[Px], dst: &mut [Px], k: &[f32]) {
    let radius = (k.len() / 2) as isize;
    for i in 0..src.len() {
        let mut acc = [0.0_f32; 4];
        let lo = (i as isize - radius).max(0);
        let hi = (i as isize + radius).min(src.len() as isize - 1);
        for j in lo..=hi {
            let w = k[(j - i as isize + radius) as usize];
            let s = src[j as usize];
            acc[0] += s[0] * w;
            acc[1] += s[1] * w;
            acc[2] += s[2] * w;
            acc[3] += s[3] * w;
        }
        dst[i] = acc;
    }
}

/// A moving-window mean of width `2 * radius + 1` over one buffer, zero
/// outside it. Premultiplied RGBA is unrolled so the four channels stay in
/// registers; the arithmetic order is unchanged.
#[inline]
fn box_1d(src: &[Px], dst: &mut [Px], radius: usize) {
    let len = src.len();
    let inv = 1.0 / (2 * radius + 1) as f32;
    let mut acc = [0.0_f32; 4];
    // Window for i = 0 covers 0..=radius.
    for s in &src[..=radius.min(len - 1)] {
        acc[0] += s[0];
        acc[1] += s[1];
        acc[2] += s[2];
        acc[3] += s[3];
    }
    for i in 0..len {
        dst[i] = [acc[0] * inv, acc[1] * inv, acc[2] * inv, acc[3] * inv];
        // Slide: add i + radius + 1, drop i - radius.
        if i + radius + 1 < len {
            let s = src[i + radius + 1];
            acc[0] += s[0];
            acc[1] += s[1];
            acc[2] += s[2];
            acc[3] += s[3];
        }
        if i >= radius {
            let s = src[i - radius];
            acc[0] -= s[0];
            acc[1] -= s[1];
            acc[2] -= s[2];
            acc[3] -= s[3];
        }
    }
}

/// Three box passes over one zero-padded line. The padding is only zeroed
/// before the first pass; later passes keep the mass that spilled past the
/// edge (and lets it flow back in), which is the transparent-black ("decal")
/// edge of a true gaussian.
fn box_line(row: &mut [Px], radii: [usize; 3], a: &mut Vec<Px>, b: &mut Vec<Px>) {
    let pad: usize = radii.iter().sum();
    let len = row.len();
    let n = len + 2 * pad;
    a.clear();
    a.resize(n, [0.0; 4]);
    b.clear();
    b.resize(n, [0.0; 4]);
    a[pad..pad + len].copy_from_slice(row);
    box_1d(a, b, radii[0]);
    box_1d(b, a, radii[1]);
    box_1d(a, b, radii[2]);
    row.copy_from_slice(&b[pad..pad + len]);
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

/// Blurs every row independently. `len` is the row length, `lines` the row
/// count, so `plane.len() == len * lines`.
fn blur_rows(plane: &mut [Px], len: usize, sigma: f32) {
    if sigma < BOX_FROM {
        let k = kernel(sigma);
        plane
            .par_chunks_mut(len)
            .for_each_init(Vec::new, |buf, row| {
                buf.clear();
                buf.resize(len, [0.0; 4]);
                convolve_line(row, buf, &k);
                row.copy_from_slice(buf);
            });
        return;
    }
    let radii = box_radii(sigma);
    plane.par_chunks_mut(len).for_each_init(
        || (Vec::new(), Vec::new()),
        |(a, b), row| {
            box_line(row, radii, a, b);
        },
    );
}

/// Transposes a `rows x cols` plane into a `cols x rows` one.
fn transpose(src: &[Px], dst: &mut [Px], rows: usize, cols: usize) {
    dst.par_chunks_mut(rows).enumerate().for_each(|(c, out)| {
        for (r, slot) in out.iter_mut().enumerate() {
            *slot = src[r * cols + c];
        }
    });
}

/// Blurs a `w x h` plane in place with standard deviations `(sx, sy)` in
/// samples.
pub(crate) fn blur(
    plane: &mut [Px],
    w: usize,
    h: usize,
    sx: f32,
    sy: f32,
    scratch: &mut BlurScratch,
) {
    if w == 0 || h == 0 {
        return;
    }
    if sx > 0.2 {
        blur_rows(plane, w, sx);
    }
    if sy > 0.2 {
        scratch.plane.clear();
        scratch.plane.resize(plane.len(), [0.0; 4]);
        transpose(plane, &mut scratch.plane, h, w);
        blur_rows(&mut scratch.plane, h, sy);
        transpose(&scratch.plane, plane, w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse(w: usize, h: usize) -> Vec<Px> {
        let mut plane = vec![[0.0_f32; 4]; w * h];
        plane[(h / 2) * w + w / 2] = [1.0, 0.0, 0.0, 0.0];
        plane
    }

    #[test]
    fn exact_kernel_preserves_energy_and_is_symmetric() {
        let (w, h) = (41, 41);
        let mut plane = impulse(w, h);
        blur(&mut plane, w, h, 1.5, 1.5, &mut BlurScratch::default());
        let sum: f32 = plane.iter().map(|p| p[0]).sum();
        assert!((sum - 1.0).abs() < 1e-3, "sum {sum}");
        assert!(plane[20 * w + 20][0] > plane[20 * w + 22][0]);
        assert!((plane[20 * w + 18][0] - plane[20 * w + 22][0]).abs() < 1e-6);
    }

    #[test]
    fn box_passes_approximate_a_gaussian() {
        let (w, h) = (101, 1);
        let mut boxed = impulse(w, h);
        blur(&mut boxed, w, h, 6.0, 0.0, &mut BlurScratch::default());
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
        let mut boxed = vec![[1.0_f32, 0.0, 0.0, 0.0]; w];
        blur(&mut boxed, w, h, 10.0, 0.0, &mut BlurScratch::default());
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
        let mut plane = vec![[0.5_f32, 0.0, 0.0, 0.0]; 9];
        blur(&mut plane, 3, 3, 0.1, 0.1, &mut BlurScratch::default());
        assert!(plane.iter().all(|p| p[0] == 0.5));
    }

    #[test]
    fn vertical_blur_matches_a_transposed_horizontal_blur() {
        let (w, h) = (37, 23);
        let mut vertical = impulse(w, h);
        let mut horizontal = impulse(w, h);
        blur(&mut vertical, w, h, 0.0, 4.0, &mut BlurScratch::default());
        // Equivalent: transpose, blur rows of the transpose, transpose back.
        let mut transposed = vec![[0.0_f32; 4]; w * h];
        for j in 0..h {
            for i in 0..w {
                transposed[i * h + j] = horizontal[j * w + i];
            }
        }
        blur(&mut transposed, h, w, 4.0, 0.0, &mut BlurScratch::default());
        for j in 0..h {
            for i in 0..w {
                horizontal[j * w + i] = transposed[i * h + j];
            }
        }
        for (a, b) in vertical.iter().zip(&horizontal) {
            assert!((a[0] - b[0]).abs() < 1e-6);
        }
    }
}
