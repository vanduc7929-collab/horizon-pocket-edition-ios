// Grand Line 0.4.0 — Cylindrical smoke tube shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_smoke_tube.{vsh,fsh}).
// Camera-facing ribbon rounded via cylindrical normal reconstruction, multi-octave flowing noise erosion,
// optical Beer-Lambert coverage, wrapped sun lighting + self-shadowing + Henyey-Greenstein rim glow, soft depth fade.

struct VfxSmokeTubeUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // xy = TargetSize, z = SoftScale, w = reserved
    target_soft: vec4<f32>,
    // xyz = SunDir
    sun_dir: vec4<f32>,
    // xyz = SunColor
    sun_color: vec4<f32>,
    // xyz = BlockColor
    block_color: vec4<f32>,
    // x = ndc_z_scale, y = ndc_z_bias, z = reverse_z, w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxSmokeTubeUniform;
@group(0) @binding(1) var noise_tex: texture_2d<f32>;
@group(0) @binding(2) var noise_smp: sampler;
@group(0) @binding(3) var depth_tex: texture_depth_2d;
@group(0) @binding(4) var depth_smp: sampler;

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
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    // x = erode, y = density, z = albedo, w = key
    @location(2) params: vec4<f32>,
    @location(3) block_light: f32,
    @location(4) side: vec3<f32>,
    @location(5) pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.pos = in.position;
    out.color = in.color;
    out.uv = in.uv0;

    let erode = f32(in.uv1.x) / 32767.0;
    let density = f32(in.uv1.y) / 32767.0;
    let albedo = f32(in.uv2.x & 255) / 255.0;
    let key = f32(in.uv2.y & 255) / 255.0;
    out.params = vec4<f32>(erode, density, albedo, key);
    out.block_light = f32((in.uv2.y >> 8u) & 127) / 127.0;
    out.side = in.normal.xyz;
    return out;
}

fn linear_depth(d: f32) -> f32 {
    let denom = (d * u.depth_params.x + u.depth_params.y) + u.proj[2][2];
    if (abs(denom) < 1.0e-7) {
        return u.depth_params.w;
    }
    return u.proj[3][2] / denom;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let erode = in.params.x;
    let density = in.params.y;
    let albedo = in.params.z;
    let v_key = in.params.w;
    let v_block = in.block_light;

    let target_size = u.target_soft.xy;
    let soft_scale = u.target_soft.z;
    let sun_dir = u.sun_dir.xyz;
    let sun_color = u.sun_color.xyz;
    let block_color = u.block_color.xyz;

    let across = in.uv.y * 2.0 - 1.0;
    let n1 = textureSampleLevel(noise_tex, noise_smp, vec2<f32>(in.uv.x * 0.31, in.uv.y * 0.42 + in.uv.x * 0.05), 0.0).r;
    let n2 = textureSampleLevel(noise_tex, noise_smp, vec2<f32>(in.uv.x * 0.83 + 0.31, in.uv.y * 0.77 - in.uv.x * 0.11), 0.0).g;
    let n3 = textureSampleLevel(noise_tex, noise_smp, vec2<f32>(in.uv.x * 1.7 + 0.57, in.uv.y * 1.1), 0.0).b;
    let n = n1 * 0.45 + n2 * 0.4 + n3 * 0.15;

    // Rim billows: cross-section pushed in and out by noise.
    let aw = across * (1.2 - 0.5 * n2) + (n1 - 0.5) * 0.3;
    let q = clamp(1.0 - aw * aw, 0.0, 1.0);
    let t = sqrt(q);

    // Density: dense in the core, falling off smoothly toward rims, lumpy along flow.
    let body = q * sqrt(q) * (0.35 + 1.1 * n);
    let keep = smoothstep(erode - 0.25, erode + 0.35, body * 0.9 + n * 0.35 - 0.12);
    var a = (1.0 - exp(-body * density * 3.4)) * keep * in.color.a;
    if (a < 0.003) {
        discard;
    }

    let z_axis = normalize(-in.pos);
    let side = normalize(in.side);
    let nrm = normalize(
        side * aw
        + z_axis * (t * 1.1 + 0.05)
        + (side * (n3 - 0.5) + vec3<f32>(0.0, n1 - 0.5, 0.0)) * 0.6,
    );
    let ndl = dot(nrm, sun_dir);
    let wrap = clamp((ndl + 0.35) / 1.35, 0.0, 1.0);
    let shade = mix(0.4, 1.0, exp(-2.4 * t * density * (1.0 - clamp(ndl, 0.0, 1.0))));

    let cosv = dot(normalize(in.pos), sun_dir);
    let g = 0.55;
    let hg = (1.0 - g * g) / pow(max(1.0 + g * g - 2.0 * g * cosv, 1.0e-3), 1.5) * 0.0796;
    let scatter = hg * 2.0 * (1.0 - t) * (1.0 - t);
    let upness = nrm.y * 0.5 + 0.5;

    // Cavities of noise are shaded (cheap AO) so body never reads as a smooth ribbon.
    let occl = mix(0.72, 1.0, n);
    let sun = sun_color * v_key * (wrap * shade + scatter * 0.3) * occl;
    let sky = in.color.rgb * (0.42 + 0.5 * upness) * mix(0.7, 1.0, shade) * occl * vec3<f32>(0.95, 0.98, 1.04);
    let fire = block_color * v_block * (0.5 + 0.5 * (1.0 - upness)) * 1.1 * occl;
    let dark = vec3<f32>(0.05, 0.055, 0.075) * (0.55 + 0.45 * upness);
    let rgb = (sun + sky + fire + dark) * albedo;

    if (soft_scale > 0.0) {
        let screen_uv = in.clip.xy / target_size;
        let self_d = linear_depth(in.clip.z);
        let scene_d = linear_depth(textureSampleLevel(depth_tex, depth_smp, screen_uv, 0));
        a *= clamp((scene_d - self_d) / (0.8 * soft_scale), 0.0, 1.0);
    }

    return vec4<f32>(rgb * a, a);
}
