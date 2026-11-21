// Grand Line 0.4.0 — Cinematic VFX particle shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_particle.{vsh,fsh}).
// Flipbook frames A/B across 3 atlases (main atlas, explosion pack, pack2 via UV2.x bit 14 / UV2.y bit 15),
// heat-LUT or colour sprite shading, additive-to-premultiplied blend control, soft depth fade, scene & emissive passes.

struct VfxParticleUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // xy = TargetSize, z = Pass (0 = scene premultiplied, 1 = emissive buffer), w = SoftScale
    target_pass_soft: vec4<f32>,
    // x = MinAlpha, y = EmissiveScale, zw = reserved
    alpha_emissive: vec4<f32>,
    // x = ndc_z_scale, y = ndc_z_bias, z = reverse_z, w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxParticleUniform;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
@group(0) @binding(3) var pack_tex: texture_2d<f32>;
@group(0) @binding(4) var pack_smp: sampler;
@group(0) @binding(5) var pack2_tex: texture_2d<f32>;
@group(0) @binding(6) var pack2_smp: sampler;
@group(0) @binding(7) var lut_tex: texture_2d<f32>;
@group(0) @binding(8) var lut_smp: sampler;
@group(0) @binding(9) var depth_tex: texture_depth_2d;
@group(0) @binding(10) var depth_smp: sampler;

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
    // xy = uvA, zw = uvB
    @location(1) uv_ab: vec4<f32>,
    // x = blend, y = soft, z = add, w = heat
    @location(2) params_a: vec4<f32>,
    // x = emissive, y = palette, z = pack
    @location(3) params_b: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.uv_ab = vec4<f32>(in.uv0, vec2<f32>(in.uv1) / 32767.0);

    let blend = in.normal.x;
    let soft = in.normal.y * 4.0;
    let add = in.normal.z;
    let hx = in.uv2.x;
    let second = hx < -8192 || (hx >= 0 && (hx & 16384) != 0);
    let heat = select(f32(hx & 16383) / 4096.0, -1.0, hx < 0);
    let emissive = f32(in.uv2.y & 4095) / 256.0;
    let palette = f32((in.uv2.y >> 12u) & 7);
    let pack = select(f32((in.uv2.y >> 15u) & 1), 2.0, second);

    out.params_a = vec4<f32>(blend, soft, add, heat);
    out.params_b = vec3<f32>(emissive, palette, pack);
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
    let uv_a = in.uv_ab.xy;
    let uv_b = in.uv_ab.zw;
    let blend = in.params_a.x;
    let soft = in.params_a.y;
    let add = in.params_a.z;
    let heat_in = in.params_a.w;
    let emissive = in.params_b.x;
    let palette = in.params_b.y;
    let pack = in.params_b.z;

    let target_size = u.target_pass_soft.xy;
    let pass_flag = u.target_pass_soft.z;
    let soft_scale = u.target_pass_soft.w;
    let min_alpha = u.alpha_emissive.x;
    let emissive_scale = u.alpha_emissive.y;

    var t: vec4<f32>;
    if (pack > 1.5) {
        t = mix(
            textureSampleLevel(pack2_tex, pack2_smp, uv_a, 0.0),
            textureSampleLevel(pack2_tex, pack2_smp, uv_b, 0.0),
            blend,
        );
    } else if (pack > 0.5) {
        t = mix(
            textureSampleLevel(pack_tex, pack_smp, uv_a, 0.0),
            textureSampleLevel(pack_tex, pack_smp, uv_b, 0.0),
            blend,
        );
    } else {
        t = mix(
            textureSampleLevel(atlas_tex, atlas_smp, uv_a, 0.0),
            textureSampleLevel(atlas_tex, atlas_smp, uv_b, 0.0),
            blend,
        );
    }

    var cover = t.a * in.color.a;
    var rgb: vec3<f32>;
    var occl: f32;
    if (heat_in >= 0.0) {
        let heat = clamp(t.r * heat_in, 0.0, 1.0);
        let lut_uv = vec2<f32>(heat * (255.0 / 256.0) + 0.5 / 256.0, (palette + 0.5) / 8.0);
        let lut = textureSampleLevel(lut_tex, lut_smp, lut_uv, 0.0);
        rgb = lut.rgb * in.color.rgb * cover;
        occl = lut.a;
    } else {
        rgb = t.rgb * in.color.rgb * in.color.a;
        occl = 1.0;
    }
    occl *= 1.0 - add;
    if (cover <= 0.002) {
        discard;
    }

    let screen_uv = in.clip.xy / target_size;
    let self_d = linear_depth(in.clip.z);
    let scene_d = linear_depth(textureSampleLevel(depth_tex, depth_smp, screen_uv, 0));

    if (pass_flag > 0.5 && self_d > scene_d + 0.05) {
        discard;
    }
    if (soft_scale > 0.0 && soft > 0.0) {
        let fade = clamp((scene_d - self_d) / (soft * soft_scale), 0.0, 1.0);
        rgb *= fade;
        cover *= fade;
    }

    if (pass_flag > 0.5) {
        return vec4<f32>(rgb * emissive * emissive_scale, cover * occl);
    }
    let alpha = max(cover * occl, min(1.0, max(rgb.r, max(rgb.g, rgb.b))) * min_alpha);
    return vec4<f32>(rgb, alpha);
}
