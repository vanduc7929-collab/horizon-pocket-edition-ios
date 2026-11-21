// Grand Line 0.4.0 — Half-resolution ray-marched volumetric smoke clouds
// (ported 1:1 from assets/devilfruits/shaders/core/vfx_screen.vsh + vfx_smoke_volume.fsh).
// Up to 4 squashed ellipsoids ray-marched front-to-back against scene depth using an 8x8 atlas of 64^3 noise,
// single shadow march + dual-octave multiple scattering, Henyey-Greenstein phase, powder effect, premultiplied output.

struct VfxSmokeVolumeUniform {
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // Each vec4: xyz = camera-relative centre, w = radius
    centers: array<vec4<f32>, 4>,
    // Each vec4: x = sigma (extinction per block), y = erosion, z = churn, w = seed
    params: array<vec4<f32>, 4>,
    // Each vec4: x = flatten (vertical squash), y = albedo, z = keyShare, w = block light
    shapes: array<vec4<f32>, 4>,
    // Each vec4: x = skyShare, yzw = reserved
    lights: array<vec4<f32>, 4>,
    // xy = FullSize, z = Count (0..4), w = Steps (14 under load, 26 full)
    full_size_count_steps: vec4<f32>,
    // xyz = SunDir, w = Time
    sun_dir_time: vec4<f32>,
    // xyz = SunColor
    sun_color: vec4<f32>,
    // xyz = SkyColor
    sky_color: vec4<f32>,
    // xyz = BlockColor
    block_color: vec4<f32>,
    // x = ndc_z_scale (1.0 wgpu [0,1], 2.0 GL [-1,1]),
    // y = ndc_z_bias  (0.0 wgpu [0,1], -1.0 GL [-1,1]),
    // z = reverse_z   (1.0 Horizon reversed-Z, 0.0 standard-Z),
    // w = far_plane
    depth_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: VfxSmokeVolumeUniform;
@group(0) @binding(1) var depth_tex: texture_depth_2d;
@group(0) @binding(2) var depth_smp: sampler;
@group(0) @binding(3) var volume_tex: texture_2d<f32>;
@group(0) @binding(4) var volume_smp: sampler;

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

// GLSL floored modulo: always non-negative for positive y (unlike WGSL truncated `%`).
fn glsl_mod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

// 64^3 noise packed into an 8x8 tile atlas: manual trilinear interpolation across slices.
fn vol_noise(p_in: vec3<f32>) -> vec4<f32> {
    let p = fract(p_in);
    let z = p.z * 64.0 - 0.5;
    let zi = floor(z);
    let zf = z - zi;
    let s0 = glsl_mod(zi, 64.0);
    let s1 = glsl_mod(zi + 1.0, 64.0);
    let uv = clamp(p.xy, vec2<f32>(0.5 / 64.0), vec2<f32>(1.0 - 0.5 / 64.0));
    let t0 = (vec2<f32>(glsl_mod(s0, 8.0), floor(s0 / 8.0)) + uv) / 8.0;
    let t1 = (vec2<f32>(glsl_mod(s1, 8.0), floor(s1 / 8.0)) + uv) / 8.0;
    return mix(
        textureSampleLevel(volume_tex, volume_smp, t0, 0.0),
        textureSampleLevel(volume_tex, volume_smp, t1, 0.0),
        zf,
    );
}

fn remap(v: f32, lo: f32, hi: f32, a: f32, b: f32) -> f32 {
    return a + (clamp(v, lo, hi) - lo) / max(hi - lo, 1.0e-4) * (b - a);
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs_main(in: ScreenOut) -> @location(0) vec4<f32> {
    let full_size = u.full_size_count_steps.xy;
    let count_f = u.full_size_count_steps.z;
    let steps_in = u.full_size_count_steps.w;
    let sun_dir = u.sun_dir_time.xyz;
    let time = u.sun_dir_time.w;
    let sun_color = u.sun_color.xyz;
    let sky_color = u.sky_color.xyz;
    let block_color = u.block_color.xyz;
    let reverse_z = u.depth_params.z > 0.5;

    // Nearest scene surface under this half-resolution pixel (closest of four taps).
    let texel = vec2<f32>(1.0) / full_size;
    let d0 = textureSampleLevel(depth_tex, depth_smp, in.uv + vec2<f32>(-0.5, -0.5) * texel, 0);
    let d1 = textureSampleLevel(depth_tex, depth_smp, in.uv + vec2<f32>( 0.5, -0.5) * texel, 0);
    let d2 = textureSampleLevel(depth_tex, depth_smp, in.uv + vec2<f32>(-0.5,  0.5) * texel, 0);
    let d3 = textureSampleLevel(depth_tex, depth_smp, in.uv + vec2<f32>( 0.5,  0.5) * texel, 0);

    let depth = select(
        min(min(d0, d1), min(d2, d3)),
        max(max(d0, d1), max(d2, d3)),
        reverse_z,
    );
    let is_sky = select(depth >= 0.9999, depth <= 0.0001, reverse_z);

    // Reconstruct world-space view ray from Y-down screen UV (top-left = (0,0), NDC top-left = (-1,+1)).
    let ndc_xy = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let view_ray = u.inv_proj * vec4<f32>(ndc_xy, 0.5, 1.0);
    let dir_view = normalize(view_ray.xyz / view_ray.w);
    let dir = normalize((u.inv_view * vec4<f32>(dir_view, 0.0)).xyz);

    var t_end = 1.0e4;
    if (!is_sky) {
        let ndc_z = depth * u.depth_params.x + u.depth_params.y;
        let view_hit = u.inv_proj * vec4<f32>(ndc_xy, ndc_z, 1.0);
        t_end = length(view_hit.xyz / view_hit.w);
    }

    let cosv = dot(dir, sun_dir);
    let g = 0.45;
    let phase = (1.0 - g * g) / pow(max(1.0 + g * g - 2.0 * g * cosv, 1.0e-3), 1.5) * 0.0796;
    let jitter = hash2(in.clip.xy + vec2<f32>(fract(time * 0.37) * 61.0));

    var col = vec3<f32>(0.0);
    var transmittance = 1.0;
    let count = i32(count_f + 0.5);

    for (var i = 0; i < 4; i = i + 1) {
        if (i >= count || transmittance < 0.02) {
            break;
        }
        let center_r = u.centers[i];
        let param = u.params[i];
        let shape = u.shapes[i];
        let light = u.lights[i];

        let c = center_r.xyz;
        let radius = center_r.w;
        let sigma = param.x;
        let erosion = param.y;
        let churn = param.z;
        let seed = param.w;
        let squash = max(0.2, shape.x);
        let albedo = shape.y;
        let key_share = shape.z;
        let block = shape.w;
        let sky_share = light.x;

        if (sigma <= 0.0) {
            continue;
        }

        // Ray intersection against vertically squashed ellipsoid.
        let sc = vec3<f32>(1.0, 1.0 / squash, 1.0);
        let o = -c * sc;
        let dd = dir * sc;
        let qa = dot(dd, dd);
        let qb = 2.0 * dot(o, dd);
        let qc = dot(o, o) - radius * radius;
        let disc = qb * qb - 4.0 * qa * qc;
        if (disc <= 0.0) {
            continue;
        }
        let sq = sqrt(disc);
        let t0 = max((-qb - sq) / (2.0 * qa), 0.0);
        let t1 = min((-qb + sq) / (2.0 * qa), t_end);
        if (t1 <= t0) {
            continue;
        }

        let steps = max(8.0, steps_in);
        let dt = (t1 - t0) / steps;
        var t = t0 + dt * jitter;
        let drift = vec3<f32>(time * churn * 0.6, -time * churn, time * churn * 0.3) / 18.0 + vec3<f32>(seed);
        let ambient = sky_color * sky_share + vec3<f32>(0.05, 0.055, 0.075);
        let base_scale = 1.0 / (radius * 0.9 + 4.0);

        for (var k = 0; k < 40; k = k + 1) {
            if (f32(k) >= steps) {
                break;
            }
            let p = dir * t - c;
            let r = length(p * sc) / radius;
            let n0 = vol_noise(p * base_scale + drift);
            let base = n0.r * 0.5 + n0.g * 0.5;
            let coverage = 1.0 - smoothstep(0.35, 1.0, r);
            var d = remap(base, 1.0 - coverage * 0.92, 1.0, 0.0, 1.0);
            if (d > 0.001) {
                let n1 = vol_noise(p / 3.3 + drift * 2.3);
                let det = n1.b * 0.6 + n1.a * 0.4;
                d = remap(d, det * (0.25 + 0.5 * erosion) * (1.0 - d * 0.5), 1.0, 0.0, 1.0);
            }
            let den = d * sigma * 2.2;
            if (den > 0.002) {
                // Single shadow sample toward the sun (coarse field) with two-octave multiple scattering.
                let q = p + sun_dir * radius * 0.18;
                let rq = length(q * sc) / radius;
                let nq = vol_noise(q * base_scale + drift);
                let dq = remap(
                    nq.r * 0.5 + nq.g * 0.5,
                    1.0 - (1.0 - smoothstep(0.35, 1.0, rq)) * 0.92,
                    1.0,
                    0.0,
                    1.0,
                );
                let tau = dq * sigma * 2.2 * radius * 0.36;
                let light_t = 0.62 * exp(-tau) + 0.38 * exp(-tau * 0.25);
                let height = clamp(p.y / (radius * squash) * 0.5 + 0.5, 0.0, 1.0);
                let powder = 1.0 - exp(-den * 4.0);
                let lit = sun_color * key_share * light_t * (phase * 3.0 + 0.32) * (0.55 + 0.45 * powder)
                        + ambient * (0.45 + 0.55 * height) * vec3<f32>(0.92, 0.97, 1.07)
                        + block_color * block * (1.0 - 0.45 * height) * 1.2;
                let st = exp(-den * dt);
                col += transmittance * (1.0 - st) * lit * albedo;
                transmittance *= st;
                if (transmittance < 0.02) {
                    break;
                }
            }
            t += dt;
        }
    }

    return vec4<f32>(col, 1.0 - transmittance);
}
