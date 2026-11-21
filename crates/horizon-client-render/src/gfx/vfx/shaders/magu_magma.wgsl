// Grand Line 0.4.0 — Magu Magu molten rock shader (ported 1:1 from assets/devilfruits/shaders/core/magu_magma.{vsh,fsh}).
// Basalt crust plates lit by world light, flowing emissive molten channels via blackbody LUT row 4,
// parallax crust cracks pulsing plate-by-plate, grazing hot rim, cooling & growth fronts.

struct MaguMagmaUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    color_modulator: vec4<f32>,
    // xyz = Light0_Direction
    light0_dir: vec4<f32>,
    // xyz = Light1_Direction
    light1_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = FogStart, y = FogEnd, z = FogShape (0 = sphere, 1 = cylinder), w = GameTime
    fog_params: vec4<f32>,
    // x = Parallax (0 on Medium, 1 on High), yzw = reserved
    magma_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: MaguMagmaUniform;
@group(0) @binding(1) var sampler0_tex: texture_2d<f32>;
@group(0) @binding(2) var sampler0_smp: sampler;
@group(0) @binding(3) var sampler2_tex: texture_2d<f32>;
@group(0) @binding(4) var emissive_tex: texture_2d<f32>;
@group(0) @binding(5) var emissive_smp: sampler;
@group(0) @binding(6) var detail_tex: texture_2d<f32>;
@group(0) @binding(7) var detail_smp: sampler;
@group(0) @binding(8) var lut_tex: texture_2d<f32>;
@group(0) @binding(9) var lut_smp: sampler;

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
    @location(2) raw_color: vec4<f32>,
    @location(3) light_map_color: vec4<f32>,
    @location(4) tex_coord0: vec2<f32>,
    @location(5) world_pos: vec3<f32>,
    @location(6) world_normal: vec3<f32>,
    @location(7) view_normal: vec3<f32>,
    @location(8) magma_params: vec2<f32>,
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
    out.vertex_color = minecraft_mix_light(u.light0_dir.xyz, u.light1_dir.xyz, in.normal.xyz, vec4<f32>(1.0));
    out.raw_color = in.color;
    out.light_map_color = load_clamped(sampler2_tex, in.uv2 / vec2<i32>(16, 16));
    out.tex_coord0 = in.uv0;
    out.world_pos = in.position;
    out.world_normal = in.normal.xyz;
    let mv3 = mat3x3<f32>(u.model_view[0].xyz, u.model_view[1].xyz, u.model_view[2].xyz);
    out.view_normal = mv3 * in.normal.xyz;
    out.magma_params = vec2<f32>(in.uv1) / 15.0;
    return out;
}

fn hash2(p_in: vec2<f32>) -> f32 {
    var p = fract(p_in * vec2<f32>(123.34, 456.21));
    p += vec2<f32>(dot(p, p + vec2<f32>(45.32, 45.32)));
    return fract(p.x * p.y);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}

fn blackbody(heat: f32) -> vec3<f32> {
    let uv = vec2<f32>(clamp(heat, 0.0, 1.0) * (255.0 / 256.0) + 0.5 / 256.0, 4.5 / 8.0);
    return textureSampleLevel(lut_tex, lut_smp, uv, 0.0).rgb;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex = textureSample(sampler0_tex, sampler0_smp, in.tex_coord0);
    if (tex.a < 0.1) {
        discard;
    }

    let grow = in.raw_color.a;
    let cool = in.magma_params.x;
    let hot = in.magma_params.y;
    let t = u.fog_params.w * 24000.0;
    let parallax = u.magma_params.x;

    // Growth: magma swells in along a noise front fixed to the model's texels.
    let front = vnoise(in.tex_coord0 * 40.0) * 0.6 + vnoise(in.tex_coord0 * 140.0) * 0.4;
    if (front > grow + 0.001) {
        discard;
    }
    let grow_edge = (1.0 - smoothstep(0.0, 0.08, grow - front)) * (1.0 - smoothstep(0.9, 0.99, grow));

    let n_vec = normalize(in.world_normal);
    let v_vec = normalize(-in.world_pos);
    let ndv = clamp(abs(dot(n_vec, v_vec)), 0.0, 1.0);
    let fres = pow(1.0 - ndv, 3.0);

    // Crust: painted texture lit by the world.
    let light = in.light_map_color.rgb * in.vertex_color.rgb;
    var crust = tex.rgb * in.raw_color.rgb * light;

    // Molten parts (emissive mask), flowing and pulsing.
    let vn = normalize(in.view_normal);
    let uv = in.tex_coord0 * 3.0;
    let flow = textureSampleLevel(detail_tex, detail_smp, uv * 0.7 + vec2<f32>(t * 0.0015, -t * 0.004), 0.0);
    let mask = textureSampleLevel(emissive_tex, emissive_smp, in.tex_coord0, 0.0).r;
    let pulse = 0.85 + 0.15 * sin(t * 0.11 + flow.a * 6.2832);
    let molten = mask * (0.62 + 0.45 * flow.a * flow.a) * pulse;

    // Cracks over the crust opening and closing plate by plate (parallax).
    let par = vn.xy * 0.035 * parallax;
    let deep = textureSampleLevel(detail_tex, detail_smp, uv - par, 0.0);
    let rhythm = 0.5 + 0.5 * sin(t * 0.045 + deep.g * 37.0);
    let crack = deep.r * smoothstep(0.35, 0.95, rhythm) * (1.0 - mask) * (1.0 - cool);
    crust *= 0.75 + 0.25 * deep.b;

    var heat = max(molten * (0.5 + 0.25 * hot), crack * 0.62) * (1.0 - 0.75 * cool);
    // Hot rim at grazing angles.
    heat = max(heat, fres * (0.25 + 0.45 * hot) * (1.0 - cool) * (0.4 + 0.6 * mask));
    // White-hot molten edge of the growth.
    heat = max(heat, grow_edge * 0.95);

    heat = min(heat, 0.9);
    let glow = blackbody(heat) * smoothstep(0.08, 0.35, heat) * (1.0 + heat * 0.3);
    var col = crust * (1.0 - smoothstep(0.2, 0.7, heat)) + glow;
    // Cooled rock is darker and duller.
    col = mix(col, col * vec3<f32>(0.55, 0.5, 0.5), cool * 0.6);

    return linear_fog(vec4<f32>(col, 1.0) * u.color_modulator, in.vertex_distance, u.fog_params.x, u.fog_params.y, u.fog_color);
}
