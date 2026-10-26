// metal-fx: Paper Shaders `liquidMetal` (shape: none) + the metal-fx layer stack.
//
// Port basis: paper-design/shaders 0.0.80 `liquid-metal` fragment stage, its
// sizing vertex stage (`shader-sizing`) and the MetalFxKit MSL port. The
// material is evaluated per fragment from a *sheet* coordinate: the web draws
// the material once onto a 192 px square sheet and crops a window onto every
// instance; here the same window is `sheet.xy + p * sheet.zw`.
//
// One program serves four layers, selected by `metrics.w`:
//   0 ring     liquid-metal band + the circle variant's outer hairline
//   1 glow     baked halo / catch-light sprites at the hotspot
//   2 rim      inner-shadow rim + the `::after` hairline
//   3 surface  wrapper background + the `::before` inset shadow
// All outputs are premultiplied; `finish` converts to linear for sRGB targets.

struct Uniforms {
    rect: vec4<f32>,         // quad origin (box-local css px) xy, quad size zw
    metrics: vec4<f32>,      // box w, box h, density, layer
    shape: vec4<f32>,        // radius, ring, after-hairline inset, glow ring inset
    flags: vec4<f32>,        // is_circle, is_light, target_srgb, dither
    color_back: vec4<f32>,
    color_tint: vec4<f32>,
    mat0: vec4<f32>,         // repetition, softness, shiftRed, shiftBlue
    mat1: vec4<f32>,         // distortion, contour, angle, u_time
    sheet: vec4<f32>,        // origin xy, scale zw
    sizing0: vec4<f32>,      // originX, originY, worldWidth, worldHeight
    sizing1: vec4<f32>,      // fit, scale, rotation, offsetX
    sizing2: vec4<f32>,      // offsetY, pixelRatio, sheetRes, opacityMul
    hair: vec4<f32>,         // ::after hairline rgba (straight)
    circ: vec4<f32>,         // circle outer hairline rgba (straight)
    halo: vec4<f32>,         // pos xy, tangent, opacity
    extra: vec4<f32>,        // pos xy, tangent, opacity
    halo_tint: vec4<f32>,    // rgb, envelope
    extra_tint: vec4<f32>,   // rgb, group alpha (0.7 dark / 0.2746 light)
    halo_sprite: vec4<f32>,  // w, h, ax, ay (css px)
    extra_sprite: vec4<f32>,
    glow_misc: vec4<f32>,    // saturate, brightness, enabled, _
    rim: vec4<f32>,          // offsetY, blur, alpha, enabled
    rim_color: vec4<f32>,
    surf: vec4<f32>,         // surface rgba (straight)
    shadow: vec4<f32>,       // ::before shadow rgba (straight)
    shadow_misc: vec4<f32>,  // sigma, _, _, _
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var halo_tex: texture_2d<f32>;
@group(0) @binding(2) var extra_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

const PI: f32 = 3.14159265358979323846;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[i];
    var out: VsOut;
    out.pos = vec4<f32>(c.x * 2.0 - 1.0, 1.0 - c.y * 2.0, 0.0, 1.0);
    out.uv = c;
    return out;
}

// ─── GLSL helpers ───────────────────────────────────────────────────────────

fn ss(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp((x - e0) / max(e1 - e0, 1e-6), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn gmod2(x: vec2<f32>, y: f32) -> vec2<f32> { return x - y * floor(x / y); }
fn gmod3(x: vec3<f32>, y: f32) -> vec3<f32> { return x - y * floor(x / y); }

// mat2(cos, sin, -sin, cos) * uv
fn rotate2(uv: vec2<f32>, th: f32) -> vec2<f32> {
    let c = cos(th);
    let s = sin(th);
    return vec2<f32>(c * uv.x - s * uv.y, s * uv.x + c * uv.y);
}

fn permute(x: vec3<f32>) -> vec3<f32> { return gmod3(((x * 34.0) + 1.0) * x, 289.0); }

fn snoise(v: vec2<f32>) -> f32 {
    let C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);
    let i1 = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), x0.x > x0.y);
    var x12 = x0.xyxy + C.xxzz;
    x12 = vec4<f32>(x12.xy - i1, x12.zw);
    i = gmod2(i, 289.0);
    let p = permute(permute(i.y + vec3<f32>(0.0, i1.y, 1.0)) + i.x + vec3<f32>(0.0, i1.x, 1.0));
    var m = max(vec3<f32>(0.5) - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;
    let x = 2.0 * fract(p * C.www) - 1.0;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;
    m = m * (1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h));
    let g = vec3<f32>(
        a0.x * x0.x + h.x * x0.y,
        a0.y * x12.x + h.y * x12.y,
        a0.z * x12.z + h.z * x12.w,
    );
    return 130.0 * dot(m, g);
}

// ─── Paper sizing vertex stage ──────────────────────────────────────────────

struct SheetUvs {
    object_uv: vec2<f32>,
    responsive_uv: vec2<f32>,
    given: vec2<f32>,
};

fn get_box_size(box_ratio: f32, given_box: vec2<f32>, res: vec2<f32>, fit: f32) -> vec2<f32> {
    var b = vec2<f32>(0.0);
    b.x = box_ratio * min(given_box.x / box_ratio, given_box.y);
    if (fit == 1.0) {
        b.x = box_ratio * min(res.x / box_ratio, res.y);
    } else if (fit == 2.0) {
        b.x = box_ratio * max(res.x / box_ratio, res.y);
    }
    b.y = b.x / box_ratio;
    return b;
}

fn sheet_uvs(ndc: vec2<f32>) -> SheetUvs {
    let res = vec2<f32>(u.sizing2.z);
    let pixel_ratio = u.sizing2.y;
    let scale = u.sizing1.y;
    let fit = u.sizing1.x;
    let uv = ndc * 0.5;
    let box_origin = vec2<f32>(0.5 - u.sizing0.x, u.sizing0.y - 0.5);
    var given = vec2<f32>(u.sizing0.z, u.sizing0.w);
    given = max(given, vec2<f32>(1.0)) * pixel_ratio;
    let r = u.sizing1.z * PI / 180.0;
    let graphic_offset = vec2<f32>(-u.sizing1.w, u.sizing2.x);

    let fixed_given = vec2<f32>(
        select(given.x, res.x, u.sizing0.z == 0.0),
        select(given.y, res.y, u.sizing0.w == 0.0),
    );

    let obj_box = get_box_size(1.0, fixed_given, res, fit);
    let obj_scale = res / obj_box;
    var ouv = uv * obj_scale;
    ouv = ouv + box_origin * (obj_scale - 1.0);
    ouv = ouv + graphic_offset;
    ouv = ouv / scale;
    ouv = rotate2(ouv, r);

    let rratio = fixed_given.x / fixed_given.y;
    let rbox = get_box_size(rratio, fixed_given, res, fit);
    let rscale = res / rbox;
    var ruv = uv * rscale;
    ruv = ruv + box_origin * (rscale - 1.0);
    ruv = ruv + graphic_offset;
    ruv = ruv / scale;
    ruv.x = ruv.x * rratio;
    ruv = rotate2(ruv, r);
    ruv.x = ruv.x / rratio;

    var out: SheetUvs;
    out.object_uv = ouv;
    out.responsive_uv = ruv;
    out.given = fixed_given;
    return out;
}

// ─── liquidMetal fragment stage (shape = none) ──────────────────────────────

struct Core {
    edge_o: f32,
    edge: f32,
    direction: f32,
    bump: f32,
    diag_tl: f32,
    noise: f32,
    dr: f32,
    db: f32,
};

fn edge_raw(s: vec2<f32>) -> f32 {
    let ndc = vec2<f32>(2.0 * s.x - 1.0, 1.0 - 2.0 * s.y);
    let su = sheet_uvs(ndc);
    let border_uv = su.responsive_uv + 0.5;
    let mask = min(border_uv, 1.0 - border_uv);
    let pt = min(vec2<f32>(250.0) / su.given, vec2<f32>(0.5));
    let mx = pow(ss(0.0, pt.x, mask.x), 0.25);
    let my = pow(ss(0.0, pt.y, mask.y), 0.25);
    return clamp(1.0 - mx * my, 0.0, 1.0);
}

fn liquid_core(s: vec2<f32>, t: f32, fw_edge: f32) -> Core {
    let ndc = vec2<f32>(2.0 * s.x - 1.0, 1.0 - 2.0 * s.y);
    let su = sheet_uvs(ndc);
    let distortion = u.mat1.x;
    let contour = u.mat1.y;
    let angle_deg = u.mat1.z;
    var cycle_width = u.mat0.x;

    var uv_obj = su.object_uv + 0.5;
    uv_obj.y = 1.0 - uv_obj.y;
    var rotated = uv_obj - vec2<f32>(0.5);
    let angle = (-angle_deg + 70.0) * PI / 180.0;
    let ca = cos(angle);
    let sa = sin(angle);
    rotated = vec2<f32>(rotated.x * ca - rotated.y * sa, rotated.x * sa + rotated.y * ca) + vec2<f32>(0.5);

    // shape == none: full-fill on canvas
    let raw = edge_raw(s);
    let ratio = su.given.x / su.given.y;
    var uv = su.responsive_uv;
    if (ratio > 1.0) {
        uv.y = uv.y / ratio;
    } else {
        uv.x = uv.x * ratio;
    }
    uv = uv + 0.5;
    uv.y = 1.0 - uv.y;
    cycle_width = cycle_width * 2.0;

    let edge_o = mix(ss(0.9 - 2.0 * fw_edge, 0.9, raw), raw, ss(0.0, 0.4, contour));
    var edge = 1.2 * edge_o;

    let diag_bl = rotated.x - rotated.y;
    let diag_tl = rotated.x + rotated.y;

    var grad_uv = uv - 0.5;
    let dist = length(grad_uv + vec2<f32>(0.0, 0.2 * diag_bl));
    grad_uv = rotate2(grad_uv, (0.25 - 0.2 * diag_bl) * PI);
    var direction = grad_uv.x;

    let uy = max(uv.y, 0.0);
    var bump = pow(1.8 * dist, 1.2);
    bump = 1.0 - bump;
    bump = bump * pow(uy, 0.3);

    let noise = snoise(uv - t);

    edge = edge + (1.0 - edge) * distortion * noise;

    direction = direction + diag_bl;
    let se = ss(0.0, 1.0, edge);
    direction = direction - 2.0 * noise * diag_bl * (se * (1.0 - se));
    let c51 = ss(0.5, 1.0, contour);
    direction = direction * mix(1.0, 1.0 - edge, c51);
    direction = direction - 1.7 * edge * c51;
    direction = direction + 0.2 * pow(contour, 4.0) * (1.0 - se);

    bump = bump * clamp(pow(uy, 0.1), 0.3, 1.0);
    direction = direction * (0.1 + (1.1 - edge) * bump);

    direction = direction * (0.4 + 0.6 * (1.0 - ss(0.5, 1.0, edge)));
    direction = direction + 0.18 * (ss(0.1, 0.2, uv.y) * (1.0 - ss(0.2, 0.4, uv.y)));
    direction = direction + 0.03 * (ss(0.1, 0.2, 1.0 - uv.y) * (1.0 - ss(0.2, 0.4, 1.0 - uv.y)));

    direction = direction * (0.5 + 0.5 * uv.y * uv.y);
    direction = direction * cycle_width;
    direction = direction - t;

    var color_dispersion = clamp(1.0 - bump, 0.0, 1.0);
    var dr = color_dispersion;
    dr = dr + 0.03 * bump * noise;
    dr = dr + 5.0 * (ss(-0.1, 0.2, uv.y) * (1.0 - ss(0.1, 0.5, uv.y))) * (ss(0.4, 0.6, bump) * (1.0 - ss(0.4, 1.0, bump)));
    dr = dr - diag_bl;

    var db = color_dispersion * 1.3;
    db = db + (ss(0.0, 0.4, uv.y) * (1.0 - ss(0.1, 0.8, uv.y))) * (ss(0.4, 0.6, bump) * (1.0 - ss(0.4, 0.8, bump)));
    db = db - 0.2 * edge;

    dr = dr * (u.mat0.z / 20.0);
    db = db * (u.mat0.w / 20.0);

    var out: Core;
    out.edge_o = edge_o;
    out.edge = edge;
    out.direction = direction;
    out.bump = bump;
    out.diag_tl = diag_tl;
    out.noise = noise;
    out.dr = dr;
    out.db = db;
    return out;
}

fn color_changes(c1: f32, c2: f32, p: f32, w: vec3<f32>, blur_in: f32, bump: f32, tint: f32) -> f32 {
    let blur = max(blur_in, 1e-5);
    var ch = mix(c2, c1, ss(0.0, 2.0 * blur, p));
    var border = w.x;
    ch = mix(ch, c2, ss(border, border + 2.0 * blur, p));
    border = w.x + 0.4 * (1.0 - bump) * w.y;
    ch = mix(ch, c1, ss(border, border + 2.0 * blur, p));
    border = w.x + 0.5 * (1.0 - bump) * w.y;
    ch = mix(ch, c2, ss(border, border + 2.0 * blur, p));
    border = w.x + w.y;
    ch = mix(ch, c1, ss(border, border + 2.0 * blur, p));
    let gradient_t = (p - w.x - w.y) / w.z;
    let gradient = mix(c1, c2, ss(0.0, 1.0, gradient_t));
    ch = mix(ch, gradient, ss(border, border + 0.5 * blur, p));
    ch = mix(ch, 1.0 - min(1.0, (1.0 - ch) / max(tint, 0.0001)), u.color_tint.w);
    return ch;
}

// The material at sheet coordinate `s` (0..1, y down), premultiplied.
fn material(s: vec2<f32>, frag_px: vec2<f32>) -> vec4<f32> {
    let t = 0.3 * (u.mat1.w + 2.8);
    let texel = 1.0 / u.sizing2.z;
    let sx = s + vec2<f32>(texel, 0.0);
    let sy = s + vec2<f32>(0.0, texel);

    let e0 = edge_raw(s);
    let fw_edge = abs(edge_raw(sx) - e0) + abs(edge_raw(sy) - e0);

    let c = liquid_core(s, t, fw_edge);
    let cx = liquid_core(sx, t, fw_edge);
    let cy = liquid_core(sy, t, fw_edge);

    let fw_r = abs((cx.direction + cx.dr) - (c.direction + c.dr)) + abs((cy.direction + cy.dr) - (c.direction + c.dr));
    let fw_g = abs(cx.direction - c.direction) + abs(cy.direction - c.direction);
    let fw_b = abs((cx.direction - cx.db) - (c.direction - c.db)) + abs((cy.direction - cy.db) - (c.direction - c.db));

    var opacity = 1.0 - ss(0.9 - 2.0 * fw_edge, 0.9, c.edge_o);

    let color1 = vec3<f32>(0.98, 0.98, 1.0);
    let color2 = vec3<f32>(0.1, 0.1, 0.1 + 0.1 * ss(0.7, 1.3, c.diag_tl));

    let cw = u.mat0.x * 2.0;
    let thin1 = 0.12 / cw * (1.0 - 0.4 * c.bump);
    let thin2 = 0.07 / cw * (1.0 + 0.4 * c.bump);
    let wide = 1.0 - thin1 - thin2;
    var w = vec3<f32>(cw * thin1, cw * thin2, wide);
    w.y = w.y - 0.02 * ss(0.0, 1.0, c.edge + c.bump);

    let blur = u.mat0.y / 15.0;
    let stripe_r = fract(c.direction + c.dr);
    let stripe_g = fract(c.direction);
    let stripe_b = fract(c.direction - c.db);
    let r = color_changes(color1.x, color2.x, stripe_r, w, blur + fw_r, c.bump, u.color_tint.x);
    let g = color_changes(color1.y, color2.y, stripe_g, w, blur + fw_g, c.bump, u.color_tint.y);
    let b = color_changes(color1.z, color2.z, stripe_b, w, blur + fw_b, c.bump, u.color_tint.z);

    var color = vec3<f32>(r, g, b) * opacity;
    let bg = u.color_back.rgb * u.color_back.a;
    color = color + bg * (1.0 - opacity);
    opacity = opacity + u.color_back.a * (1.0 - opacity);

    if (u.flags.w > 0.5) {
        color = color + (1.0 / 256.0) * (fract(sin(dot(0.014 * frag_px, vec2<f32>(12.9898, 78.233))) * 43758.5453123) - 0.5);
    }
    return vec4<f32>(color, opacity);
}

// ─── Geometry ───────────────────────────────────────────────────────────────

fn sd_rr(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let rr = clamp(r, 0.0, max(min(half_size.x, half_size.y), 0.0));
    let q = abs(p) - half_size + vec2<f32>(rr);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - rr;
}

fn cover(sd: f32) -> f32 {
    return clamp(0.5 - sd * u.metrics.z, 0.0, 1.0);
}

fn box_sd(p: vec2<f32>) -> f32 {
    let half_size = 0.5 * u.metrics.xy;
    return sd_rr(p - half_size, half_size, u.shape.x);
}

fn inset_sd(p: vec2<f32>, inset: f32) -> f32 {
    let half_size = 0.5 * u.metrics.xy;
    return sd_rr(p - half_size, max(half_size - vec2<f32>(inset), vec2<f32>(0.0)), max(u.shape.x - inset, 0.0));
}

fn band_cover(p: vec2<f32>, inset: f32) -> f32 {
    return cover(box_sd(p)) * (1.0 - cover(inset_sd(p, inset)));
}

// premultiplied `over`
fn over(top: vec4<f32>, bottom: vec4<f32>) -> vec4<f32> {
    return top + bottom * (1.0 - top.a);
}

fn straight_to_premul(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(c.rgb * c.a, c.a);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn finish(c: vec4<f32>) -> vec4<f32> {
    if (u.flags.z > 0.5 && c.a > 0.0) {
        let straight = clamp(c.rgb / c.a, vec3<f32>(0.0), vec3<f32>(1.0));
        return vec4<f32>(srgb_to_linear(straight) * c.a, c.a);
    }
    return c;
}

// ─── Layers ─────────────────────────────────────────────────────────────────

fn layer_ring(p: vec2<f32>, frag_px: vec2<f32>) -> vec4<f32> {
    let sd_o = box_sd(p);
    let co = cover(sd_o);
    let a = co * (1.0 - cover(inset_sd(p, u.shape.y)));
    var out = vec4<f32>(0.0);
    if (a > 0.0) {
        let s = u.sheet.xy + p * u.sheet.zw;
        let m = material(s, frag_px);
        out = m * (u.sizing2.w * a);
    }
    if (u.flags.x > 0.5 && u.circ.a > 0.0) {
        let outer = max(0.0, cover(sd_o - 1.0) - co);
        out = over(straight_to_premul(vec4<f32>(u.circ.rgb, u.circ.a * outer)), out);
    }
    return out;
}

fn sprite_alpha(tex: texture_2d<f32>, p: vec2<f32>, pos: vec2<f32>, tangent: f32, dims: vec4<f32>) -> f32 {
    let d = p - pos;
    let c = cos(tangent);
    let s = sin(tangent);
    let q = vec2<f32>(c * d.x + s * d.y, -s * d.x + c * d.y);
    let st = (q + dims.zw) / dims.xy;
    if (st.x < 0.0 || st.y < 0.0 || st.x > 1.0 || st.y > 1.0) {
        return 0.0;
    }
    return textureSampleLevel(tex, samp, st, 0.0).r;
}

fn layer_glow(p: vec2<f32>) -> vec4<f32> {
    if (u.glow_misc.z < 0.5) {
        return vec4<f32>(0.0);
    }
    let co = cover(box_sd(p));
    let ci = cover(inset_sd(p, u.shape.w));
    let m = (1.0 - co) * 0.5 + co * (1.0 - ci);
    let ah = sprite_alpha(halo_tex, p, u.halo.xy, u.halo.z, u.halo_sprite) * u.halo.w * m;
    let ae = sprite_alpha(extra_tex, p, u.extra.xy, u.extra.z, u.extra_sprite) * u.extra.w * m;
    let a = ae + ah * (1.0 - ae);
    if (a <= 0.0) {
        return vec4<f32>(0.0);
    }
    let rgb = u.extra_tint.rgb * ae + u.halo_tint.rgb * ah * (1.0 - ae);
    var straight = rgb / a;
    // CSS `filter: saturate(s) brightness(b)` on the light-theme glow group.
    let sat = u.glow_misc.x;
    if (sat != 1.0 || u.glow_misc.y != 1.0) {
        let l = dot(straight, vec3<f32>(0.213, 0.715, 0.072));
        straight = clamp(vec3<f32>(l) + (straight - vec3<f32>(l)) * sat, vec3<f32>(0.0), vec3<f32>(1.0));
        straight = clamp(straight * u.glow_misc.y, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let alpha = a * u.halo_tint.w * u.extra_tint.w;
    return vec4<f32>(straight * alpha, alpha);
}

fn rim_raw(p: vec2<f32>, ring: f32, offset_y: f32) -> f32 {
    return max(0.0, band_cover(p, ring) - band_cover(p - vec2<f32>(0.0, offset_y), ring));
}

fn layer_rim(p: vec2<f32>) -> vec4<f32> {
    let ring = u.shape.y;
    var out = vec4<f32>(0.0);
    if (u.rim.w > 0.5) {
        let a0 = band_cover(p, ring);
        if (a0 > 0.0) {
            let sigma = u.rim.y * u.metrics.z;
            var blurred = rim_raw(p, ring, u.rim.x);
            if (sigma > 0.05) {
                let radius = i32(min(ceil(3.0 * sigma), 3.0));
                var acc = 0.0;
                var wsum = 0.0;
                for (var j = -radius; j <= radius; j = j + 1) {
                    for (var i = -radius; i <= radius; i = i + 1) {
                        let dpx = vec2<f32>(f32(i), f32(j));
                        let w = exp(-dot(dpx, dpx) / (2.0 * sigma * sigma));
                        acc = acc + w * rim_raw(p + dpx / u.metrics.z, ring, u.rim.x);
                        wsum = wsum + w;
                    }
                }
                blurred = acc / wsum;
            }
            let a = min(1.0, blurred * a0 * u.rim.z);
            out = vec4<f32>(u.rim_color.rgb * a, a);
        }
    }
    let after = band_cover(p, u.shape.z) * u.hair.a;
    out = over(vec4<f32>(u.hair.rgb * after, after), out);
    return out;
}

// Abramowitz–Stegun 7.1.26 erf.
fn erf_approx(x: f32) -> f32 {
    let s = sign(x);
    let a = abs(x);
    let t = 1.0 / (1.0 + 0.3275911 * a);
    let y = 1.0 - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t + 0.254829592) * t * exp(-a * a);
    return s * y;
}

fn phi(x: f32) -> f32 {
    return 0.5 * (1.0 + erf_approx(x * 0.70710678118));
}

fn layer_surface(p: vec2<f32>) -> vec4<f32> {
    let co = cover(box_sd(p));
    if (co <= 0.0) {
        return vec4<f32>(0.0);
    }
    var out = straight_to_premul(u.surf);
    // `box-shadow: inset 0 0 50px 0`: the complement of the box, blurred.
    let sigma = max(u.shadow_misc.x, 1e-3);
    let rx = phi(p.x / sigma) - phi((p.x - u.metrics.x) / sigma);
    let ry = phi(p.y / sigma) - phi((p.y - u.metrics.y) / sigma);
    let sa = u.shadow.a * (1.0 - rx * ry);
    out = over(vec4<f32>(u.shadow.rgb * sa, sa), out);
    return out * co;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let local = in.uv * u.rect.zw;
    let p = u.rect.xy + local;
    let frag_px = local * u.metrics.z;
    let layer = i32(u.metrics.w + 0.5);
    var out = vec4<f32>(0.0);
    if (layer == 0) {
        out = layer_ring(p, frag_px);
    } else if (layer == 1) {
        out = layer_glow(p);
    } else if (layer == 2) {
        out = layer_rim(p);
    } else {
        out = layer_surface(p);
    }
    return finish(out);
}
