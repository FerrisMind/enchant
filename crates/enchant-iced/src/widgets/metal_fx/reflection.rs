//! Proximity reflection layout + CPU paint — iOS `MetalReflection` /
//! `engine/reflection/{constants,geometry,paint}.ts`.

use super::geometry::MetalFxVariant;
use super::material::{MetalMaterial, SheetMapping};
use super::reflection_anchor::MetalReflectionAnchor;
use super::reflection_occluder::{MetalReflectionOccluderConfig, reflection_occluder_config};

/// How a neighbour catches the metal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetalReflectionStyle {
    /// Chip / card: soft strip along the facing edge + hairline stroke.
    #[default]
    Surface,
    /// Text: mirrored sheet lands on letterforms only (caller supplies mask).
    Glyphs,
}

/// Canonical reflection constants (`constants.ts`).
pub mod constants {
    pub const RANGE_PX: f32 = 12.0;
    #[allow(dead_code)] // public API parity with constants.ts
    pub const ATTACH_RANGE_PX: f32 = 32.0;
    #[allow(dead_code)]
    pub const OVERLAP_MIN_PX: f32 = 1.0;
    pub const BASE_ALPHA: f32 = 0.55;
    pub const BOOST_ALPHA: f32 = 1.0;
    pub const INTENSITY_MULT: f32 = 1.3;
    pub const MAX_ALPHA_STACK: f32 = 3.6;
    pub const GLOBAL_ATTENUATION: f32 = 0.7;
    pub const STROKE_EXTRA_ALPHA: f32 = 0.52;
    pub const BORDER_HILITE_ALPHA: f32 = 0.044;
    pub const REF_DRAW_CSS_W: f32 = 235.0;
    pub const FILL_EXTRA_ALPHA: f32 = 2.535;
    pub const FILL_OPACITY_MUL: f32 = 0.7;
    pub const FILL_CIRCLE_ATTENUATION: f32 = 0.5;
    pub const FILL_BLUR_CSS_PX: f32 = 4.0;
}

use constants::*;

/// Resolved strip layout in the target's local box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalReflectionLayout {
    pub draw: [f32; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    /// Gradient start (unit, toward the facing edge).
    pub g0: [f32; 2],
    /// Gradient end (unit, fade into the target).
    pub g1: [f32; 2],
    pub reflection_alpha: f32,
    pub horizontal: bool,
}

/// Shortest gap between two AABBs `[x,y,w,h]`.
#[must_use]
pub fn shortest_rect_distance(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dx = (a[0] - (b[0] + b[2])).max(b[0] - (a[0] + a[2])).max(0.0);
    let dy = (a[1] - (b[1] + b[3])).max(b[1] - (a[1] + a[3])).max(0.0);
    dx.hypot(dy)
}

/// Layout of the mirrored catch for one target rect (absolute coords).
#[must_use]
pub fn reflection_layout(
    anchor: &MetalReflectionAnchor,
    target: [f32; 4],
    strength: f32,
    style: MetalReflectionStyle,
) -> Option<MetalReflectionLayout> {
    let a = anchor.rect();
    let (tw, th) = (target[2], target[3]);
    if a[2] < 4.0 || a[3] < 4.0 || tw < 1.0 || th < 1.0 {
        return None;
    }
    let amx = a[0] + a[2] * 0.5;
    let amy = a[1] + a[3] * 0.5;
    let tmx = target[0] + tw * 0.5;
    let tmy = target[1] + th * 0.5;
    let dx = amx - tmx;
    let dy = amy - tmy;
    let edge_gap_h = (a[0] - (target[0] + tw))
        .max(target[0] - (a[0] + a[2]))
        .max(0.0);
    let edge_gap_v = (a[1] - (target[1] + th))
        .max(target[1] - (a[1] + a[3]))
        .max(0.0);
    let horiz = edge_gap_h >= edge_gap_v;
    let dist = edge_gap_h.hypot(edge_gap_v);
    let mut proximity = 1.0 - (dist / RANGE_PX).min(1.0);
    proximity = proximity * proximity * (3.0 - 2.0 * proximity);
    let intensity = BASE_ALPHA + (BOOST_ALPHA - BASE_ALPHA) * proximity;
    let reflection_alpha =
        (intensity * INTENSITY_MULT * GLOBAL_ATTENUATION).min(MAX_ALPHA_STACK) * strength;

    let glyph = style == MetalReflectionStyle::Glyphs;
    let band = if glyph {
        RANGE_PX * 1.5
    } else {
        RANGE_PX
    }
    .min(tw.max(th));
    let ref_w = if glyph {
        (if horiz { tw } else { th })
            .min(if horiz { a[2] } else { a[3] })
            .max(1.0)
    } else {
        (REF_DRAW_CSS_W * (a[2] / 140.0).max(0.1)).max(1.0)
    };

    if horiz {
        let top = a[1].max(target[1]);
        let bot = (a[1] + a[3]).min(target[1] + th);
        let draw = [
            if dx > 0.0 { tw - ref_w } else { 0.0 },
            top - target[1],
            ref_w,
            (bot - top).max(1.0),
        ];
        Some(MetalReflectionLayout {
            draw,
            flip_x: true,
            flip_y: false,
            g0: [if dx > 0.0 { 1.0 } else { 0.0 }, 0.5],
            g1: [
                if dx > 0.0 {
                    1.0 - band / tw
                } else {
                    band / tw
                },
                0.5,
            ],
            reflection_alpha,
            horizontal: true,
        })
    } else {
        let left = a[0].max(target[0]);
        let right = (a[0] + a[2]).min(target[0] + tw);
        let draw = [
            left - target[0],
            if dy > 0.0 { th - ref_w } else { 0.0 },
            (right - left).max(1.0),
            ref_w,
        ];
        Some(MetalReflectionLayout {
            draw,
            flip_x: false,
            flip_y: true,
            g0: [0.5, if dy > 0.0 { 1.0 } else { 0.0 }],
            g1: [
                0.5,
                if dy > 0.0 {
                    1.0 - band / th
                } else {
                    band / th
                },
            ],
            reflection_alpha,
            horizontal: false,
        })
    }
}

fn gradient_weight(layout: &MetalReflectionLayout, lx: f32, ly: f32, tw: f32, th: f32) -> f32 {
    let u = if (layout.g1[0] - layout.g0[0]).abs() > (layout.g1[1] - layout.g0[1]).abs() {
        let t = lx / tw.max(1e-3);
        if layout.g0[0] > layout.g1[0] {
            (t - layout.g1[0]) / (layout.g0[0] - layout.g1[0] + 1e-6)
        } else {
            (t - layout.g0[0]) / (layout.g1[0] - layout.g0[0] + 1e-6)
        }
    } else {
        let t = ly / th.max(1e-3);
        if layout.g0[1] > layout.g1[1] {
            (t - layout.g1[1]) / (layout.g0[1] - layout.g1[1] + 1e-6)
        } else {
            (t - layout.g0[1]) / (layout.g1[1] - layout.g0[1] + 1e-6)
        }
    };
    // Stops: 0→1, mid→0.85, far→0 (matches GRAD_NEAR/MID/FAR usage).
    let u = u.clamp(0.0, 1.0);
    if u < 0.5 {
        1.0 - 0.15 * (u / 0.5)
    } else {
        0.85 * (1.0 - (u - 0.5) / 0.5)
    }
    .clamp(0.0, 1.0)
}

fn in_ring(lx: f32, ly: f32, w: f32, h: f32, r: f32, ring: f32) -> bool {
    let outer = sd_rr(lx, ly, w, h, r);
    let inner = if w > 2.0 * ring && h > 2.0 * ring {
        sd_rr(
            lx - ring,
            ly - ring,
            w - 2.0 * ring,
            h - 2.0 * ring,
            (r - ring).max(0.0),
        )
    } else {
        -1.0
    };
    outer <= 0.0 && inner > 0.0
}

fn sd_rr(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let rr = r.min(w.min(h) * 0.5).max(0.0);
    let qx = (x - w * 0.5).abs() - (w * 0.5 - rr);
    let qy = (y - h * 0.5).abs() - (h * 0.5 - rr);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    outside + qx.max(qy).min(0.0) - rr
}

/// Soft cursor occluder factor (`0..=1` keep) at a target-local pixel.
#[must_use]
pub fn occluder_keep(
    cfg: &MetalReflectionOccluderConfig,
    pointer: Option<[f32; 2]>,
    anchor: [f32; 4],
    target: [f32; 4],
    horiz: bool,
    lx: f32,
    ly: f32,
) -> f32 {
    let Some(p) = pointer else {
        return 1.0;
    };
    if !cfg.enabled || cfg.strength <= 0.0 {
        return 1.0;
    }
    let r = cfg.radius;
    let (gap_start, gap_end, along, across, band_lo, band_hi) = if horiz {
        let anchor_right = anchor[0] >= target[0] + target[2];
        let gap_start = if anchor_right {
            target[0] + target[2]
        } else {
            anchor[0] + anchor[2]
        };
        let gap_end = if anchor_right {
            anchor[0]
        } else {
            target[0]
        };
        let band_lo = anchor[1].max(target[1]);
        let band_hi = (anchor[1] + anchor[3]).min(target[1] + target[3]);
        (gap_start, gap_end, p[0], p[1], band_lo, band_hi)
    } else {
        let anchor_below = anchor[1] >= target[1] + target[3];
        let gap_start = if anchor_below {
            target[1] + target[3]
        } else {
            anchor[1] + anchor[3]
        };
        let gap_end = if anchor_below {
            anchor[1]
        } else {
            target[1]
        };
        let band_lo = anchor[0].max(target[0]);
        let band_hi = (anchor[0] + anchor[2]).min(target[0] + target[2]);
        (gap_start, gap_end, p[1], p[0], band_lo, band_hi)
    };
    let lo = gap_start.min(gap_end);
    let hi = gap_start.max(gap_end);
    let gap_w = (hi - lo).max(1.0);
    if along < lo - r || along > hi + r {
        return 1.0;
    }
    if across < band_lo - r || across > band_hi + r {
        return 1.0;
    }
    let t = ((along - gap_start).abs() / gap_w).clamp(0.0, 1.0);
    let fade_px = (r * cfg.edge_fade).max(0.5);
    let end_fade = ((along - (lo - r)).min((hi + r) - along) / fade_px).clamp(0.0, 1.0);
    let side_fade =
        ((across - (band_lo - r)).min((band_hi + r) - across) / fade_px).clamp(0.0, 1.0);
    let depth = cfg.strength * (1.0 - cfg.falloff * t) * end_fade * side_fade;
    if depth <= 0.001 {
        return 1.0;
    }
    let half_band = r * (1.0 + cfg.penumbra * t);
    let c = if horiz {
        across - target[1]
    } else {
        across - target[0]
    };
    let p_along = if horiz { ly } else { lx };
    let core = ((1.0 - cfg.softness) * 0.5).clamp(0.0, 0.5);
    let ramp = (0.5 - core).max(1e-3);
    let pn = (p_along + 0.5 - (c - half_band)) / (2.0 * half_band);
    let a = if pn < ramp {
        pn / ramp
    } else if pn > 1.0 - ramp {
        (1.0 - pn) / ramp
    } else {
        1.0
    }
    .clamp(0.0, 1.0);
    1.0 - depth * a
}

/// Inputs for [`paint_reflection`].
#[derive(Clone, Copy, Debug)]
pub struct MetalReflectionPaint<'a> {
    pub anchor: &'a MetalReflectionAnchor,
    pub target: [f32; 4],
    pub strength: f32,
    pub style: MetalReflectionStyle,
    pub density: f32,
    pub pointer: Option<[f32; 2]>,
    /// Optional glyph coverage; when set, reflection ink is clipped to it
    /// (CSS-px sampling via [`AlphaMask::sample`](super::mask::AlphaMask::sample)).
    pub glyph_mask: Option<&'a super::mask::AlphaMask>,
}

/// Renders one reflection frame into straight RGBA8 (target-local).
#[must_use]
pub fn paint_reflection(input: MetalReflectionPaint<'_>) -> Option<(u32, u32, Vec<u8>)> {
    let MetalReflectionPaint {
        anchor,
        target,
        strength,
        style,
        density,
        pointer,
        glyph_mask,
    } = input;
    // Reflections only in dark theme (web rule).
    if anchor.theme.is_light() {
        return None;
    }
    let layout = reflection_layout(anchor, target, strength, style)?;
    let density = density.clamp(1.0, 3.0);
    let tw = target[2];
    let th = target[3];
    let w = (tw * density).round().max(1.0) as u32;
    let h = (th * density).round().max(1.0) as u32;
    let material = MetalMaterial::new(anchor.mode);
    let mapping = SheetMapping::new(anchor.width, anchor.height, anchor.shader_scale);
    let u_time = material.u_time(anchor.clock);
    let cfg = reflection_occluder_config();

    let fill_alpha = if style == MetalReflectionStyle::Glyphs {
        (layout.reflection_alpha * FILL_OPACITY_MUL).min(1.0)
    } else {
        let circle = if anchor.variant == MetalFxVariant::Circle {
            FILL_CIRCLE_ATTENUATION
        } else {
            1.0
        };
        (layout.reflection_alpha * FILL_EXTRA_ALPHA * FILL_OPACITY_MUL * circle).min(MAX_ALPHA_STACK)
    };
    let stroke_alpha = layout.reflection_alpha * STROKE_EXTRA_ALPHA;
    let band = RANGE_PX + FILL_BLUR_CSS_PX * 3.0;

    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let draw = layout.draw;
    for py in 0..h {
        for px in 0..w {
            let lx = (px as f32 + 0.5) / density;
            let ly = (py as f32 + 0.5) / density;
            if lx < draw[0]
                || ly < draw[1]
                || lx >= draw[0] + draw[2]
                || ly >= draw[1] + draw[3]
            {
                continue;
            }
            // Map target-local draw px → mirrored anchor box.
            let ux = (lx - draw[0]) / draw[2].max(1e-3);
            let uy = (ly - draw[1]) / draw[3].max(1e-3);
            let ax = if layout.flip_x {
                (1.0 - ux) * anchor.width
            } else {
                ux * anchor.width
            };
            let ay = if layout.flip_y {
                (1.0 - uy) * anchor.height
            } else {
                uy * anchor.height
            };

            let on_source = if anchor.is_sheet || style == MetalReflectionStyle::Glyphs {
                ax >= 0.0 && ay >= 0.0 && ax <= anchor.width && ay <= anchor.height
            } else {
                in_ring(ax, ay, anchor.width, anchor.height, anchor.radius, anchor.ring_px)
            };
            if !on_source {
                continue;
            }

            let gw = gradient_weight(&layout, lx, ly, tw, th);
            if gw < 0.01 {
                continue;
            }

            // Edge-band mask for surface fill.
            let edge_d = -sd_rr(lx, ly, tw, th, (tw.min(th) * 0.5).min(999.0));
            let in_fill_band = style == MetalReflectionStyle::Glyphs || edge_d <= band;
            let in_stroke = style != MetalReflectionStyle::Glyphs && edge_d <= 1.5;

            let sample = material.shade(mapping.uv([ax, ay]), u_time, None);
            let mut a = 0.0_f32;
            if in_fill_band {
                a += sample[3] * fill_alpha * gw * (sample[0] + sample[1] + sample[2]) / 3.0;
                // Lifted brightness approximation of the CSS filter chain.
                a = (a * 1.2).min(1.0);
            }
            if in_stroke {
                a = (a + sample[3] * stroke_alpha * gw).min(1.0);
            }
            if style != MetalReflectionStyle::Glyphs {
                a = (a + BORDER_HILITE_ALPHA * layout.reflection_alpha * gw).min(1.0);
            }

            if let Some(mask) = glyph_mask {
                a *= mask.sample(lx, ly);
            }

            let keep = occluder_keep(
                &cfg,
                pointer,
                anchor.rect(),
                target,
                layout.horizontal,
                lx,
                ly,
            );
            a *= keep * anchor.strength;
            if a < 0.01 {
                continue;
            }
            let i = ((py * w + px) * 4) as usize;
            rgba[i] = (sample[0].clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba[i + 1] = (sample[1].clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba[i + 2] = (sample[2].clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba[i + 3] = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Some((w, h, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::metal_fx::preset::{MetalFxPreset, MetalFxResolvedTheme};
    use crate::widgets::metal_fx::reflection_anchor::MetalReflectionAnchor;

    fn anchor() -> MetalReflectionAnchor {
        MetalReflectionAnchor {
            origin: [0.0, 0.0],
            width: 140.0,
            height: 40.0,
            radius: 20.0,
            variant: MetalFxVariant::Button,
            mode: MetalFxPreset::Chromatic.mode(MetalFxResolvedTheme::Dark),
            theme: MetalFxResolvedTheme::Dark,
            clock: 0.5,
            strength: 1.0,
            ring_px: 1.0,
            shader_scale: 1.6,
            is_sheet: false,
        }
    }

    #[test]
    fn layout_faces_a_right_neighbour() {
        let a = anchor();
        let target = [150.0, 0.0, 100.0, 40.0];
        let layout = reflection_layout(&a, target, 1.0, MetalReflectionStyle::Surface).unwrap();
        assert!(layout.horizontal);
        assert!(layout.flip_x);
        assert!(
            layout.draw[0] < 1.0,
            "strip on the left of the target: {:?}",
            layout.draw
        );
        assert!(layout.reflection_alpha > 0.3);
    }

    #[test]
    fn paint_puts_ink_on_the_facing_edge() {
        let a = anchor();
        let target = [150.0, 0.0, 100.0, 40.0];
        let (w, h, rgba) = paint_reflection(MetalReflectionPaint {
            anchor: &a,
            target,
            strength: 1.0,
            style: MetalReflectionStyle::Surface,
            density: 1.0,
            pointer: None,
            glyph_mask: None,
        })
        .unwrap();
        assert_eq!((w, h), (100, 40));
        let mut ink = 0u32;
        for px in rgba.chunks_exact(4) {
            if px[3] > 8 {
                ink += 1;
            }
        }
        assert!(ink > 40, "expected mirrored band ink, got {ink}");
    }

    #[test]
    fn light_theme_skips_reflections() {
        let mut a = anchor();
        a.theme = MetalFxResolvedTheme::Light;
        assert!(
            paint_reflection(MetalReflectionPaint {
                anchor: &a,
                target: [150.0, 0.0, 100.0, 40.0],
                strength: 1.0,
                style: MetalReflectionStyle::Surface,
                density: 1.0,
                pointer: None,
                glyph_mask: None,
            })
            .is_none()
        );
    }
}
