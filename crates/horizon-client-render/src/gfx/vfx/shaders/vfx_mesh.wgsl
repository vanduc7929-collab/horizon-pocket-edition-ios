// Grand Line 0.4.0 — Cinematic VFX mesh & ribbon shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_mesh.{vsh,fsh}).
// Scrolling texture, LUT heat ramp or direct colour, noise dissolve with hot burn rim, Fresnel rim,
// soft depth fade, and dual-pass scene (premultiplied) / emissive bloom output.

struct VfxMeshUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // xy = TargetSize, z = Pass (0 = scene, 1 = emissive), w = SoftScale
    target_pass_soft: vec4<f32>,
    // x = MinAlpha, y = AlphaCut, z = EmissiveScale, w = reserved
    alpha_emissive: vec4<f32>,
    // x = ndc_z_scale (1.0 for wgpu [0,1], 2.0 for GL [-1,1]),
    // y = ndc_z_bias  (0.0 for wgpu [0,1], -1.0 for GL [-1,1]),
    // z = reverse_z   (1.0 for Horizon reversed-Z, 0.0 for standard-Z),
    // w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxMeshUniform;
@group(0) @binding(1) var main_tex: texture_2d<f32>;
@group(0) @binding(2) var main_smp: sampler;
@group(0) @binding(3) var noise_tex: texture_2d<f32>;
@group(0) @binding(4) var noise_smp: sampler;
@group(0) @binding(5) var lut_tex: texture_2d<f32>;
@group(0) @binding(6) var lut_smp: sampler;
@group(0) @binding(7) var depth_tex: texture_depth_2d;
@group(0) @binding(8) var depth_smp: sampler;

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
    // x = dissolve, y = fresnel, z = heat, w = emissive
    @location(2) params: vec4<f32>,
    @location(3) palette: f32,
    @location(4) pos: vec3<f32>,
    @location(5) normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.uv = in.uv0;
    let dissolve = f32(in.uv1.x) / 32767.0;
    let fresnel = f32(in.uv1.y) / 32767.0;
    let heat = f32(in.uv2.x) / 4096.0;
    let emissive = f32(in.uv2.y & 4095) / 256.0;
    out.params = vec4<f32>(dissolve, fresnel, heat, emissive);
    out.palette = f32((in.uv2.y >> 12u) & 7);
    out.pos = in.position;
    out.normal = in.normal.xyz;
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
    let dissolve = in.params.x;
    let fresnel = in.params.y;
    let heat_in = in.params.z;
    let emissive = in.params.w;

    let target_size = u.target_pass_soft.xy;
    let pass_flag = u.target_pass_soft.z;
    let soft_scale = u.target_pass_soft.w;
    let min_alpha = u.alpha_emissive.x;
    let alpha_cut = u.alpha_emissive.y;
    let emissive_scale = u.alpha_emissive.z;

    let t = textureSampleLevel(main_tex, main_smp, in.uv, 0.0);
    var cover: f32;
    var rgb: vec3<f32>;
    var occl: f32;

    if (heat_in >= 0.0) {
        let heat = clamp(t.r * heat_in, 0.0, 1.0);
        let lut_uv = vec2<f32>(heat * (255.0 / 256.0) + 0.5 / 256.0, (in.palette + 0.5) / 8.0);
        let lut = textureSampleLevel(lut_tex, lut_smp, lut_uv, 0.0);
        cover = t.a * in.color.a * smoothstep(0.0, 0.22, heat);
        rgb = lut.rgb * in.color.rgb * cover;
        occl = lut.a * 0.3;
    } else {
        cover = t.a * in.color.a;
        rgb = t.rgb * in.color.rgb * in.color.a;
        occl = 1.0;
    }

    var burn = vec3<f32>(0.0);
    if (dissolve > 0.0) {
        let n = textureSampleLevel(noise_tex, noise_smp, in.uv * 0.61 + vec2<f32>(0.37, 0.11), 0.0).r;
        let k = smoothstep(dissolve, dissolve + 0.12, n);
        cover *= k;
        rgb *= k;
        if (heat_in < 0.0) {
            // Colour surfaces (cocoons, shells) burn away with a glowing rim in their palette's hot colour.
            let rim = k * (1.0 - smoothstep(dissolve + 0.03, dissolve + 0.18, n));
            let rim_uv = vec2<f32>(0.86, (in.palette + 0.5) / 8.0);
            burn = textureSampleLevel(lut_tex, lut_smp, rim_uv, 0.0).rgb * rim * 1.6 * in.color.a;
        }
    }

    if (fresnel > 0.0) {
        let rim = 1.0 - abs(dot(normalize(in.normal), normalize(-in.pos)));
        let f = mix(1.0, rim * rim * 1.8, fresnel);
        cover *= f;
        rgb *= f;
    }

    rgb += burn;
    if (cover <= alpha_cut && dot(burn, burn) < 0.0001) {
        discard;
    }

    let screen_uv = in.clip.xy / target_size;
    let self_d = linear_depth(in.clip.z);
    let scene_d = linear_depth(textureSampleLevel(depth_tex, depth_smp, screen_uv, 0));

    if (pass_flag > 0.5 && self_d > scene_d + 0.05) {
        discard;
    }

    if (soft_scale > 0.0) {
        let fade = clamp((scene_d - self_d) / (0.35 * soft_scale), 0.0, 1.0);
        rgb *= fade;
        cover *= fade;
    }

    if (pass_flag > 0.5) {
        return vec4<f32>(rgb * emissive * emissive_scale, cover * occl);
    }

    let alpha = max(cover * occl, min(1.0, max(rgb.r, max(rgb.g, rgb.b))) * min_alpha);
    return vec4<f32>(rgb, alpha);
}
