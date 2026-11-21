// Grand Line 0.4.0 — Final cinematic post-process composite
// (ported 1:1 from assets/devilfruits/shaders/core/vfx_screen.vsh + vfx_composite.fsh).
// Applies distortion, chromatic aberration, 10-tap radial zoom blur, exponential screen-blended bloom,
// warm S-curve colour grade, desaturation, edge-weighted flash, and vignette.

struct VfxCompositeUniform {
    // xy = RadialCenter (wgpu Y-down screen UV), z = RadialStrength, w = Chromatic
    radial_chroma: vec4<f32>,
    // x = BloomIntensity, y = DistortOn (> 0.5), z = Vignette, w = Desaturate
    bloom_distort_vig_desat: vec4<f32>,
    // rgb = flash colour, a = flash strength
    flash: vec4<f32>,
    // rgb = grade tint, a = grade blend
    grade: vec4<f32>,
    // xy = InvSize (1.0 / screen_width, 1.0 / screen_height), zw = reserved
    inv_size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxCompositeUniform;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var scene_smp: sampler;
@group(0) @binding(3) var distort_tex: texture_2d<f32>;
@group(0) @binding(4) var distort_smp: sampler;
@group(0) @binding(5) var emissive_tex: texture_2d<f32>;
@group(0) @binding(6) var emissive_smp: sampler;
@group(0) @binding(7) var bloom_tex: texture_2d<f32>;
@group(0) @binding(8) var bloom_smp: sampler;

struct ScreenOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> ScreenOut {
    let p = vec2<f32>(f32((vid << 1u) & 2u), f32(vid & 2u));
    var out: ScreenOut;
    out.clip = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(p.x, 1.0 - p.y);
    return out;
}

fn scene_at(uv: vec2<f32>, ca: vec2<f32>) -> vec3<f32> {
    return vec3<f32>(
        textureSampleLevel(scene_tex, scene_smp, uv + ca, 0.0).r,
        textureSampleLevel(scene_tex, scene_smp, uv, 0.0).g,
        textureSampleLevel(scene_tex, scene_smp, uv - ca, 0.0).b,
    );
}

@fragment
fn fs_main(in: ScreenOut) -> @location(0) vec4<f32> {
    let radial_center = u.radial_chroma.xy;
    let radial_strength = u.radial_chroma.z;
    let chromatic = u.radial_chroma.w;
    let bloom_intensity = u.bloom_distort_vig_desat.x;
    let distort_on = u.bloom_distort_vig_desat.y;
    let vignette = u.bloom_distort_vig_desat.z;
    let desaturate = u.bloom_distort_vig_desat.w;
    let inv_size = u.inv_size.xy;

    let uv = in.uv;
    let off = select(
        vec2<f32>(0.0),
        textureSampleLevel(distort_tex, distort_smp, uv, 0.0).rg,
        distort_on > 0.5,
    );
    let suv = clamp(uv + off, inv_size, vec2<f32>(1.0) - inv_size);
    let dir = suv - radial_center;
    let ca = dir * chromatic;

    var col: vec3<f32>;
    if (radial_strength > 0.0005) {
        col = vec3<f32>(0.0);
        for (var i = 0; i < 10; i = i + 1) {
            let k = 1.0 - radial_strength * f32(i) / 9.0;
            col += scene_at(radial_center + dir * k, ca);
        }
        col /= 10.0;
    } else {
        col = scene_at(suv, ca);
    }

    if (bloom_intensity > 0.0) {
        let glow = (textureSampleLevel(emissive_tex, emissive_smp, suv, 0.0).rgb * 0.45
                  + textureSampleLevel(bloom_tex, bloom_smp, suv, 0.0).rgb) * bloom_intensity;
        col = col + (vec3<f32>(1.0) - col) * (vec3<f32>(1.0) - exp(-glow));
    }

    if (u.grade.a > 0.0) {
        var tinted = clamp(col * u.grade.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        tinted = mix(tinted, tinted * tinted * (vec3<f32>(3.0) - 2.0 * tinted), 0.3);
        col = mix(col, tinted, u.grade.a);
    }

    if (desaturate > 0.0) {
        col = mix(col, vec3<f32>(dot(col, vec3<f32>(0.299, 0.587, 0.114))), desaturate);
    }

    if (u.flash.a > 0.0) {
        let edge = 0.55 + 0.45 * smoothstep(0.15, 0.75, length(uv - vec2<f32>(0.5)));
        col = mix(col, u.flash.rgb, u.flash.a * edge);
    }

    if (vignette > 0.0) {
        let v = smoothstep(0.9, 0.3, length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.8)));
        col *= mix(1.0, v, vignette);
    }

    return vec4<f32>(col, 1.0);
}
