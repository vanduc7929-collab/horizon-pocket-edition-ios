// Grand Line 0.4.0 — Half-resolution volumetric smoke upsample composite
// (ported 1:1 from assets/devilfruits/shaders/core/vfx_screen.vsh + vfx_smoke_composite.fsh).
// 5-tap tent upsample of the half-res Rgba16Float smoke volume buffer over the scene (premultiplied).

struct VfxSmokeCompositeUniform {
    // xy = SrcTexel (1.0 / half_width, 1.0 / half_height), zw = reserved
    src_texel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxSmokeCompositeUniform;
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

@fragment
fn fs_main(in: ScreenOut) -> @location(0) vec4<f32> {
    let texel = u.src_texel.xy;
    var c = textureSampleLevel(source_tex, source_smp, in.uv, 0.0) * 0.5;
    c += textureSampleLevel(source_tex, source_smp, in.uv + vec2<f32>( 0.5,  0.5) * texel, 0.0) * 0.125;
    c += textureSampleLevel(source_tex, source_smp, in.uv + vec2<f32>(-0.5,  0.5) * texel, 0.0) * 0.125;
    c += textureSampleLevel(source_tex, source_smp, in.uv + vec2<f32>( 0.5, -0.5) * texel, 0.0) * 0.125;
    c += textureSampleLevel(source_tex, source_smp, in.uv + vec2<f32>(-0.5, -0.5) * texel, 0.0) * 0.125;
    if (c.a < 0.002) {
        discard;
    }
    return c;
}
