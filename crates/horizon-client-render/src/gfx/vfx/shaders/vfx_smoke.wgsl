// Grand Line 0.4.0 — Six-way relit smoke puff shader (ported 1:1 from assets/devilfruits/shaders/core/vfx_smoke.{vsh,fsh}).
// Six-way directional lightmaps (PosSampler / NegSampler), world-anchored drifting erosion noise,
// Henyey-Greenstein forward scattering, sky/ground bounce & warm firelight, soft depth fade, highlight shoulder.

struct VfxSmokeUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // xy = TargetSize, z = SoftScale, w = Time
    target_soft_time: vec4<f32>,
    // xyz = SunDir
    sun_dir: vec4<f32>,
    // xyz = SunColor
    sun_color: vec4<f32>,
    // xyz = BlockColor
    block_color: vec4<f32>,
    // xyz = CamPos (mod 512)
    cam_pos: vec4<f32>,
    // x = ndc_z_scale, y = ndc_z_bias, z = reverse_z, w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxSmokeUniform;
@group(0) @binding(1) var pos_tex: texture_2d<f32>;
@group(0) @binding(2) var pos_smp: sampler;
@group(0) @binding(3) var neg_tex: texture_2d<f32>;
@group(0) @binding(4) var neg_smp: sampler;
@group(0) @binding(5) var noise_tex: texture_2d<f32>;
@group(0) @binding(6) var noise_smp: sampler;
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
    // xy = uvA, zw = uvB
    @location(1) uv_ab: vec4<f32>,
    // x = blend, y = erode, z = soft, w = key
    @location(2) params_a: vec4<f32>,
    // x = block, y = albedo
    @location(3) params_b: vec2<f32>,
    @location(4) right: vec3<f32>,
    @location(5) up: vec3<f32>,
    @location(6) pos: vec3<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.pos = in.position;
    out.color = in.color;
    out.uv_ab = vec4<f32>(in.uv0, vec2<f32>(in.uv1) / 32767.0);

    let x = in.uv2.x;
    let blend = f32(x & 63) / 63.0;
    let erode = f32((x >> 6u) & 31) / 31.0;
    let soft = f32((x >> 11u) & 15) / 4.0;
    let y = in.uv2.y;
    let key = f32(y & 255) / 255.0;
    let block = f32((y >> 8u) & 127) / 127.0;

    out.params_a = vec4<f32>(blend, erode, soft, key);
    out.params_b = vec2<f32>(block, 1.0);

    // Only the roll's direction: whatever scale the normal arrives at.
    var roll = in.normal.xy;
    let len = length(roll);
    roll = select(vec2<f32>(1.0, 0.0), roll / len, len > 1.0e-4);
    let c = roll.x;
    let s = roll.y;

    // Camera axes in world space read directly from the view rotation rows of ModelViewMat.
    let cam_right = vec3<f32>(u.model_view[0][0], u.model_view[1][0], u.model_view[2][0]);
    let cam_up = vec3<f32>(u.model_view[0][1], u.model_view[1][1], u.model_view[2][1]);
    out.right = cam_right * c + cam_up * s;
    out.up = cam_up * c - cam_right * s;
    return out;
}

fn linear_depth(d: f32) -> f32 {
    let denom = (d * u.depth_params.x + u.depth_params.y) + u.proj[2][2];
    if (abs(denom) < 1.0e-7) {
        return u.depth_params.w;
    }
    return u.proj[3][2] / denom;
}

// Six-way directional lightmap lookup: l_dir in sprite's frame (x right, y up, z toward viewer).
fn six_way(p_tex: vec4<f32>, n_tex: vec4<f32>, l_dir: vec3<f32>) -> f32 {
    let w = l_dir * l_dir;
    let rx = select(n_tex.r, p_tex.r, l_dir.x > 0.0);
    let gy = select(n_tex.g, p_tex.g, l_dir.y > 0.0);
    let bz = select(p_tex.b, n_tex.b, l_dir.z > 0.0);
    return w.x * rx + w.y * gy + w.z * bz;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let uv_a = in.uv_ab.xy;
    let uv_b = in.uv_ab.zw;
    let blend = in.params_a.x;
    let erode = in.params_a.y;
    let soft = in.params_a.z;
    let v_key = in.params_a.w;
    let v_block = in.params_b.x;
    let v_albedo = in.params_b.y;

    let target_size = u.target_soft_time.xy;
    let soft_scale = u.target_soft_time.z;
    let time = u.target_soft_time.w;
    let sun_dir = u.sun_dir.xyz;
    let sun_color = u.sun_color.xyz;
    let block_color = u.block_color.xyz;
    let cam_pos = u.cam_pos.xyz;

    let p_sample = mix(
        textureSampleLevel(pos_tex, pos_smp, uv_a, 0.0),
        textureSampleLevel(pos_tex, pos_smp, uv_b, 0.0),
        blend,
    );
    let n_sample = mix(
        textureSampleLevel(neg_tex, neg_smp, uv_a, 0.0),
        textureSampleLevel(neg_tex, neg_smp, uv_b, 0.0),
        blend,
    );

    var cover = p_sample.a;
    if (cover < 0.004) {
        discard;
    }

    // Denser than baked optical depth: core holds, edge stays a soft falloff.
    let tau = -log(max(1.0 - cover, 1.0e-3));
    cover = 1.0 - exp(-tau * 1.3);

    // Erosion: world-fixed noise slowly drifting upward.
    let wp = in.pos + cam_pos;
    let q1 = wp.xz * 0.13 + vec2<f32>(wp.y * 0.09, time * 0.004);
    let q2 = vec2<f32>(wp.x - wp.z, wp.y) * 0.2 - vec2<f32>(0.0, time * 0.006);
    let q3 = wp.zy * 0.25 + vec2<f32>(wp.x * 0.17, time * 0.005);
    let n = textureSampleLevel(noise_tex, noise_smp, q1, 0.0).g * 0.45
          + textureSampleLevel(noise_tex, noise_smp, q2, 0.0).b * 0.25
          + textureSampleLevel(noise_tex, noise_smp, q3, 0.0).r * 0.3;

    let thick = n_sample.a;
    let keep = smoothstep(erode - 0.06, erode + 0.3, cover * 0.7 + thick * 0.25 + n * 0.62 - 0.33);
    var a = cover * keep * in.color.a;
    if (a < 0.003) {
        discard;
    }

    let z_axis = normalize(-in.pos);
    let r_axis = select(vec3<f32>(1.0, 0.0, 0.0), normalize(in.right), dot(in.right, in.right) > 1.0e-6);
    let u_axis = select(vec3<f32>(0.0, 1.0, 0.0), normalize(in.up), dot(in.up, in.up) > 1.0e-6);

    let ls = vec3<f32>(dot(sun_dir, r_axis), dot(sun_dir, u_axis), dot(sun_dir, z_axis));
    let us = vec3<f32>(r_axis.y, u_axis.y, z_axis.y);
    let key_l = clamp(six_way(p_sample, n_sample, ls), 0.0, 1.0);
    let sky_t = clamp(six_way(p_sample, n_sample, us), 0.0, 1.0);
    let ground_t = clamp(six_way(p_sample, n_sample, -us), 0.0, 1.0);
    let avg = (p_sample.r + p_sample.g + p_sample.b + n_sample.r + n_sample.g + n_sample.b) / 6.0;

    // Forward scattering (Henyey-Greenstein).
    let cosv = dot(normalize(in.pos), sun_dir);
    let g = 0.55;
    let hg = (1.0 - g * g) / pow(max(1.0 + g * g - 2.0 * g * cosv, 1.0e-3), 1.5) * 0.0796;
    let thin = 1.0 - thick;
    let scatter = hg * 5.0 * thin * thin;

    let sun = sun_color * v_key * (key_l + scatter * 0.55);
    let sky = in.color.rgb * (0.68 * sky_t + 0.32 * avg) * vec3<f32>(0.92, 0.97, 1.07);
    let bounce = in.color.rgb * 0.2 * ground_t * vec3<f32>(0.98, 0.93, 0.86);
    let fire = block_color * v_block * (0.45 * avg + 0.55 * ground_t) * 1.3;
    let dark = vec3<f32>(0.05, 0.055, 0.075) * (0.6 * sky_t + 0.4 * avg);
    var rgb = (sun + sky + bounce + fire + dark) * v_albedo;

    if (soft_scale > 0.0 && soft > 0.0) {
        let screen_uv = in.clip.xy / target_size;
        let self_d = linear_depth(in.clip.z);
        let scene_d = linear_depth(textureSampleLevel(depth_tex, depth_smp, screen_uv, 0));
        a *= clamp((scene_d - self_d) / (soft * soft_scale), 0.0, 1.0);
    }

    // Soft shoulder over highlights.
    let over = max(rgb - vec3<f32>(0.78), vec3<f32>(0.0));
    rgb = min(rgb, vec3<f32>(0.78)) + 0.22 * (vec3<f32>(1.0) - exp(-over * 4.5));
    return vec4<f32>(rgb * a, a);
}
