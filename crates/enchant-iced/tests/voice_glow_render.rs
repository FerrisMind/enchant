//! Headless renders of the voice glow: the software renderer's output at a
//! fixed time, level and host, checked for where the light is, what the clip
//! lets through, pause and strength semantics, the distortion warp and a
//! tolerance-based golden.

use std::time::Duration;

use enchant_iced::widgets::voice_glow::{
    GlowBitmap, HostSize, RenderOptions, ResolvedTheme, VoiceGlow, VoiceGlowTheme, VoiceGlowType,
};

const HOST: (f64, f64) = (350.0, 100.0);

fn render(glow: &VoiceGlow, secs: f64, theme: ResolvedTheme) -> GlowBitmap {
    glow.render_at(
        Duration::from_secs_f64(secs),
        HostSize::new(HOST.0, HOST.1),
        theme,
        &RenderOptions::new(),
    )
}

fn px(b: &GlowBitmap, x: usize, y: usize) -> [u8; 4] {
    let i = (y * b.width as usize + x) * 4;
    [b.rgba[i], b.rgba[i + 1], b.rgba[i + 2], b.rgba[i + 3]]
}

/// Sum of alpha over a pixel rectangle.
fn alpha_sum(b: &GlowBitmap, x0: usize, y0: usize, x1: usize, y1: usize) -> f64 {
    let mut sum = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += f64::from(px(b, x, y)[3]);
        }
    }
    sum
}

fn loud() -> VoiceGlow {
    VoiceGlow::new().level(0.9).border_radius(20.0)
}

#[test]
fn glow_is_concentrated_at_the_bottom_centre() {
    let b = render(&loud(), 2.0, ResolvedTheme::Dark);
    assert_eq!((b.width, b.height), (350, 100));
    let (w, h) = (350, 100);
    let bottom_centre = alpha_sum(&b, w / 3, h * 2 / 3, w * 2 / 3, h);
    let top_centre = alpha_sum(&b, w / 3, 0, w * 2 / 3, h / 3);
    let top_left = alpha_sum(&b, 0, 0, w / 3, h / 3);
    let top_right = alpha_sum(&b, w * 2 / 3, 0, w, h / 3);
    assert!(bottom_centre > 0.0);
    assert!(
        bottom_centre > 3.0 * top_centre,
        "bottom {bottom_centre} vs top {top_centre}"
    );
    assert!(bottom_centre > 3.0 * (top_left + top_right) / 2.0);
}

#[test]
fn the_edge_stroke_has_a_hot_core_at_the_bottom_centre() {
    let b = render(&loud(), 2.0, ResolvedTheme::Dark);
    let bottom_row = 99;
    let centre = px(&b, 175, bottom_row);
    let near_corner = px(&b, 30, bottom_row);
    assert!(
        centre[3] > 100,
        "the ring is opaque at the centre: {centre:?}"
    );
    // The mask is strongest at the centre of the edge: the ring is more
    // opaque there than near the corner (brightness alone is not a test:
    // saturated lobes clamp, like in the browser).
    assert!(centre[3] > near_corner[3], "{centre:?} vs {near_corner:?}");
}

#[test]
fn nothing_paints_outside_the_rounded_rect() {
    let b = render(&loud(), 2.0, ResolvedTheme::Dark);
    // The 20px corner arc leaves the very corner pixels outside.
    for &(x, y) in &[
        (0, 0),
        (1, 0),
        (0, 1),
        (349, 0),
        (0, 99),
        (349, 99),
        (348, 99),
    ] {
        assert_eq!(px(&b, x, y)[3], 0, "pixel ({x},{y}) is outside the clip");
    }
    // Bigger radius, bigger cut: a pixel inside the 20px host but outside a 50px one.
    let round = VoiceGlow::new().level(0.9).border_radius(50.0);
    let r = render(&round, 2.0, ResolvedTheme::Dark);
    assert_eq!(px(&r, 8, 99)[3], 0);
    assert!(px(&b, 8, 99)[3] > 0 || alpha_sum(&b, 6, 90, 12, 100) > 0.0);
}

#[test]
fn a_paused_glow_is_the_same_frame_at_any_time() {
    let glow = loud().paused(true);
    let early = render(&glow, 1.0, ResolvedTheme::Dark);
    let late = render(&glow, 3.5, ResolvedTheme::Dark);
    assert_eq!(early, late);
}

#[test]
fn a_running_glow_changes_with_time() {
    let glow = VoiceGlow::new().level_source(
        enchant_iced::widgets::voice_glow::LevelSource::getter(|| 0.8),
    );
    let a = render(&glow, 1.0, ResolvedTheme::Dark);
    let b = render(&glow, 3.0, ResolvedTheme::Dark);
    assert_ne!(a, b);
}

#[test]
fn strength_zero_draws_nothing() {
    let b = render(&loud().strength(0.0), 2.0, ResolvedTheme::Dark);
    assert!(b.rgba.chunks(4).all(|p| p[3] == 0));
    let light = render(&loud().strength(0.0), 2.0, ResolvedTheme::Light);
    // The light theme's epicentre wash does not follow strength (upstream),
    // everything else does.
    let wash: u32 = light.rgba.chunks(4).map(|p| u32::from(p[3])).sum();
    let full = render(&loud(), 2.0, ResolvedTheme::Light);
    let full_sum: u32 = full.rgba.chunks(4).map(|p| u32::from(p[3])).sum();
    assert!(wash < full_sum);
}

#[test]
fn an_inactive_glow_fades_to_nothing() {
    let b = render(&loud().active(false), 2.0, ResolvedTheme::Dark);
    assert!(b.rgba.chunks(4).all(|p| p[3] == 0));
}

#[test]
fn a_louder_voice_lights_more() {
    let quiet = render(&VoiceGlow::new().level(0.1), 3.0, ResolvedTheme::Dark);
    let loud = render(&VoiceGlow::new().level(1.0), 3.0, ResolvedTheme::Dark);
    let sum = |b: &GlowBitmap| b.rgba.chunks(4).map(|p| u64::from(p[3])).sum::<u64>();
    assert!(sum(&loud) > sum(&quiet));
}

#[test]
fn the_warp_moves_pixels_below_the_band_line_only() {
    let warped = render(&loud().distortion(1.0), 2.0, ResolvedTheme::Dark);
    let flat = render(&loud().distortion(0.0), 2.0, ResolvedTheme::Dark);
    assert_ne!(warped, flat, "distortion should change the picture");
    // Far above the glow (top rows) the two are the same.
    for y in 0..8 {
        for x in 0..350 {
            let (a, b) = (px(&warped, x, y), px(&flat, x, y));
            for c in 0..4 {
                assert!(
                    (i32::from(a[c]) - i32::from(b[c])).abs() <= 3,
                    "({x},{y}) differs above the band: {a:?} vs {b:?}"
                );
            }
        }
    }
}

#[test]
fn density_scales_the_bitmap() {
    let glow = loud();
    let b = glow.render_at(
        Duration::from_secs(1),
        HostSize::new(120.0, 40.0),
        ResolvedTheme::Dark,
        &RenderOptions::new().density(2.0),
    );
    assert_eq!((b.width, b.height), (240, 80));
}

#[test]
fn a_background_fills_the_host_and_the_page_fills_the_corners() {
    let b = loud().render_at(
        Duration::from_secs(1),
        HostSize::new(120.0, 40.0),
        ResolvedTheme::Dark,
        &RenderOptions::new()
            .background([30, 30, 30, 255])
            .page([255, 0, 0, 255]),
    );
    assert_eq!(px(&b, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&b, 60, 5)[3], 255);
    assert!(b.rgba.chunks(4).all(|p| p[3] == 255));
}

#[test]
fn every_type_and_theme_renders() {
    for kind in [
        VoiceGlowType::Default,
        VoiceGlowType::Mobile,
        VoiceGlowType::Pill,
    ] {
        for theme in [ResolvedTheme::Dark, ResolvedTheme::Light] {
            let glow = VoiceGlow::new().kind(kind).level(0.7);
            let b = glow.render_at(
                Duration::from_secs(2),
                HostSize::new(200.0, 80.0),
                theme,
                &RenderOptions::new(),
            );
            assert!(
                b.rgba.chunks(4).any(|p| p[3] > 0),
                "{kind:?}/{theme:?} drew nothing"
            );
        }
    }
}

#[test]
fn rendering_is_deterministic() {
    let a = render(&loud(), 2.0, ResolvedTheme::Dark);
    let b = render(&loud(), 2.0, ResolvedTheme::Dark);
    assert_eq!(a, b);
}

/// Mean RGBA over a `cols x rows` grid of cells.
fn grid(b: &GlowBitmap, cols: usize, rows: usize) -> Vec<[f64; 4]> {
    let (w, h) = (b.width as usize, b.height as usize);
    let mut out = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let (x0, x1) = (c * w / cols, (c + 1) * w / cols);
            let (y0, y1) = (r * h / rows, (r + 1) * h / rows);
            let mut acc = [0.0; 4];
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = px(b, x, y);
                    // Weighted by alpha so transparent pixels do not drag the colour.
                    let a = f64::from(p[3]) / 255.0;
                    for k in 0..3 {
                        acc[k] += f64::from(p[k]) * a;
                    }
                    acc[3] += f64::from(p[3]);
                }
            }
            out.push(acc.map(|v| v / n));
        }
    }
    out
}

/// A fixed render of the default dark chat host at 2 s, level 0.8: a regression
/// golden of this renderer (the browser comparison is below).
/// Regenerate with `VOICE_GLOW_PRINT_GOLDEN=1 cargo test --test voice_glow_render golden -- --nocapture`.
const GOLDEN: [[f64; 4]; 24] = [
    [0.373, 0.373, 0.373, 0.373],
    [0.909, 0.946, 0.939, 0.949],
    [4.006, 4.182, 4.406, 4.422],
    [4.820, 7.401, 5.593, 7.414],
    [1.660, 5.401, 5.693, 5.760],
    [0.593, 0.784, 0.871, 0.893],
    [4.789, 4.494, 4.151, 4.790],
    [12.611, 16.091, 15.355, 16.661],
    [36.169, 33.615, 40.668, 45.388],
    [41.711, 51.214, 42.615, 59.931],
    [19.346, 36.235, 38.490, 39.747],
    [12.171, 9.497, 12.476, 13.893],
    [43.090, 33.173, 29.914, 43.090],
    [77.326, 72.848, 70.219, 93.943],
    [125.579, 119.704, 128.871, 152.759],
    [128.693, 139.902, 123.455, 162.184],
    [98.418, 94.634, 101.409, 125.485],
    [64.016, 40.623, 53.432, 65.645],
    [128.460, 99.648, 90.888, 130.452],
    [128.536, 148.951, 142.840, 166.970],
    [82.359, 134.752, 161.465, 165.549],
    [81.077, 156.646, 142.811, 170.333],
    [122.766, 153.120, 157.996, 170.554],
    [140.178, 108.231, 115.756, 144.637],
];

#[test]
fn golden_grid_of_the_default_dark_host() {
    let glow = VoiceGlow::new().level(0.8).border_radius(20.0);
    let b = render(&glow, 2.0, ResolvedTheme::Dark);
    let cells = grid(&b, 6, 4);
    if std::env::var_os("VOICE_GLOW_PRINT_GOLDEN").is_some() {
        println!("const GOLDEN: [[f64; 4]; 24] = [");
        for c in &cells {
            println!("    [{:.3}, {:.3}, {:.3}, {:.3}],", c[0], c[1], c[2], c[3]);
        }
        println!("];");
        return;
    }
    for (i, (got, want)) in cells.iter().zip(GOLDEN).enumerate() {
        for k in 0..4 {
            assert!(
                (got[k] - want[k]).abs() <= 1.5,
                "cell {i} channel {k}: got {:.3}, golden {:.3}",
                got[k],
                want[k]
            );
        }
    }
}

#[test]
fn a_350x100_frame_renders_within_the_frame_budget() {
    use enchant_iced::widgets::voice_glow::VoiceGlowRaster;

    let glow = VoiceGlow::new().level(0.9).border_radius(20.0);
    let host = HostSize::new(350.0, 100.0);
    let config = glow.config(ResolvedTheme::Dark);
    let mut engine = glow.engine(ResolvedTheme::Dark);
    let mut raster = VoiceGlowRaster::new();
    // Warm the buffers, then time steady-state frames (engine step included).
    for _ in 0..5 {
        let frame = engine.step(1.0 / 60.0, host);
        let _ = raster.render(&frame, &config, 1.0);
    }
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        let frame = engine.step(1.0 / 60.0, host);
        let _ = raster.render(&frame, &config, 1.0);
    }
    let per_frame = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    eprintln!("voice glow 350x100 @1x: {per_frame:.2} ms/frame");
    // 60 fps is 16.7 ms; leave slack for unoptimised test profiles and noisy CI.
    assert!(per_frame < 40.0, "{per_frame:.2} ms/frame");
}

// ── browser reference ───────────────────────────────────────────────────────
// Mean RGB of a 6x4 grid of cells of Chromium 1234 captures (2x DPR, the
// harness of `tools/voice_cmp`: `VoiceBeam` from the TypeScript package with
// a fake 60 Hz clock, host `#1d1d1d` / `#ffffff` on a `#0a0a0a` / `#e6e6e6`
// page), after 300 / 200 frames. Tolerance 3/255 on every cell mean (measured worst: 2.6 on the pill, whose strong warp is the loosest part).

const DEFAULT_DARK_L0_6: [[f64; 4]; 24] = [
    [28.02, 28.02, 28.02, 255.00],
    [31.41, 30.68, 31.34, 255.00],
    [46.52, 33.99, 33.53, 255.00],
    [40.44, 31.76, 36.96, 255.00],
    [29.68, 29.41, 29.66, 255.00],
    [28.03, 28.03, 28.03, 255.00],
    [30.04, 30.20, 30.22, 255.00],
    [42.92, 43.01, 47.86, 255.00],
    [84.81, 54.11, 54.76, 255.00],
    [80.33, 49.95, 68.12, 255.00],
    [40.14, 37.46, 36.70, 255.00],
    [30.50, 30.49, 30.47, 255.00],
    [43.65, 47.74, 46.36, 255.00],
    [83.09, 79.30, 82.09, 255.00],
    [159.64, 111.45, 100.25, 255.00],
    [159.16, 108.53, 115.83, 255.00],
    [97.38, 77.21, 58.08, 255.00],
    [47.29, 47.49, 46.26, 255.00],
    [99.79, 95.34, 78.91, 255.00],
    [141.09, 147.28, 147.53, 255.00],
    [169.31, 132.32, 149.05, 255.00],
    [166.32, 126.31, 165.18, 255.00],
    [151.93, 144.91, 124.39, 255.00],
    [108.86, 101.17, 87.26, 255.00],
];

const PILL_DARK_L1: [[f64; 4]; 24] = [
    [22.31, 22.31, 22.31, 255.00],
    [31.51, 42.04, 42.42, 255.00],
    [75.07, 55.94, 69.96, 255.00],
    [79.82, 42.22, 62.84, 255.00],
    [44.72, 37.15, 35.14, 255.00],
    [22.44, 22.44, 22.43, 255.00],
    [28.67, 28.67, 28.88, 255.00],
    [35.06, 59.99, 60.75, 255.00],
    [110.36, 81.85, 102.66, 255.00],
    [119.83, 60.76, 90.16, 255.00],
    [65.00, 52.01, 41.84, 255.00],
    [28.70, 29.15, 29.02, 255.00],
    [36.93, 35.39, 37.89, 255.00],
    [65.20, 98.11, 100.01, 255.00],
    [172.79, 135.93, 134.48, 255.00],
    [186.67, 118.04, 120.46, 255.00],
    [113.43, 98.14, 73.40, 255.00],
    [38.83, 43.63, 43.16, 255.00],
    [63.86, 54.87, 48.92, 255.00],
    [146.09, 160.65, 147.50, 255.00],
    [205.11, 177.90, 193.58, 255.00],
    [209.47, 154.86, 180.36, 255.00],
    [176.49, 166.45, 143.78, 255.00],
    [74.11, 72.32, 58.02, 255.00],
];

const MOBILE_LIGHT_L0_6: [[f64; 4]; 24] = [
    [252.29, 252.29, 252.29, 255.00],
    [255.00, 254.97, 254.98, 255.00],
    [255.00, 254.94, 254.97, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [252.30, 252.30, 252.30, 255.00],
    [252.75, 254.87, 251.81, 255.00],
    [253.37, 251.56, 249.50, 255.00],
    [254.98, 249.99, 252.36, 255.00],
    [254.61, 254.43, 253.48, 255.00],
    [255.00, 255.00, 254.99, 255.00],
    [254.98, 254.98, 254.98, 255.00],
    [246.00, 254.04, 241.31, 255.00],
    [247.87, 244.43, 232.55, 255.00],
    [254.82, 235.37, 238.77, 255.00],
    [250.92, 248.62, 241.87, 255.00],
    [251.48, 252.82, 254.68, 255.00],
    [254.53, 253.98, 254.83, 255.00],
    [236.53, 227.61, 222.77, 255.00],
    [241.67, 223.27, 222.00, 255.00],
    [252.19, 220.02, 232.33, 255.00],
    [248.38, 228.98, 235.32, 255.00],
    [244.60, 227.85, 245.44, 255.00],
    [246.12, 222.98, 243.47, 255.00],
];

const DEFAULT_LIGHT_PROCESSING_B: [[f64; 4]; 24] = [
    [253.62, 253.62, 253.62, 255.00],
    [254.98, 254.98, 254.98, 255.00],
    [254.80, 254.80, 254.80, 255.00],
    [254.89, 254.89, 254.89, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [253.63, 253.63, 253.63, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [254.99, 254.94, 254.98, 255.00],
    [254.93, 254.04, 254.77, 255.00],
    [254.95, 254.58, 254.90, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [255.00, 254.98, 254.99, 255.00],
    [254.89, 253.14, 254.59, 255.00],
    [254.36, 242.00, 248.85, 255.00],
    [253.63, 247.96, 251.46, 255.00],
    [254.90, 254.56, 254.83, 255.00],
    [255.00, 255.00, 255.00, 255.00],
    [253.59, 252.92, 253.45, 255.00],
    [252.61, 239.20, 245.74, 255.00],
    [242.52, 228.51, 243.60, 255.00],
    [242.55, 228.01, 234.20, 255.00],
    [253.92, 249.82, 251.42, 255.00],
    [253.63, 253.63, 253.63, 255.00],
];

fn browser_grid(
    glow: &VoiceGlow,
    theme: ResolvedTheme,
    host: (f64, f64),
    frames: f64,
    golden: &[[f64; 4]; 24],
) {
    let (bg, page) = if theme.is_dark() {
        ([29, 29, 29, 255], [10, 10, 10, 255])
    } else {
        ([255, 255, 255, 255], [230, 230, 230, 255])
    };
    let b = glow.render_at(
        Duration::from_secs_f64((frames + 0.01) / 60.0),
        HostSize::new(host.0, host.1),
        theme,
        &RenderOptions::new().density(2.0).background(bg).page(page),
    );
    for (i, (got, want)) in grid(&b, 6, 4).iter().zip(golden).enumerate() {
        for k in 0..3 {
            assert!(
                (got[k] - want[k]).abs() <= 3.0,
                "cell {i} channel {k}: got {:.2}, browser {:.2}",
                got[k],
                want[k]
            );
        }
    }
}

#[test]
fn matches_the_browser_chat_input_dark() {
    let glow = VoiceGlow::new()
        .theme(VoiceGlowTheme::Dark)
        .level(0.6)
        .border_radius(20.0);
    browser_grid(
        &glow,
        ResolvedTheme::Dark,
        (371.0, 104.0),
        300.0,
        &DEFAULT_DARK_L0_6,
    );
}

#[test]
fn matches_the_browser_recording_pill_dark() {
    let glow = VoiceGlow::new()
        .kind(VoiceGlowType::Pill)
        .theme(VoiceGlowTheme::Dark)
        .level(1.0)
        .border_radius(22.0);
    browser_grid(
        &glow,
        ResolvedTheme::Dark,
        (149.0, 44.0),
        300.0,
        &PILL_DARK_L1,
    );
}

#[test]
fn matches_the_browser_mobile_light() {
    let glow = VoiceGlow::new()
        .kind(VoiceGlowType::Mobile)
        .theme(VoiceGlowTheme::Light)
        .level(0.6)
        .scale(0.85)
        .border_radius(44.88);
    browser_grid(
        &glow,
        ResolvedTheme::Light,
        (273.0, 357.0),
        300.0,
        &MOBILE_LIGHT_L0_6,
    );
}

#[test]
fn matches_the_browser_processing_light() {
    let glow = VoiceGlow::new()
        .theme(VoiceGlowTheme::Light)
        .level(0.0)
        .processing(true)
        .border_radius(20.0);
    browser_grid(
        &glow,
        ResolvedTheme::Light,
        (371.0, 104.0),
        200.0,
        &DEFAULT_LIGHT_PROCESSING_B,
    );
}
