// Grand Line 0.4.0 — 9-tap tent bloom upsample, additively blended onto the next larger mip
// (ported 1:1 from assets/devilfruits/shaders/core/vfx_screen.vsh + vfx_bloom_up.fsh).

struct VfxBloomUpUniform {
    // xy = SrcTexel, z = Radius, w = Intensity
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxBloomUpUniform;
@group(0) @binding(1) var source_tex: texture_2d<f32>;
@group(0) @binding(2) var source_smp: sampler;

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

fn tap(uv: vec2<f32>, step_uv: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(source_tex, source_smp, uv + vec2<f32>(x, y) * step_uv, 0.0).rgb;
}

@fragment
fn fs_main(in: ScreenOut) -> @location(0) vec4<f32> {
    let step_uv = u.params.xy * u.params.z;
    let intensity = u.params.w;

    let col = tap(in.uv, step_uv, -1.0,  1.0)
            + 2.0 * tap(in.uv, step_uv,  0.0,  1.0)
            + tap(in.uv, step_uv,  1.0,  1.0)
            + 2.0 * tap(in.uv, step_uv, -1.0,  0.0)
            + 4.0 * tap(in.uv, step_uv,  0.0,  0.0)
            + 2.0 * tap(in.uv, step_uv,  1.0,  0.0)
            + tap(in.uv, step_uv, -1.0, -1.0)
            + 2.0 * tap(in.uv, step_uv,  0.0, -1.0)
            + tap(in.uv, step_uv,  1.0, -1.0);

    return vec4<f32>(col / 16.0 * intensity, 1.0);
}
