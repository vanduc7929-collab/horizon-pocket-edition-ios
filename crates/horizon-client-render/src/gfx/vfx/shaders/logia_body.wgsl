// Grand Line 0.4.0 — Logia body shader (ported 1:1 from assets/devilfruits/shaders/core/logia_body.{vsh,fsh}).
// Opens holes along wound capsules and converts limbs into elemental grains, crystal facets, or molten basalt crust.

struct LogiaBodyUniform {
    model_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    color_modulator: vec4<f32>,
    // xyz = Light0_Direction
    light0_dir: vec4<f32>,
    // xyz = Light1_Direction
    light1_dir: vec4<f32>,
    fog_color: vec4<f32>,
    // x = FogStart, y = FogEnd, z = FogShape (0 = sphere, 1 = cylinder), w = GameTime
    fog_params: vec4<f32>,
    // Capsule entry points + radii: xyz = point A, w = radius (H0A..H7A)
    hole_a: array<vec4<f32>, 8>,
    // Capsule exit points: xyz = point B (H0B..H7B)
    hole_b: array<vec4<f32>, 8>,
    // Limb root positions + conversion level: xyz = root, w = level 0..1 (L0R..L5R)
    limb_r: array<vec4<f32>, 6>,
    // Limb tip positions: xyz = tip (L0T..L5T)
    limb_t: array<vec4<f32>, 6>,
    // xyz = EdgeColor, w = EdgeGlow
    edge_color_glow: vec4<f32>,
    // x = HoleCount, y = Facet, z = Crystal, w = Molten
    flags: vec4<f32>,
    // xyz = CracksA (limbs 0..2)
    cracks_a: vec4<f32>,
    // xyz = CracksB (limbs 3..5)
    cracks_b: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: LogiaBodyUniform;
@group(0) @binding(1) var sampler0_tex: texture_2d<f32>;
@group(0) @binding(2) var sampler0_smp: sampler;
@group(0) @binding(3) var sampler1_tex: texture_2d<f32>;
@group(0) @binding(4) var sampler2_tex: texture_2d<f32>;
@group(0) @binding(5) var limb_tex: texture_2d<f32>;
@group(0) @binding(6) var limb_smp: sampler;

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
    @location(0) vertex_distance: f32,
    @location(1) vertex_color: vec4<f32>,
    @location(2) light_map_color: vec4<f32>,
    @location(3) overlay_color: vec4<f32>,
    @location(4) tex_coord0: vec2<f32>,
    @location(5) body_pos: vec3<f32>,
};

const MINECRAFT_LIGHT_POWER: f32 = 0.6;
const MINECRAFT_AMBIENT_LIGHT: f32 = 0.4;

fn minecraft_mix_light(light_dir0: vec3<f32>, light_dir1: vec3<f32>, normal: vec3<f32>, color: vec4<f32>) -> vec4<f32> {
    let l0 = max(0.0, dot(light_dir0, normal));
    let l1 = max(0.0, dot(light_dir1, normal));
    let light_accum = min(1.0, (l0 + l1) * MINECRAFT_LIGHT_POWER + MINECRAFT_AMBIENT_LIGHT);
    return vec4<f32>(color.rgb * light_accum, color.a);
}

fn fog_distance(pos: vec3<f32>, shape: f32) -> f32 {
    if (shape < 0.5) {
        return length(pos);
    }
    return max(length(pos.xz), abs(pos.y));
}

fn linear_fog(in_color: vec4<f32>, dist: f32, fog_start: f32, fog_end: f32, fog_color: vec4<f32>) -> vec4<f32> {
    if (dist <= fog_start) {
        return in_color;
    }
    let f = select(
        (dist - fog_start) / max(fog_end - fog_start, 1.0e-5),
        1.0,
        dist >= fog_end,
    );
    return vec4<f32>(mix(in_color.rgb, fog_color.rgb, f * fog_color.a), in_color.a);
}

fn load_clamped(tex: texture_2d<f32>, coords: vec2<i32>) -> vec4<f32> {
    let dims = vec2<i32>(textureDimensions(tex, 0));
    let max_coord = max(dims - vec2<i32>(1, 1), vec2<i32>(0, 0));
    return textureLoad(tex, clamp(coords, vec2<i32>(0, 0), max_coord), 0);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.clip = u.proj * u.model_view * vec4<f32>(in.position, 1.0);
    out.vertex_distance = fog_distance(in.position, u.fog_params.z);
    out.vertex_color = minecraft_mix_light(u.light0_dir.xyz, u.light1_dir.xyz, in.normal.xyz, in.color);
    out.light_map_color = load_clamped(sampler2_tex, in.uv2 / vec2<i32>(16, 16));
    out.overlay_color = load_clamped(sampler1_tex, in.uv1);
    out.tex_coord0 = in.uv0;
    out.body_pos = in.position;
    return out;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}

// Fine grain, about three cells to a skin texel, on the skin's own coordinates.
fn grain(uv: vec2<f32>) -> f32 {
    let p = uv * 192.0;
    return vnoise(p) * 0.6 + vnoise(p * 2.7 + 17.0) * 0.4;
}

// Voronoi on the skin's coordinates: x = distance to nearest cell centre, y = to edge, z = cell hash.
fn voronoi(p: vec2<f32>) -> vec3<f32> {
    let i = floor(p);
    let f = fract(p);
    var d1 = 8.0;
    var d2 = 8.0;
    var id = 0.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let g = vec2<f32>(f32(x), f32(y));
            let o = vec2<f32>(hash2(i + g), hash2(i + g + vec2<f32>(31.7, 31.7)));
            let d = length(g + o - f);
            if (d < d1) {
                d2 = d1;
                d1 = d;
                id = hash2(i + g + vec2<f32>(7.3, 7.3));
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return vec3<f32>(d1, d2 - d1, id);
}

fn capsule(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1.0e-6), 0.0, 1.0);
    return length(p - (a + ab * t));
}

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let tex = textureSample(sampler0_tex, sampler0_smp, in.tex_coord0);
    let limb_sample = textureSampleLevel(limb_tex, limb_smp, in.tex_coord0, 0.0);

    var color = tex * in.vertex_color * u.color_modulator;
    if (color.a < 0.1) {
        discard;
    }

    let edge_color = u.edge_color_glow.xyz;
    let edge_glow = u.edge_color_glow.w;
    let hole_count = u.flags.x;
    let facet = u.flags.y;
    let crystal_flag = u.flags.z;
    let molten_flag = u.flags.w;

    var n = grain(in.tex_coord0);
    let cell_a = voronoi(in.tex_coord0 * 26.0);
    if (facet > 0.5) {
        // Faceted ice: every facet one value, so a cut edge breaks along straight planes, not grains.
        n = cell_a.z;
    }

    // > 0: this point of the body is element (gone); the frontier just below 0.
    var cut = -1.0;

    let count = i32(hole_count + 0.5);
    for (var i = 0; i < 8; i = i + 1) {
        if (i >= count) {
            break;
        }
        let ha = u.hole_a[i];
        let hb = u.hole_b[i];
        let r = ha.w;
        if (r <= 0.0) {
            continue;
        }
        let d = capsule(in.body_pos, ha.xyz, hb.xyz);
        cut = max(cut, (r - d) / max(r, 0.06) + (n - 0.5) * 0.5);
    }

    let limb = i32(limb_sample.r * 255.0 + 0.5) - 1;
    if (limb >= 0 && limb < 6) {
        let lr = u.limb_r[limb];
        let lt = u.limb_t[limb];
        let level = lr.w;
        if (level >= 0.999 && crystal_flag < 0.5 && molten_flag < 0.5) {
            discard;
        }
        if (level > 0.0) {
            let axis = lt.xyz - lr.xyz;
            let along = clamp(dot(in.body_pos - lr.xyz, axis) / max(dot(axis, axis), 1.0e-6), 0.0, 1.0);
            // Turns into the element from its end back towards where it joins the body.
            let front = 1.0 - level * 1.15;
            cut = max(cut, (along - front) * 3.5 + (n - 0.5) * 0.9);
        }
    }

    // Ice Form: what is element turns to crystal; molten: dark crust plates & glowing seams.
    let crystal = crystal_flag > 0.5 && cut > 0.0;
    let molten = molten_flag > 0.5 && cut > 0.0;
    if (cut > 0.0 && !crystal && !molten) {
        discard;
    }

    // Hurt flash is the flesh's: a solid element (ice) only cracks.
    if (facet < 0.5) {
        color = vec4<f32>(mix(in.overlay_color.rgb, color.rgb, in.overlay_color.a), color.a);
    }

    // Skin is lit by the world; the element glows on its own if the theme says so.
    let element_light = mix(in.light_map_color.rgb, vec3<f32>(1.0), edge_glow);
    color = vec4<f32>(color.rgb * in.light_map_color.rgb, color.a * in.light_map_color.a);

    // Seen through an opening, the inside of the body is the element itself.
    if (!front_facing && cut > -0.85) {
        color = vec4<f32>(clamp(edge_color * (0.75 + 0.7 * n), vec3<f32>(0.0), vec3<f32>(1.0)) * element_light, color.a);
    }

    // The frontier: grains of the element where the body opens, light and dark.
    let rim = smoothstep(-0.3, 0.0, cut);
    if (rim > 0.0) {
        let grains = clamp(edge_color * (0.85 + 0.95 * n), vec3<f32>(0.0), vec3<f32>(1.0)) * element_light;
        color = vec4<f32>(mix(color.rgb, grains, rim * 0.92), color.a);
    }

    // Cracks in a solid body: fracture planes between facets.
    if (facet > 0.5 && limb >= 0 && limb < 6) {
        var stages = array<f32, 6>(
            u.cracks_a.x, u.cracks_a.y, u.cracks_a.z,
            u.cracks_b.x, u.cracks_b.y, u.cracks_b.z,
        );
        let stage = stages[limb];
        if (stage > 0.0) {
            let fine = voronoi(in.tex_coord0 * 58.0 + vec2<f32>(3.1, 3.1));
            let open = step(cell_a.z, stage * 0.9 + 0.1);
            let seam = (1.0 - smoothstep(0.0, 0.12, cell_a.y)) * open;
            let halo = (1.0 - smoothstep(0.08, 0.32, cell_a.y)) * open;
            let hair = (1.0 - smoothstep(0.0, 0.06, fine.y)) * step(fine.z, stage * 0.7) * 0.7;
            let crack = clamp(seam + hair, 0.0, 1.0);
            let core = (1.0 - smoothstep(0.0, 0.035, cell_a.y)) * open;
            let frost = vec3<f32>(0.78, 0.9, 1.0) * in.light_map_color.rgb;
            let deep = mix(edge_color, vec3<f32>(0.08, 0.16, 0.42), 0.55) * mix(in.light_map_color.rgb, vec3<f32>(1.0), 0.3);
            let glass = mix(edge_color, vec3<f32>(0.92, 0.98, 1.0), 0.75) * mix(in.light_map_color.rgb, vec3<f32>(1.0), 0.7);
            var rgb = color.rgb;
            rgb = mix(rgb, frost, halo * 0.35 * stage);
            rgb = mix(rgb, deep, crack * 0.85);
            rgb = mix(rgb, glass, core);
            color = vec4<f32>(rgb, color.a);
        }
    }

    if (crystal) {
        let facet_shade = 0.55 + 0.45 * cell_a.z;
        var ice = mix(edge_color, vec3<f32>(0.86, 0.96, 1.0), facet_shade);
        let seam = 1.0 - smoothstep(0.0, 0.06, cell_a.y);
        ice = mix(ice, edge_color * 0.5, seam * 0.7);
        let glint = step(0.985, hash2(floor(in.tex_coord0 * 96.0) + vec2<f32>(floor(cell_a.z * 7.0))));
        color = vec4<f32>(
            clamp(ice + vec3<f32>(glint * 0.8), vec3<f32>(0.0), vec3<f32>(1.0)) * mix(in.light_map_color.rgb, vec3<f32>(1.0), 0.45),
            0.86,
        );
    }

    if (molten) {
        let t = u.fog_params.w * 24000.0;
        let flow_uv = in.tex_coord0 * 30.0 + vec2<f32>(sin(t * 0.021 + in.tex_coord0.y * 9.0) * 0.35, t * 0.012);
        let plates = voronoi(flow_uv);
        let fine = voronoi(flow_uv * 2.3 + vec2<f32>(11.0, 11.0));
        let pulse = 0.78 + 0.22 * sin(t * 0.13 + plates.z * 6.2832);
        let seam = 1.0 - smoothstep(0.0, 0.16, plates.y);
        let hair = (1.0 - smoothstep(0.0, 0.07, fine.y)) * step(0.45, fine.z) * 0.6;
        let raw = 1.0 - smoothstep(0.0, 0.6, cut);
        let heat = clamp(max(seam * pulse, hair * pulse) + raw * 0.85 + (1.0 - plates.x) * 0.08, 0.0, 1.0);
        var basalt = mix(vec3<f32>(0.055, 0.04, 0.045), vec3<f32>(0.17, 0.09, 0.07), plates.z * 0.55 + n * 0.45);
        basalt *= in.light_map_color.rgb * 0.75 + vec3<f32>(0.25);
        let glow = select(
            mix(vec3<f32>(1.0, 0.32, 0.03), vec3<f32>(1.0, 0.88, 0.55), (heat - 0.5) * 2.0),
            mix(vec3<f32>(0.45, 0.03, 0.01), vec3<f32>(1.0, 0.32, 0.03), heat * 2.0),
            heat < 0.5,
        );
        let bleed = (1.0 - smoothstep(0.0, 0.32, plates.y)) * 0.35;
        let rock = basalt + glow * bleed;
        color = vec4<f32>(mix(rock, glow, smoothstep(0.22, 0.8, heat)), 1.0);
    }

    return linear_fog(color, in.vertex_distance, u.fog_params.x, u.fog_params.y, u.fog_color);
}
