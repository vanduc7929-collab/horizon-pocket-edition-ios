// Grand Line 0.4.0 — Yomi Yomi Soul King body shader (ported 1:1 from assets/devilfruits/shaders/core/yomi_soul_body.{vsh,fsh}).
// 3D domain-warped fBm soul fire in the ghost's local frame (Origin, Yaw), shaped by painted luminance,
// dark hollow eye sockets / maw with ember lip, and ragged flickering silhouette discard.

struct YomiSoulBodyUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    color_modulator: vec4<f32>,
    fog_color: vec4<f32>,
    // x = FogStart, y = FogEnd, z = FogShape (0 = sphere, 1 = cylinder), w = reserved
    fog_params: vec4<f32>,
    // xyz = Origin (camera-relative ghost origin), w = Yaw (radians)
    origin_yaw: vec4<f32>,
    // x = Time, y = Burn, zw = reserved
    time_burn: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: YomiSoulBodyUniform;
@group(0) @binding(1) var sampler0_tex: texture_2d<f32>;
@group(0) @binding(2) var sampler0_smp: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<i32>,
    @location(4) uv2: vec2<i32>,
    @location(5) normal: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) vertex_distance: f32,
    @location(1) tex_coord0: vec2<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) world_normal: vec3<f32>,
};

fn fog_distance(pos: vec3<f32>, shape: f32) -> f32 {
    if (shape < 0.5) {
        return length(pos);
    }
    return max(length(pos.xz), abs(pos.y));
}

fn linear_fog(in_color: vec4<f32>, dist: f32, fog_start: f32, fog_end: f32, fog_color: vec4<f32>) -> vec4<f32> {
    if (dist <= fog_start) {
        return in_color;
    }
    let f = select(
        (dist - fog_start) / max(fog_end - fog_start, 1.0e-5),
        1.0,
        dist >= fog_end,
    );
    return vec4<f32>(mix(in_color.rgb, fog_color.rgb, f * fog_color.a), in_color.a);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.vertex_distance = fog_distance(in.position, u.fog_params.z);
    out.tex_coord0 = in.uv0;
    out.world_pos = in.position;
    out.world_normal = in.normal.xyz;
    return out;
}

fn hash3(p_in: vec3<f32>) -> f32 {
    var p = fract(p_in * 0.3183099 + vec3<f32>(0.1));
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

fn noise3(x: vec3<f32>) -> f32 {
    let i = floor(x);
    var f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), f.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), f.x),
            f.y,
        ),
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), f.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), f.x),
            f.y,
        ),
        f.z,
    );
}

fn fbm3(p_in: vec3<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i = i + 1) {
        v += a * noise3(p);
        p = p * 2.03 + vec3<f32>(1.7, 9.2, 3.1);
        a *= 0.5;
    }
    return v;
}

// Soul-fire ramp (LUT row SOUL_FIRE), a touch brighter: teal soot, deep green, green, neon, lime, lemon white.
fn soul_ramp(h_in: f32) -> vec3<f32> {
    let h = clamp(h_in, 0.0, 1.0);
    var c = mix(vec3<f32>(0.02, 0.10, 0.07), vec3<f32>(0.05, 0.40, 0.12), smoothstep(0.0, 0.3, h));
    c = mix(c, vec3<f32>(0.20, 0.85, 0.20), smoothstep(0.25, 0.5, h));
    c = mix(c, vec3<f32>(0.49, 1.00, 0.23), smoothstep(0.45, 0.7, h));
    c = mix(c, vec3<f32>(0.80, 1.00, 0.45), smoothstep(0.68, 0.88, h));
    c = mix(c, vec3<f32>(0.98, 1.00, 0.86), smoothstep(0.86, 1.0, h));
    return c;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex = textureSample(sampler0_tex, sampler0_smp, in.tex_coord0);
    if (tex.a < 0.1) {
        discard;
    }

    let origin = u.origin_yaw.xyz;
    let yaw = u.origin_yaw.w;
    let time = u.time_burn.x;
    let burn = u.time_burn.y;

    // Ghost's own frame (blocks): rotated with it so the fire stays locked to its body.
    var p = in.world_pos - origin;
    let cy = cos(yaw);
    let sy = sin(yaw);
    p = vec3<f32>(cy * p.x + sy * p.z, p.y, -sy * p.x + cy * p.z);

    let t = time * 0.05;
    let q = vec3<f32>(p.x * 2.6, p.y * 1.25 - t * 3.0, p.z * 2.6);
    let warp = vec3<f32>(
        fbm3(q + vec3<f32>(0.0, t * 0.7, 0.0)),
        fbm3(q + vec3<f32>(5.2, 1.3 - t * 0.5, 2.8)),
        0.0,
    ) - vec3<f32>(0.5);

    let flame = clamp((fbm3(q + warp * 1.8) - 0.3) / 0.4, 0.0, 1.0);
    let tongue = pow(flame, 2.0);
    let fine = noise3(q * 3.1 + vec3<f32>(0.0, -t * 3.5, 0.0));

    let lum = dot(tex.rgb, vec3<f32>(0.3, 0.59, 0.11));
    let dark = 1.0 - smoothstep(0.12, 0.165, lum);

    let n = normalize(in.world_normal);
    let v = normalize(-in.world_pos);
    var rim = 1.0 - abs(dot(n, v));
    rim = rim * rim;

    var heat = 0.46 + tongue * 0.6 + fine * 0.1 + (lum - 0.45) * 0.06 + rim * 0.25;
    heat *= burn;

    // Silhouette burns ragged: grazing faces flicker away where the flame is thin.
    if (rim > 0.55 && flame * 0.6 + fine * 0.3 < (rim - 0.55) * 1.9 + 0.2) {
        discard;
    }

    var col = soul_ramp(heat);
    // Thin ember lip round the dark hollows, then the hollow itself black.
    let lip = smoothstep(0.12, 0.15, lum) * (1.0 - smoothstep(0.15, 0.2, lum));
    col += vec3<f32>(0.25, 0.6, 0.15) * lip * 0.5;
    col = mix(col, vec3<f32>(0.006, 0.02, 0.012), dark);

    let color = vec4<f32>(col, 1.0) * u.color_modulator;
    return linear_fog(color, in.vertex_distance, u.fog_params.x, u.fog_params.y, u.fog_color);
}
