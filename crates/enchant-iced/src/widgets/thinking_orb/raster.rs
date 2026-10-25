//! Pixel work for the gravity sprite: bending the pointer toward the orb, the
//! masked, blurred tail, and the canvas-style compositing that stacks them.

pub(super) struct Region {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
}

pub(super) struct Mask {
    pub(super) x0: f64,
    pub(super) y0: f64,
    pub(super) dx: f64,
    pub(super) dy: f64,
    pub(super) len: f64,
}

pub(super) struct Bend<'a> {
    pub(super) src: &'a [u8],
    pub(super) sw: usize,
    pub(super) sh: usize,
    pub(super) ow: usize,
    pub(super) oh: usize,
    pub(super) b: f64,
    pub(super) taper: f64,
    pub(super) p: f64,
    pub(super) hx: f64,
    pub(super) hy: f64,
    pub(super) cxo: f64,
    pub(super) cyo: f64,
}

pub(super) fn blit(
    src: &[u8],
    sw: usize,
    sh: usize,
    out: &mut [u8],
    ow: usize,
    oh: usize,
    offset: i64,
) {
    for y in 0..sh {
        let oy = y as i64 + offset;
        if oy < 0 || oy >= oh as i64 {
            continue;
        }
        for x in 0..sw {
            let ox = x as i64 + offset;
            if ox < 0 || ox >= ow as i64 {
                continue;
            }
            let (i, j) = ((y * sw + x) * 4, (oy as usize * ow + ox as usize) * 4);
            out[j..j + 4].copy_from_slice(&src[i..i + 4]);
        }
    }
}

/// Bends the pointer toward the orb: each output pixel samples the sprite from
/// a point displaced away from the orb, by an amount that grows with the
/// source's distance from the tip, so the tip stays where it is and the body
/// leans in behind it. Nothing is rotated or scaled as a whole.
pub(super) fn bend_sprite(job: &Bend<'_>, out: &mut [u8]) {
    let Bend {
        src,
        sw,
        sh,
        ow,
        oh,
        b,
        taper,
        p,
        hx,
        hy,
        cxo,
        cyo,
    } = *job;
    out.fill(0);
    let big_l = (sw as f64).hypot(sh as f64).max(1.0);
    // Only the sprite's box, swept along the pull by up to the bend, can hold
    // a pixel; the rest of the padded raster stays clear.
    let (tdx, tdy) = (cxo - hx, cyo - hy);
    let tdl = {
        let l = tdx.hypot(tdy);
        if l == 0.0 { 1.0 } else { l }
    };
    let (tux, tuy) = (tdx / tdl, tdy / tdl);
    let x1 = (p + (b * tux).min(0.0) - 3.0).floor().max(0.0) as usize;
    let x2 = ((p + sw as f64 + (b * tux).max(0.0) + 3.0)
        .ceil()
        .min(ow as f64)) as usize;
    let y1 = (p + (b * tuy).min(0.0) - 3.0).floor().max(0.0) as usize;
    let y2 = ((p + sh as f64 + (b * tuy).max(0.0) + 3.0)
        .ceil()
        .min(oh as f64)) as usize;

    for y in y1..y2 {
        for x in x1..x2 {
            let i = (y * ow + x) * 4;
            let (xf, yf) = (x as f64, y as f64);
            // Where this pixel comes from: the source q with q + m(q)*u = (x, y),
            // m being the bend at q, at the source rather than here. Solved by a
            // damped fixed-point iteration (the plain one oscillates once the
            // bend's slope passes 1); a pixel with no solution stays empty.
            let (mut qx, mut qy) = (xf, yf);
            let (mut m, mut dx, mut dy, mut dl) = (0.0, 0.0, 0.0, 1.0);
            for _ in 0..7 {
                let s = (qx - hx).hypot(qy - hy) / big_l;
                m = b * s.min(1.0).powf(taper);
                dx = cxo - qx;
                dy = cyo - qy;
                dl = dx.hypot(dy);
                if dl == 0.0 {
                    dl = 1.0;
                }
                qx += (xf - (m * dx) / dl - qx) * 0.5;
                qy += (yf - (m * dy) / dl - qy) * 0.5;
            }
            let ex = qx + (m * dx) / dl - xf;
            let ey = qy + (m * dy) / dl - yf;
            if ex * ex + ey * ey > 2.25 {
                continue;
            }
            let (sx, sy) = (qx - p, qy - p);
            let (x0, y0) = (sx.floor() as i64, sy.floor() as i64);
            if x0 < -1 || y0 < -1 || x0 >= sw as i64 || y0 >= sh as i64 {
                continue;
            }
            let (fx, fy) = (sx - x0 as f64, sy - y0 as f64);
            let (mut r, mut g, mut bl, mut a) = (0.0, 0.0, 0.0, 0.0);
            for k in 0..4usize {
                let xx = x0 + (k & 1) as i64;
                let yy = y0 + (k >> 1) as i64;
                if xx < 0 || yy < 0 || xx >= sw as i64 || yy >= sh as i64 {
                    continue;
                }
                let wgt = (if k & 1 == 1 { fx } else { 1.0 - fx })
                    * (if k >> 1 == 1 { fy } else { 1.0 - fy });
                let j = (yy as usize * sw + xx as usize) * 4;
                let wa = wgt * f64::from(src[j + 3]);
                r += f64::from(src[j]) * wa;
                g += f64::from(src[j + 1]) * wa;
                bl += f64::from(src[j + 2]) * wa;
                a += wa;
            }
            if a > 0.0 {
                out[i] = (r / a).round().clamp(0.0, 255.0) as u8;
                out[i + 1] = (g / a).round().clamp(0.0, 255.0) as u8;
                out[i + 2] = (bl / a).round().clamp(0.0, 255.0) as u8;
                out[i + 3] = a.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

pub(super) fn premultiply(rgba: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.chunks_exact(4) {
        let a = f32::from(px[3]) / 255.0;
        out.push(f32::from(px[0]) / 255.0 * a);
        out.push(f32::from(px[1]) / 255.0 * a);
        out.push(f32::from(px[2]) / 255.0 * a);
        out.push(a);
    }
    out
}

pub(super) fn unpremultiply(buf: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len());
    for px in buf.chunks_exact(4) {
        let a = px[3];
        if a <= 0.0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let q = |c: f32| ((c / a).clamp(0.0, 1.0) * 255.0).round() as u8;
            out.extend_from_slice(&[
                q(px[0]),
                q(px[1]),
                q(px[2]),
                (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]);
        }
    }
    out
}

/// The bent pointer shifted by `(dx, dy)` raster px (bilinear), then keeping
/// only the part under the linear-gradient mask. Only source pixels inside the
/// region contribute, matching a region-clipped image draw.
pub(super) fn shifted_masked(
    bent: &[f32],
    ow: usize,
    region: &Region,
    dx: f64,
    dy: f64,
    mask: &Mask,
) -> Vec<f32> {
    let mut out = vec![0f32; region.w * region.h * 4];
    for ly in 0..region.h {
        for lx in 0..region.w {
            let (x, y) = (region.x + lx, region.y + ly);
            let m = (((x as f64 + 0.5 - mask.x0) * mask.dx + (y as f64 + 0.5 - mask.y0) * mask.dy)
                / mask.len)
                .clamp(0.0, 1.0) as f32;
            if m <= 0.0 {
                continue;
            }
            let (sx, sy) = (x as f64 - dx, y as f64 - dy);
            let (x0, y0) = (sx.floor(), sy.floor());
            let (fx, fy) = ((sx - x0) as f32, (sy - y0) as f32);
            let o = (ly * region.w + lx) * 4;
            for k in 0..4usize {
                let xx = x0 as i64 + (k & 1) as i64;
                let yy = y0 as i64 + (k >> 1) as i64;
                if xx < region.x as i64
                    || yy < region.y as i64
                    || xx >= (region.x + region.w) as i64
                    || yy >= (region.y + region.h) as i64
                {
                    continue;
                }
                let wgt = (if k & 1 == 1 { fx } else { 1.0 - fx })
                    * (if k >> 1 == 1 { fy } else { 1.0 - fy });
                let j = (yy as usize * ow + xx as usize) * 4;
                for c in 0..4 {
                    out[o + c] += bent[j + c] * wgt * m;
                }
            }
        }
    }
    out
}

/// Separable Gaussian blur of a premultiplied region; outside is transparent.
pub(super) fn gaussian_blur(buf: &mut [f32], w: usize, h: usize, sigma: f64) {
    let radius = (sigma * 3.0).ceil().max(1.0) as i64;
    let mut kernel: Vec<f32> = (-radius..=radius)
        .map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32)
        .collect();
    let sum: f32 = kernel.iter().sum();
    for k in &mut kernel {
        *k /= sum;
    }
    let mut tmp = vec![0f32; buf.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 4];
            for (ki, &kv) in kernel.iter().enumerate() {
                let sx = x as i64 + ki as i64 - radius;
                if sx < 0 || sx >= w as i64 {
                    continue;
                }
                let j = (y * w + sx as usize) * 4;
                for c in 0..4 {
                    acc[c] += buf[j + c] * kv;
                }
            }
            let o = (y * w + x) * 4;
            tmp[o..o + 4].copy_from_slice(&acc);
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 4];
            for (ki, &kv) in kernel.iter().enumerate() {
                let sy = y as i64 + ki as i64 - radius;
                if sy < 0 || sy >= h as i64 {
                    continue;
                }
                let j = (sy as usize * w + x) * 4;
                for c in 0..4 {
                    acc[c] += tmp[j + c] * kv;
                }
            }
            let o = (y * w + x) * 4;
            buf[o..o + 4].copy_from_slice(&acc);
        }
    }
}

/// Canvas `destination-over`: paints `stamp` (scaled by `alpha`) *under* what
/// is already in `acc`.
pub(super) fn under(acc: &mut [f32], ow: usize, region: &Region, stamp: &[f32], alpha: f32) {
    for ly in 0..region.h {
        for lx in 0..region.w {
            let o = (ly * region.w + lx) * 4;
            let j = ((region.y + ly) * ow + region.x + lx) * 4;
            let back = 1.0 - acc[j + 3];
            for c in 0..4 {
                acc[j + c] += stamp[o + c] * alpha * back;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bend_keeps_the_tip_and_moves_the_body_toward_the_orb() {
        let (sw, sh) = (12usize, 20usize);
        let pad = 48usize;
        let (ow, oh) = (sw + 2 * pad, sh + 2 * pad);
        let src = vec![255u8; sw * sh * 4];
        let mut out = vec![0u8; ow * oh * 4];
        let p = pad as f64;
        // orb far to the right of the tip
        bend_sprite(
            &Bend {
                src: &src,
                sw,
                sh,
                ow,
                oh,
                b: 10.0,
                taper: 2.0,
                p,
                hx: p,
                hy: p,
                cxo: p + 80.0,
                cyo: p,
            },
            &mut out,
        );
        let alpha = |x: usize, y: usize| out[(y * ow + x) * 4 + 3];
        // the tip pixel stays solid
        assert!(alpha(pad + 1, pad + 1) > 200);
        // the far end of the body has been dragged right: past the sprite's
        // right edge there is now ink
        let rightmost = (0..ow)
            .rev()
            .find(|&x| (0..oh).any(|y| alpha(x, y) > 0))
            .unwrap();
        assert!(
            rightmost > pad + sw,
            "body should lean right, got {rightmost}"
        );
    }

    #[test]
    fn blur_conserves_total_alpha() {
        let (w, h) = (21, 21);
        let mut buf = vec![0f32; w * h * 4];
        buf[(10 * w + 10) * 4 + 3] = 1.0;
        gaussian_blur(&mut buf, w, h, 1.5);
        let total: f32 = buf.chunks_exact(4).map(|p| p[3]).sum();
        assert!((total - 1.0).abs() < 1e-3);
    }

    #[test]
    fn under_paints_behind_existing_ink() {
        let region = Region {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };
        let mut acc = vec![0.0, 0.0, 0.0, 1.0];
        under(&mut acc, 1, &region, &[1.0, 1.0, 1.0, 1.0], 1.0);
        assert_eq!(acc, vec![0.0, 0.0, 0.0, 1.0]);
        let mut clear = vec![0.0; 4];
        under(&mut clear, 1, &region, &[1.0, 1.0, 1.0, 1.0], 0.5);
        assert_eq!(clear, vec![0.5, 0.5, 0.5, 0.5]);
    }

    #[test]
    fn premultiply_round_trips() {
        let px = [200u8, 100, 50, 128];
        let back = unpremultiply(&premultiply(&px));
        for (a, b) in px.iter().zip(&back) {
            assert!((i32::from(*a) - i32::from(*b)).abs() <= 1);
        }
    }
}
