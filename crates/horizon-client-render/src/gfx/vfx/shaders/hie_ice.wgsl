// Grand Line 0.4.0 — Hie Hie glacier ice shader (ported 1:1 from assets/devilfruits/shaders/core/hie_ice.{vsh,fsh}).
// Normal-bent scene refraction (or lite blended fallback), Beer-Lambert depth absorption, dual-layer parallax
// fractures/bubbles, top-face rime, Fresnel rim, sun specular facets, view-dependent glints, growth & melt fronts.

struct HieIceUniform {
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
    // xy = ScreenSize, z = Refract (1.0 refract / 0.0 lite blended), w = reserved
    screen_refract: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: HieIceUniform;
@group(0) @binding(1) var sampler0_tex: texture_2d<f32>;
@group(0) @binding(2) var sampler0_smp: sampler;
@group(0) @binding(3) var sampler2_tex: texture_2d<f32>;
@group(0) @binding(4) var scene_tex: texture_2d<f32>;
@group(0) @binding(5) var scene_smp: sampler;
@group(0) @binding(6) var detail_tex: texture_2d<f32>;
@group(0) @binding(7) var detail_smp: sampler;

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
    @location(8) ice_params: vec2<f32>,
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
    out.vertex_color = minecraft_mix_light(u.light0_dir.xyz, u.light1_dir.xyz, in.normal.xyz, in.color);
    out.raw_color = in.color;
    out.light_map_color = load_clamped(sampler2_tex, in.uv2 / vec2<i32>(16, 16));
    out.tex_coord0 = in.uv0;
    out.world_pos = in.position;
    out.world_normal = in.normal.xyz;
    let mv3 = mat3x3<f32>(u.model_view[0].xyz, u.model_view[1].xyz, u.model_view[2].xyz);
    out.view_normal = mv3 * in.normal.xyz;
    out.ice_params = vec2<f32>(in.uv1) / 15.0;
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

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let tex = textureSample(sampler0_tex, sampler0_smp, in.tex_coord0);
    if (tex.a < 0.02) {
        discard;
    }

    let grow = in.raw_color.a;
    let melt = in.ice_params.x;
    let core = in.ice_params.y;
    let game_time = u.fog_params.w;
    let screen_size = u.screen_refract.xy;
    let refract_on = u.screen_refract.z;

    // Growth: crystallising in along a noise front fixed to the model's own texels.
    let front = vnoise(in.tex_coord0 * 48.0) * 0.6 + vnoise(in.tex_coord0 * 160.0) * 0.4;
    if (front > grow + 0.001) {
        discard;
    }
    let grow_edge = 1.0 - smoothstep(0.0, 0.07, grow - front);

    let n_vec = normalize(in.world_normal);
    let v_vec = normalize(-in.world_pos);
    let ndv = clamp(abs(dot(n_vec, v_vec)), 0.0, 1.0);
    let fres = pow(1.0 - ndv, 3.0);

    // Path through the ice: painted thickness, longer at grazing angles, thinner as it melts.
    let thick = tex.a * (0.55 + 1.1 * fres) * (1.0 - 0.6 * melt);
    let absorb = exp(-thick * vec3<f32>(2.4, 1.15, 0.45) * 1.6);

    // Inside: fractures and bubbles at two depths, shifted against the view (parallax).
    let vn = normalize(in.view_normal);
    let base_uv = in.tex_coord0 * 3.0;
    let par = vn.xy * 0.06;
    let near_d = textureSampleLevel(detail_tex, detail_smp, base_uv + par * 0.4, 0.0);
    let deep_d = textureSampleLevel(detail_tex, detail_smp, base_uv * 0.73 + vec2<f32>(0.37, 0.11) - par * 1.4, 0.0);

    let light = in.light_map_color.rgb;
    let tint = in.raw_color.rgb * tex.rgb;
    var behind = vec3<f32>(0.0);
    if (refract_on > 0.5) {
        // wgpu/Metal @builtin(position).xy is Y-down top-left; vn.y points up in view space so invert Y offset.
        let suv = in.clip.xy / screen_size;
        let bend = vec2<f32>(vn.x, -vn.y) * (0.018 + 0.035 * thick) + vec2<f32>((deep_d.a - 0.5) * 0.012);
        behind = textureSampleLevel(scene_tex, scene_smp, clamp(suv - bend, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
    }

    // What the ice lets through, absorbed, plus its own body colour scattered inside.
    let body = tint * light * (0.35 + 0.35 * (1.0 - ndv));
    var col = behind * absorb * 0.9 + body * mix(vec3<f32>(1.0), absorb, 0.5);

    // Inner core glowing out through the ice (subsurface): strongest head-on.
    col += core * vec3<f32>(0.35, 0.75, 1.0) * (0.4 + 0.6 * ndv) * (1.0 - 0.5 * thick);

    // Fractures & bubbles.
    col += near_d.r * vec3<f32>(0.75, 0.92, 1.0) * 0.55 * light;
    col += deep_d.r * vec3<f32>(0.25, 0.55, 1.0) * 0.35 * (0.6 + core);
    col += near_d.g * vec3<f32>(0.8, 0.9, 1.0) * 0.18;

    // Rime on upward-facing surfaces.
    let up = smoothstep(0.35, 0.9, n_vec.y);
    let rime = up * (0.35 + 0.65 * near_d.b) * (1.0 - melt);
    col = mix(col, vec3<f32>(0.86, 0.95, 1.0) * light, clamp(rime * 0.8, 0.0, 1.0));

    // Fresnel rim, pale cyan to white.
    col += fres * vec3<f32>(0.62, 0.86, 1.0) * 0.85 * light;

    // Sun facets: sharp highlight off each flat face (fixed sun high in south-west).
    let sun = normalize(vec3<f32>(-0.35, 0.85, 0.4));
    let h = normalize(sun + v_vec);
    let spec = pow(clamp(dot(n_vec, h), 0.0, 1.0), 64.0) * (0.6 + 0.4 * light.r);
    col += spec * vec3<f32>(1.0, 0.98, 0.92) * 1.4;

    // Glints: rare texels flare as view or time moves over them.
    let cell = floor(in.tex_coord0 * 96.0);
    let g = hash2(cell + vec2<f32>(floor(game_time * 2400.0 + dot(v_vec, vec3<f32>(13.1, 7.7, 3.3)) * 3.0)));
    let glint = smoothstep(0.985, 1.0, g) * (0.4 + 0.6 * fres + spec);
    col += glint * vec3<f32>(1.0, 1.0, 1.0) * 1.5;

    // Melting: wet sheen.
    col += melt * pow(1.0 - ndv, 2.0) * vec3<f32>(0.5, 0.7, 0.9) * 0.4;

    // Frost-white front of growth.
    col = mix(col, vec3<f32>(0.92, 0.98, 1.0), grow_edge * 0.85);

    var alpha: f32;
    if (refract_on > 0.5) {
        alpha = 1.0;
    } else {
        alpha = clamp(0.05 + 0.42 * tex.a + 0.45 * fres + rime * 0.25 + grow_edge * 0.6 + glint, 0.0, 0.92);
        col = mix(col, vec3<f32>(0.16, 0.38, 0.78) * light, clamp(thick * 0.45, 0.0, 0.5));
    }

    let out_color = vec4<f32>(col, alpha) * u.color_modulator;
    return linear_fog(out_color, in.vertex_distance, u.fog_params.x, u.fog_params.y, u.fog_color);
}
