// Grand Line 0.4.0 — Screen-space distortion shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_distort.{vsh,fsh}).
// Writes additive RG screen-space offsets into the half-resolution Rg16Float distortion buffer.
// Modes: 0 = radial/atlas normal, 1 = scrolling heat-haze normal, 2 = glass shard, 3 = bent-air sine band.

struct VfxDistortUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // xy = TargetSize, z = Time, w = reserved
    target_time: vec4<f32>,
    // x = ndc_z_scale, y = ndc_z_bias, z = reverse_z, w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxDistortUniform;
@group(0) @binding(1) var atlas_tex: texture_2d<f32>;
@group(0) @binding(2) var atlas_smp: sampler;
@group(0) @binding(3) var heat_tex: texture_2d<f32>;
@group(0) @binding(4) var heat_smp: sampler;
@group(0) @binding(5) var depth_tex: texture_depth_2d;
@group(0) @binding(6) var depth_smp: sampler;

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
    @location(2) noise_uv: vec2<f32>,
    @location(3) mode: f32,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.uv = in.uv0;
    out.noise_uv = vec2<f32>(in.uv1) / 32767.0;
    out.mode = f32(in.uv2.x);
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
    let target_size = u.target_time.xy;
    let time = u.target_time.z;

    let t = textureSampleLevel(atlas_tex, atlas_smp, in.uv, 0.0);
    var off: vec2<f32>;
    if (in.mode < 0.5) {
        off = (t.rg * 2.0 - vec2<f32>(1.0)) * t.a;
    } else if (in.mode < 1.5) {
        let n = textureSampleLevel(heat_tex, heat_smp, in.noise_uv * 4.0 + vec2<f32>(0.0, -time * 0.035), 0.0).rg * 2.0 - vec2<f32>(1.0);
        off = n * t.a;
    } else {
        // 2: glass shard (offset rides on vertex RG), 3: bent-air band softened across V 0..1.
        let mask = select(sin(clamp(in.uv.y, 0.0, 1.0) * 3.14159), 1.0, in.mode < 2.5);
        off = (in.color.rg * 2.0 - vec2<f32>(1.0)) * mask;
    }

    let screen_uv = in.clip.xy / target_size;
    let self_d = linear_depth(in.clip.z);
    let scene_d = linear_depth(textureSampleLevel(depth_tex, depth_smp, screen_uv, 0));
    if (self_d > scene_d + 0.1) {
        discard;
    }

    return vec4<f32>(off * in.color.a * 0.04, 0.0, 1.0);
}
