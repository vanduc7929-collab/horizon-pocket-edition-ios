// Grand Line 0.4.0 — 13-tap Jimenez bloom downsample with soft-knee threshold on first pass
// (ported 1:1 from assets/devilfruits/shaders/core/vfx_screen.vsh + vfx_bloom_down.fsh).

struct VfxBloomDownUniform {
    // xy = SrcTexel, z = Threshold, w = FirstPass (> 0.5 applies soft-knee threshold)
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxBloomDownUniform;
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

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(source_tex, source_smp, uv + vec2<f32>(x, y) * texel, 0.0).rgb;
}

@fragment
fn fs_main(in: ScreenOut) -> @location(0) vec4<f32> {
    let texel = u.params.xy;
    let threshold = u.params.z;
    let first_pass = u.params.w;

    let a = tap(in.uv, texel, -2.0,  2.0);
    let b = tap(in.uv, texel,  0.0,  2.0);
    let c = tap(in.uv, texel,  2.0,  2.0);
    let d = tap(in.uv, texel, -2.0,  0.0);
    let e = tap(in.uv, texel,  0.0,  0.0);
    let f = tap(in.uv, texel,  2.0,  0.0);
    let g = tap(in.uv, texel, -2.0, -2.0);
    let h = tap(in.uv, texel,  0.0, -2.0);
    let i = tap(in.uv, texel,  2.0, -2.0);
    let j = tap(in.uv, texel, -1.0,  1.0);
    let k = tap(in.uv, texel,  1.0,  1.0);
    let l = tap(in.uv, texel, -1.0, -1.0);
    let m = tap(in.uv, texel,  1.0, -1.0);

    var col = e * 0.125
            + (a + c + g + i) * 0.03125
            + (b + d + f + h) * 0.0625
            + (j + k + l + m) * 0.125;

    if (first_pass > 0.5) {
        let br = max(col.r, max(col.g, col.b));
        let knee = threshold * 0.5 + 1.0e-4;
        var soft = clamp(br - threshold + knee, 0.0, 2.0 * knee);
        soft = soft * soft / (4.0 * knee);
        col *= max(soft, br - threshold) / max(br, 1.0e-4);
    }

    return vec4<f32>(min(col, vec3<f32>(64.0)), 1.0);
}
