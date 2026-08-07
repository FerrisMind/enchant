// Port of cobe `marker.glslx` with soft radial falloff (WebGL `antialias` look).

struct MarkerUniforms {
    phi: f32,
    theta: f32,
    resolution: vec2<f32>,
    scale: f32,
    marker_elevation: f32,
    offset: vec2<f32>,
    marker_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: MarkerUniforms;

struct MarkerInstance {
    @location(1) marker_pos: vec3<f32>,
    @location(2) marker_size: f32,
    @location(3) marker_color: vec3<f32>,
    @location(4) has_color: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) marker_color: vec3<f32>,
    @location(2) has_color: f32,
    @location(3) marker_size: f32,
}

@vertex
fn vs_marker(
    @builtin(vertex_index) vi: u32,
    instance: MarkerInstance,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    );
    let a_position = corners[vi];

    let cx = cos(u.theta);
    let sx = sin(u.theta);
    let cy = cos(u.phi);
    let sy = sin(u.phi);

    let p = instance.marker_pos * (0.8 + u.marker_elevation);
    let rp = vec3(
        cy * p.x + sy * p.z,
        sy * sx * p.x + cx * p.y - cy * sx * p.z,
        -sy * cx * p.x + sx * p.y + cy * cx * p.z,
    );

    var out: VertexOutput;
    out.marker_color = instance.marker_color;
    out.has_color = instance.has_color;
    out.uv = a_position;
    out.marker_size = instance.marker_size;

    if (rp.z < 0.0 && length(rp.xy) < 0.8) {
        out.clip_position = vec4(2.0, 2.0, 0.0, 1.0);
        return out;
    }

    let ia = u.resolution.y / u.resolution.x;
    let pos = (rp.xy + a_position * instance.marker_size * 2.0) * vec2(ia, 1.0) * u.scale
        + u.offset * vec2(1.0, -1.0) * u.scale / u.resolution;
    out.clip_position = vec4(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_marker(input: VertexOutput) -> @location(0) vec4<f32> {
    let d = length(input.uv);

    // Tighter halo than before — large `glow_outer` made markers look detached from the sphere.
    let glow_outer = input.marker_size * 1.75;
    if (d > glow_outer) {
        discard;
    }

    let core_r = min(0.06, glow_outer * 0.45);
    let core = 1.0 - smoothstep(0.0, core_r, d);
    let t = (d - core_r) / max(glow_outer - core_r, 0.0001);
    let halo = (1.0 - clamp(t, 0.0, 1.0)) * (1.0 - clamp(t, 0.0, 1.0)) * 0.7;
    let alpha = clamp(core + halo * (1.0 - core * 0.35), 0.0, 1.0);
    if (alpha <= 0.004) {
        discard;
    }

    let col = select(u.marker_color.xyz, input.marker_color, input.has_color > 0.5);
    return vec4(col * alpha, alpha);
}
