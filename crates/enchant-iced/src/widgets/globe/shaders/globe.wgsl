// Port of cobe `globe.frag.glslx` + fullscreen quad vertex shader.

struct GlobeUniforms {
    resolution: vec2<f32>,
    offset: vec2<f32>,
    rotation: vec2<f32>,
    dots: f32,
    scale: f32,
    map_base_brightness: f32,
    padding0: f32,
    _vec4_align: vec2<f32>,
    base_color: vec4<f32>,
    glow_color: vec4<f32>,
    render_params: vec4<f32>,
    _struct_pad: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: GlobeUniforms;
@group(0) @binding(1) var map_texture: texture_2d<f32>;
@group(0) @binding(2) var map_sampler: sampler;

const SQRT5: f32 = 2.236068;
const PI: f32 = 3.141593;
const K_TAU: f32 = 6.283185;
const K_PHI: f32 = 1.618034;
const R: f32 = 0.8;

fn rotate(theta: f32, phi: f32) -> mat3x3<f32> {
    let cx = cos(theta);
    let cy = cos(phi);
    let sx = sin(theta);
    let sy = sin(phi);
    return mat3x3<f32>(
        vec3(cy, sy * sx, -sy * cx),
        vec3(0.0, cx, sx),
        vec3(sy, cy * -sx, cy * cx),
    );
}

fn nearest_fibonacci_lattice(p: vec3<f32>, by_dots: f32, dots: f32) -> vec4<f32> {
    var q = vec3(p.x, p.z, p.y);

    let k = max(2.0, floor(log2(SQRT5 * dots * PI * (1.0 - q.z * q.z)) * 0.72021));

    var f = floor(pow(K_PHI, k) / SQRT5 * vec2(1.0, K_PHI) + 0.5);
    let br1 = fract((f + 1.0) * (K_PHI - 1.0)) * K_TAU - 3.883222;
    let br2 = -2.0 * f;
    let sp = vec2(atan2(q.y, q.x), q.z - 1.0);
    let denom = br1.x * br2.y - br2.x * br1.y;
    var c = floor(vec2(
        br2.y * sp.x - br1.y * (sp.y * dots + 1.0),
        -br2.x * sp.x + br1.x * (sp.y * dots + 1.0),
    ) / denom);

    var mindist = PI;
    var minip = vec3(0.0);

    for (var s = 0.0; s < 4.0; s += 1.0) {
        let o = vec2(fract(s * 0.5) * 2.0, floor(s * 0.5));
        let idx = dot(f, c + o);
        if (idx > dots) {
            continue;
        }

        var a = idx;
        var b = 0.0;
        if (a >= 16384.0) { a -= 16384.0; b += 0.868872; }
        if (a >= 8192.0) { a -= 8192.0; b += 0.934436; }
        if (a >= 4096.0) { a -= 4096.0; b += 0.467218; }
        if (a >= 2048.0) { a -= 2048.0; b += 0.733609; }
        if (a >= 1024.0) { a -= 1024.0; b += 0.866804; }
        if (a >= 512.0) { a -= 512.0; b += 0.433402; }
        if (a >= 256.0) { a -= 256.0; b += 0.216701; }
        if (a >= 128.0) { a -= 128.0; b += 0.108351; }
        if (a >= 64.0) { a -= 64.0; b += 0.554175; }
        if (a >= 32.0) { a -= 32.0; b += 0.777088; }
        if (a >= 16.0) { a -= 16.0; b += 0.888544; }
        if (a >= 8.0) { a -= 8.0; b += 0.944272; }
        if (a >= 4.0) { a -= 4.0; b += 0.472136; }
        if (a >= 2.0) { a -= 2.0; b += 0.236068; }
        if (a >= 1.0) { a -= 1.0; b += 0.618034; }

        let theta = fract(b) * K_TAU;
        let cosphi = 1.0 - 2.0 * idx * by_dots;
        let sinphi = sqrt(max(0.0, 1.0 - cosphi * cosphi));
        let sample = vec3(cos(theta) * sinphi, sin(theta) * sinphi, cosphi);
        let dist = length(q - sample);

        if (dist < mindist) {
            mindist = dist;
            minip = vec3(sample.x, sample.z, sample.y);
        }
    }

    return vec4(minip, mindist);
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_gl: vec2<f32>,
}

@vertex
fn vs_globe(@builtin(vertex_index) vi: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    );
    let ndc = corners[vi];
    var out: VertexOutput;
    out.clip_position = vec4(ndc, 0.0, 1.0);
    // Iced widget coords: origin top-left (matches COBE `gl_FragCoord` uv math without Y flip).
    let local = (ndc * 0.5 + 0.5) * u.resolution;
    out.local_gl = local;
    return out;
}

@fragment
fn fs_globe(input: VertexOutput) -> @location(0) vec4<f32> {
    let by_dots = 1.0 / u.dots;
    let inv_resolution = 1.0 / u.resolution;

    var uv = ((input.local_gl * inv_resolution) * 2.0 - 1.0) / u.scale
        - u.offset * vec2(1.0, -1.0) * inv_resolution;
    uv.x *= u.resolution.x * inv_resolution.y;

    let l = dot(uv, uv);
    var glow_factor = 0.0;
    var color = vec4(0.0);

    let outer_glow = u.padding0;

    if (l <= R * R) {
        let dis = sqrt(max(0.0, R * R - l));
        var layer = vec4(0.0);
        let p = normalize(vec3(uv, dis));
        let rot = rotate(u.rotation.y, u.rotation.x);
        let dot_nl = p.z;

        let lattice = nearest_fibonacci_lattice(p * rot, by_dots, u.dots);
        let g_p = lattice.xyz;
        let dis_lat = lattice.w;

        var g_phi = asin(clamp(g_p.y, -1.0, 1.0));
        var g_theta = acos(clamp(-g_p.x / cos(g_phi), -1.0, 1.0));
        if (g_p.z < 0.0) {
            g_theta = -g_theta;
        }

        let map_uv = vec2(((g_theta * 0.5) / PI), -(g_phi / PI + 0.5));
        let map_sample = max(textureSample(map_texture, map_sampler, map_uv).x, u.map_base_brightness);

        let sample_strength = map_sample
            * smoothstep(0.008, 0.0, dis_lat)
            * pow(dot_nl, u.render_params.y)
            * u.render_params.x;

        let color_factor = mix((1.0 - sample_strength) * pow(dot_nl, 0.4), sample_strength, u.render_params.z) + 0.1;
        let glow = pow(1.0 - dot_nl, 4.0) * u.glow_color.xyz;
        layer += vec4(u.base_color.xyz * color_factor + glow, 1.0);

        color += layer * (1.0 + u.render_params.w) * 0.5;
        glow_factor = (1.0 - l) * (1.0 - l) * smoothstep(0.0, 1.0, 0.2 / max(0.0001, l - R * R)) * outer_glow;
    } else {
        let out_d = sqrt(0.2 / max(0.0001, l - R * R));
        glow_factor = smoothstep(0.5, 1.0, out_d / (out_d + 1.0)) * outer_glow;
    }

    return color + vec4(u.glow_color.xyz * glow_factor, glow_factor);
}
