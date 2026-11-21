// Grand Line 0.4.0 — Texel-snapped burning dissolve shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_dissolve.{vsh,fsh}).
// Vertex alpha carries the dissolve threshold (0 = invisible, 1 = fully visible); texels burn in with a glowing ember edge.

struct VfxDissolveUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    color_modulator: vec4<f32>,
    // xyz = Light0_Direction
    light0_dir: vec4<f32>,
    // xyz = Light1_Direction
    light1_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = FogStart, y = FogEnd, z = FogShape (0 = sphere, 1 = cylinder), w = reserved
    fog_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxDissolveUniform;
@group(0) @binding(1) var sampler0_tex: texture_2d<f32>;
@group(0) @binding(2) var sampler0_smp: sampler;
@group(0) @binding(3) var sampler1_tex: texture_2d<f32>;
@group(0) @binding(4) var sampler2_tex: texture_2d<f32>;

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
    @location(1) vertex_color: vec4<f32>,
    @location(2) light_map_color: vec4<f32>,
    @location(3) overlay_color: vec4<f32>,
    @location(4) tex_coord0: vec2<f32>,
    @location(5) progress: f32,
};

const MINECRAFT_LIGHT_POWER: f32 = 0.6;
const MINECRAFT_AMBIENT_LIGHT: f32 = 0.4;

fn minecraft_mix_light(light_dir0: vec3<f32>, light_dir1: vec3<f32>, normal: vec3<f32>, color: vec4<f32>) -> vec4<f32> {
    let l0 = max(0.0, dot(light_dir0, normal));
    let l1 = max(0.0, dot(light_dir1, normal));
    let light_accum = min(1.0, (l0 + l1) * MINECRAFT_LIGHT_POWER + MINECRAFT_AMBIENT_LIGHT);
    return vec4<f32>(color.rgb * light_accum, color.a);
}

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

fn load_clamped(tex: texture_2d<f32>, coords: vec2<i32>) -> vec4<f32> {
    let dims = vec2<i32>(textureDimensions(tex, 0));
    let max_coord = max(dims - vec2<i32>(1, 1), vec2<i32>(0, 0));
    return textureLoad(tex, clamp(coords, vec2<i32>(0, 0), max_coord), 0);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.vertex_distance = fog_distance(in.position, u.fog_params.z);
    out.vertex_color = minecraft_mix_light(u.light0_dir.xyz, u.light1_dir.xyz, in.normal.xyz, vec4<f32>(in.color.rgb, 1.0));
    out.light_map_color = load_clamped(sampler2_tex, in.uv2 / vec2<i32>(16, 16));
    out.overlay_color = load_clamped(sampler1_tex, in.uv1);
    out.tex_coord0 = in.uv0;
    out.progress = in.color.a;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(sampler0_tex, sampler0_smp, in.tex_coord0);
    if (color.a < 0.1) {
        discard;
    }
    let cell = floor(in.tex_coord0 * vec2<f32>(textureDimensions(sampler0_tex, 0)));
    let n = fract(sin(dot(cell, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    if (n > in.progress) {
        discard;
    }
    color *= in.vertex_color * u.color_modulator;
    color = vec4<f32>(mix(in.overlay_color.rgb, color.rgb, in.overlay_color.a), color.a);
    color *= in.light_map_color;
    let edge = (1.0 - smoothstep(0.0, 0.16, in.progress - n)) * step(in.progress, 0.995);
    let ember = mix(vec3<f32>(0.91, 0.20, 0.12), vec3<f32>(1.0, 0.96, 0.76), edge * edge);
    color = vec4<f32>(mix(color.rgb, ember, edge), color.a);
    return linear_fog(color, in.vertex_distance, u.fog_params.x, u.fog_params.y, u.fog_color);
}
