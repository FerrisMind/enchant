//! Visual parity against the captured Libraries.dev frames.
//!
//! The reference renders a 348x122 card with radius 20 inside a 60px page
//! padding and captures at device scale 2. Each test frame is composited the
//! same way (page, burst behind, card fill, 1px border, beam above) and
//! compared with the browser crop as 6x4 mean-colour blocks.
//!
//! Set `BEAM_DUMP=1` to write the rendered frames to `target/border_beam/`.

use std::path::PathBuf;

use enchant_iced::{
    BeamBitmap, BeamHost, BeamLayers, BeamRenderOptions, BorderBeamOptions, BorderBeamPalette,
    BorderBeamSize, BorderBeamTheme, ResolvedBeamTheme,
};
use image::{Rgba, RgbaImage};

const CARD_W: f32 = 348.0;
const CARD_H: f32 = 122.0;
const RADIUS: f32 = 20.0;
const PAD: f32 = 60.0;
const DENSITY: f32 = 2.0;

const SIZES: [(&str, BorderBeamSize, f32); 5] = [
    ("sm", BorderBeamSize::Sm, 0.49),
    ("md", BorderBeamSize::Md, 0.49),
    ("line", BorderBeamSize::Line, 1.55),
    ("pulse-outside", BorderBeamSize::PulseOutside, 0.8),
    ("pulse-inner", BorderBeamSize::PulseInner, 0.8),
];

const PALETTES: [(&str, BorderBeamPalette); 8] = [
    ("colorful", BorderBeamPalette::Colorful),
    ("mono", BorderBeamPalette::Mono),
    ("ocean", BorderBeamPalette::Ocean),
    ("sunset", BorderBeamPalette::Sunset),
    ("forest", BorderBeamPalette::Forest),
    ("candy", BorderBeamPalette::Candy),
    ("ice", BorderBeamPalette::Ice),
    ("gold", BorderBeamPalette::Gold),
];

const THEMES: [(&str, ResolvedBeamTheme); 2] = [
    ("dark", ResolvedBeamTheme::Dark),
    ("light", ResolvedBeamTheme::Light),
];

fn parity_dir() -> PathBuf {
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/border_beam");
    if local.is_dir() {
        local
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.superpowers/beam-parity")
    }
}

fn page(theme: ResolvedBeamTheme) -> [u8; 3] {
    if theme.is_dark() {
        [7, 7, 7]
    } else {
        [255, 255, 255]
    }
}

fn card_fill(theme: ResolvedBeamTheme) -> [u8; 3] {
    if theme.is_dark() {
        [24, 24, 24]
    } else {
        [247, 247, 247]
    }
}

fn card_border(theme: ResolvedBeamTheme) -> [u8; 3] {
    if theme.is_dark() {
        [61, 61, 61]
    } else {
        [217, 217, 217]
    }
}

fn composite(layers: &BeamLayers, theme: ResolvedBeamTheme) -> RgbaImage {
    let width = ((CARD_W + 2.0 * PAD) * DENSITY) as u32;
    let height = ((CARD_H + 2.0 * PAD) * DENSITY) as u32;
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    let basemap = page(theme);
    for pixel in image.pixels_mut() {
        *pixel = Rgba([basemap[0], basemap[1], basemap[2], 255]);
    }
    if let Some(behind) = layers.behind() {
        draw_bitmap(&mut image, behind, PAD, PAD);
    }
    draw_card(&mut image, theme);
    if let Some(above) = layers.above() {
        draw_bitmap(&mut image, above, PAD, PAD);
    }
    image
}

fn draw_bitmap(image: &mut RgbaImage, bitmap: &BeamBitmap, origin_x: f32, origin_y: f32) {
    let x0 = ((origin_x + bitmap.left()) * DENSITY).round() as i64;
    let y0 = ((origin_y + bitmap.top()) * DENSITY).round() as i64;
    for row in 0..bitmap.height() {
        for col in 0..bitmap.width() {
            let x = x0 + col as i64;
            let y = y0 + row as i64;
            if x < 0 || y < 0 || x >= image.width() as i64 || y >= image.height() as i64 {
                continue;
            }
            let i = ((row * bitmap.width() + col) * 4) as usize;
            let src = &bitmap.rgba()[i..i + 4];
            if src[3] == 0 {
                continue;
            }
            let pixel = image.get_pixel_mut(x as u32, y as u32);
            blend(&mut pixel.0, [src[0], src[1], src[2], src[3]]);
        }
    }
}

fn blend(dst: &mut [u8; 4], src: [u8; 4]) {
    let sa = src[3] as f32 / 255.0;
    for c in 0..3 {
        dst[c] = (src[c] as f32 * sa + dst[c] as f32 * (1.0 - sa)).round() as u8;
    }
    dst[3] = 255;
}

fn draw_card(image: &mut RgbaImage, theme: ResolvedBeamTheme) {
    let fill = card_fill(theme);
    let border = card_border(theme);
    for y in 0..image.height() {
        for x in 0..image.width() {
            let mut acc = [0.0_f32; 3];
            let mut coverage = 0.0_f32;
            for sy in [0.25_f32, 0.75] {
                for sx in [0.25_f32, 0.75] {
                    let px = (x as f32 + sx) / DENSITY - PAD;
                    let py = (y as f32 + sy) / DENSITY - PAD;
                    let sd = rounded_rect_sd(px, py, CARD_W, CARD_H, RADIUS);
                    let color = if sd <= -1.0 {
                        fill
                    } else if sd < 0.0 {
                        mix(fill, border, -sd)
                    } else {
                        continue;
                    };
                    for c in 0..3 {
                        acc[c] += color[c] as f32 * 0.25;
                    }
                    coverage += 0.25;
                }
            }
            if coverage <= 0.0 {
                continue;
            }
            let color = acc.map(|v| v / coverage);
            let pixel = image.get_pixel_mut(x, y);
            let dst = [pixel.0[0] as f32, pixel.0[1] as f32, pixel.0[2] as f32];
            pixel.0 = [
                (color[0] * coverage + dst[0] * (1.0 - coverage)).round() as u8,
                (color[1] * coverage + dst[1] * (1.0 - coverage)).round() as u8,
                (color[2] * coverage + dst[2] * (1.0 - coverage)).round() as u8,
                255,
            ];
        }
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    std::array::from_fn(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8)
}

fn rounded_rect_sd(px: f32, py: f32, w: f32, h: f32, radius: f32) -> f32 {
    let hx = w * 0.5;
    let hy = h * 0.5;
    let qx = (px - hx).abs() - (hx - radius);
    let qy = (py - hy).abs() - (hy - radius);
    let outside = qx.max(0.0).hypot(qy.max(0.0));
    outside + qx.max(qy).min(0.0) - radius
}

fn render(
    size: BorderBeamSize,
    palette: BorderBeamPalette,
    theme: ResolvedBeamTheme,
    time: f32,
) -> BeamLayers {
    BorderBeamOptions::new()
        .size(size)
        .palette(palette)
        .theme(if theme.is_dark() {
            BorderBeamTheme::Dark
        } else {
            BorderBeamTheme::Light
        })
        .radius(RADIUS)
        .render_at(
            std::time::Duration::from_secs_f32(time),
            BeamHost::new(CARD_W, CARD_H),
            theme,
            &BeamRenderOptions::new().density(DENSITY),
        )
}

fn blocks(image: &RgbaImage) -> [[[f32; 3]; 6]; 4] {
    let mut out = [[[0.0; 3]; 6]; 4];
    let bw = image.width() / 6;
    let bh = image.height() / 4;
    for (by, row) in out.iter_mut().enumerate() {
        for (bx, block) in row.iter_mut().enumerate() {
            let mut acc = [0.0_f64; 3];
            let mut n = 0.0_f64;
            for y in by as u32 * bh..(by as u32 + 1) * bh {
                for x in bx as u32 * bw..(bx as u32 + 1) * bw {
                    let p = image.get_pixel(x, y).0;
                    for c in 0..3 {
                        acc[c] += p[c] as f64;
                    }
                    n += 1.0;
                }
            }
            for c in 0..3 {
                block[c] = (acc[c] / n) as f32;
            }
        }
    }
    out
}

#[test]
fn rendered_frames_match_the_browser_capture() {
    // The reference ships malformed `rgba(r, g, b,, a)` declarations for four
    // palettes; `ref-fixed` is captured from the same source through a Vite
    // transform that repairs the extra comma, matching this port's deliberate
    // deviation. Fall back to the unmodified captures when it is absent.
    let repaired = parity_dir().join("ref-fixed");
    let ref_dir = if repaired.is_dir() {
        repaired
    } else {
        parity_dir().join("ref")
    };
    assert!(
        ref_dir.is_dir(),
        "reference captures missing at {} (checked-in fixtures expected)",
        ref_dir.display()
    );
    let dump = std::env::var_os("BEAM_DUMP").is_some();
    let dump_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/border_beam");
    if dump {
        std::fs::create_dir_all(&dump_dir).expect("create dump dir");
    }

    let mut failures = Vec::new();
    let mut worst = (0.0_f32, String::new());
    for (size_name, size, time) in SIZES {
        for (palette_name, palette) in PALETTES {
            for (theme_name, theme) in THEMES {
                let name = format!("{size_name}_{palette_name}_{theme_name}");
                let layers = render(size, palette, theme, time);
                let image = composite(&layers, theme);
                if dump {
                    image
                        .save(dump_dir.join(format!("{name}.png")))
                        .expect("save frame");
                }
                let ref_path = ref_dir.join(format!("{name}.png"));
                let reference = image::open(&ref_path)
                    .unwrap_or_else(|err| panic!("open {}: {err}", ref_path.display()))
                    .to_rgba8();
                let a = blocks(&image);
                let b = blocks(&reference);
                let mut max = 0.0_f32;
                for (row_a, row_b) in a.iter().zip(b.iter()) {
                    for (block_a, block_b) in row_a.iter().zip(row_b.iter()) {
                        for c in 0..3 {
                            max = max.max((block_a[c] - block_b[c]).abs());
                        }
                    }
                }
                if max > worst.0 {
                    worst = (max, name.clone());
                }
                if max > 10.0 {
                    failures.push(format!("{name}: max block delta {max:.2}"));
                }
            }
        }
    }
    eprintln!("worst block delta: {:.2} ({})", worst.0, worst.1);
    assert!(
        failures.is_empty(),
        "{} frames exceed the block tolerance:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
