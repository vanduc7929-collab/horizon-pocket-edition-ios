//! Vertex format and bit-packing builders for Grand Line's VFX and custom entity shaders.
//!
//! Matches Minecraft's `DefaultVertexFormat.NEW_ENTITY` / Grand Line's `VfxFormat.FORMAT`
//! byte-for-byte (36-byte stride, 4-byte aligned for Apple Metal & wgpu).

/// Distortion shader modes written into `VfxVertex::uv2[0]` for `vfx_distort.wgsl`.
#[repr(i16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DistortMode {
    /// Mode 0: normal map offset read from `AtlasSampler` RG * A.
    RingNormal = 0,
    /// Mode 1: scrolling heat-haze normal read from `HeatSampler` using `uv1`.
    HeatHaze = 1,
    /// Mode 2: per-shard offset encoded directly in vertex `color.rg`.
    Glass = 2,
    /// Mode 3: per-vertex offset softened across UV `v` via `sin(v * pi)`.
    Band = 3,
}

/// 36-byte vertex layout shared by all 10 geometric Grand Line VFX / entity shaders
/// (`logia_body`, `hie_ice`, `magu_magma`, `yomi_soul_body`, `vfx_dissolve`, `vfx_mesh`,
/// `vfx_particle`, `vfx_distort`, `vfx_smoke`, `vfx_smoke_tube`).
///
/// Attribute mapping:
/// - `@location(0)` `position`: `Float32x3` (offset 0)
/// - `@location(1)` `color`:    `Unorm8x4`  (offset 12, RGBA)
/// - `@location(2)` `uv0`:      `Float32x2` (offset 16)
/// - `@location(3)` `uv1`:      `Sint16x2`  (offset 24 -> WGSL `vec2<i32>`)
/// - `@location(4)` `uv2`:      `Sint16x2`  (offset 28 -> WGSL `vec2<i32>`)
/// - `@location(5)` `normal`:   `Snorm8x4`  (offset 32 -> WGSL `vec4<f32>`, `.xyz` used)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxVertex {
    pub position: [f32; 3],
    pub color: [u8; 4],
    pub uv0: [f32; 2],
    pub uv1: [i16; 2],
    pub uv2: [i16; 2],
    pub normal: [i8; 4],
}

const _: () = assert!(std::mem::size_of::<VfxVertex>() == 36);
const _: () = assert!(std::mem::align_of::<VfxVertex>() == 4);

/// Convert Minecraft's packed `0xAARRGGBB` integer to `[R, G, B, A]` vertex color bytes.
#[inline]
pub const fn argb_to_rgba(argb: u32) -> [u8; 4] {
    [
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
        ((argb >> 24) & 0xFF) as u8,
    ]
}

/// Pack normalized floating-point RGBA channels in `0.0..=1.0` to `[u8; 4]`.
#[inline]
pub fn rgba_from_f32(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// `VfxFormat.unit(v)`: quantizes a unit-range float to signed 16-bit integer `[-32767, 32767]`.
#[inline]
pub fn pack_unit(v: f32) -> i16 {
    let q = (v * 32767.0) as i32;
    q.clamp(-32767, 32767) as i16
}

/// Quantize a signed normalized normal component (`-1.0..=1.0`) to `i8` (`-127..=127`).
#[inline]
pub fn pack_snorm8(v: f32) -> i8 {
    (v.clamp(-1.0, 1.0) * 127.0) as i8
}

/// Pack a 3D normal vector into `Snorm8x4` (4th byte zero padding, matching `VfxFormat.FORMAT`).
#[inline]
pub fn pack_normal(nx: f32, ny: f32, nz: f32) -> [i8; 4] {
    [pack_snorm8(nx), pack_snorm8(ny), pack_snorm8(nz), 0]
}

/// `VfxFormat.heat(heat)` for `vfx_mesh`: negative means colour sprite (`-4096`),
/// non-negative encodes `heat * 4096` clamped to `32767`.
#[inline]
pub fn pack_mesh_heat(heat: f32) -> i16 {
    if heat < 0.0 {
        -4096
    } else {
        ((heat * 4096.0) as i32).min(32767) as i16
    }
}

/// `VfxFormat.heat(heat, atlas)` for `vfx_particle`:
/// - `atlas == 2`: sets bit 14 (`16384`) for heat >= 0, or `-20480` for heat < 0.
/// - `atlas != 2`: `heat * 4096` clamped to `16383`, or `-4096` for heat < 0.
#[inline]
pub fn pack_particle_heat(heat: f32, atlas: u8) -> i16 {
    if atlas == 2 {
        if heat < 0.0 {
            -20480
        } else {
            ((((heat * 4096.0) as i32).min(16383)) | 16384) as i16
        }
    } else if heat < 0.0 {
        -4096
    } else {
        ((heat * 4096.0) as i32).min(16383) as i16
    }
}

/// `VfxFormat.extra(palette, emissive)` for `vfx_mesh`:
/// bits `0..12` = `emissive * 256` (0..4095), bits `12..15` = `palette & 7`.
#[inline]
pub fn pack_extra(palette: u8, emissive: f32) -> i16 {
    let e = ((emissive * 256.0) as i32).clamp(0, 4095) as u16;
    let p = (u16::from(palette & 7)) << 12;
    (p | e) as i16
}

/// `VfxFormat.extra(palette, emissive, atlas)` for `vfx_particle`:
/// sets bit 15 (`0x8000`, sign-extended when read as `Sint16x2`) when `atlas == 1`.
#[inline]
pub fn pack_particle_extra(palette: u8, emissive: f32, atlas: u8) -> i16 {
    let base = pack_extra(palette, emissive) as u16;
    let pack_bit = if atlas == 1 { 0x8000u16 } else { 0u16 };
    (base | pack_bit) as i16
}

/// `SmokeParticles` `UV2.x` packing:
/// `blend (6 bits) | erosion (5 bits) << 6 | soft * 4 (4 bits) << 11`.
#[inline]
pub fn pack_smoke_uv2_x(blend: f32, erosion: f32, soft: f32) -> i16 {
    let b = ((blend * 63.0 + 0.5) as i32).clamp(0, 63);
    let e = ((erosion * 31.0 + 0.5) as i32).clamp(0, 31);
    let s = ((soft * 4.0 + 0.5) as i32).clamp(0, 15);
    (b | (e << 6) | (s << 11)) as i16
}

/// `SmokeParticles` / `SmokeTubes` `UV2.y` packing:
/// `key (8 bits) | block (7 bits) << 8`.
#[inline]
pub fn pack_smoke_uv2_y(key: f32, block: f32) -> i16 {
    let k = ((key * 255.0) as i32).clamp(0, 255);
    let blk = ((block * 127.0) as i32).clamp(0, 127);
    (k | (blk << 8)) as i16
}

/// Pack Minecraft block/sky light levels (`0..=15`) into `UV2` (`level * 16`).
#[inline]
pub const fn pack_light_uv2(block_light: u8, sky_light: u8) -> [i16; 2] {
    [
        ((block_light & 0x0F) as i16) * 16,
        ((sky_light & 0x0F) as i16) * 16,
    ]
}

/// Pack Minecraft overlay coordinates `(u, v)` (`0..=15`) into `UV1`.
#[inline]
pub const fn pack_overlay_uv1(u: u8, v: u8) -> [i16; 2] {
    [(u & 0x0F) as i16, (v & 0x0F) as i16]
}

impl VfxVertex {
    /// Vertex attribute descriptors matching `@location(0)..@location(5)` in all 10 3D VFX shaders.
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 6] = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Unorm8x4,
            offset: 12,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 16,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Sint16x2,
            offset: 24,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Sint16x2,
            offset: 28,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Snorm8x4,
            offset: 32,
            shader_location: 5,
        },
    ];

    /// `wgpu::VertexBufferLayout` for `VfxVertex` (`array_stride = 36`).
    pub const fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }

    /// Build a vertex for `vfx_mesh.wgsl` (1:1 with `MeshWriter.vertex`).
    #[allow(clippy::too_many_arguments)]
    pub fn for_mesh(
        position: [f32; 3],
        color_rgba: [u8; 4],
        uv: [f32; 2],
        heat: f32,
        palette: u8,
        emissive: f32,
        dissolve: f32,
        fresnel: f32,
        normal: [f32; 3],
    ) -> Self {
        Self {
            position,
            color: color_rgba,
            uv0: uv,
            uv1: [pack_unit(dissolve), pack_unit(fresnel)],
            uv2: [pack_mesh_heat(heat), pack_extra(palette, emissive)],
            normal: pack_normal(normal[0], normal[1], normal[2]),
        }
    }

    /// Build a vertex for `vfx_particle.wgsl` (1:1 with `VfxParticles.v`).
    ///
    /// - `atlas`: `0` = main atlas, `1` = explosion pack, `2` = pack2.
    /// - `soft`: soft depth fade distance in blocks (clamped to `0.0..=4.0`).
    /// - `additive`: `0.0` = alpha-occluding, `1.0` = purely additive.
    #[allow(clippy::too_many_arguments)]
    pub fn for_particle(
        position: [f32; 3],
        color_rgba: [u8; 4],
        uv_a: [f32; 2],
        uv_b: [f32; 2],
        blend: f32,
        soft: f32,
        additive: f32,
        heat: f32,
        palette: u8,
        emissive: f32,
        atlas: u8,
    ) -> Self {
        let soft_norm = (soft / 4.0).clamp(0.0, 1.0);
        Self {
            position,
            color: color_rgba,
            uv0: uv_a,
            uv1: [pack_unit(uv_b[0]), pack_unit(uv_b[1])],
            uv2: [
                pack_particle_heat(heat, atlas),
                pack_particle_extra(palette, emissive, atlas),
            ],
            normal: pack_normal(blend.clamp(0.0, 1.0), soft_norm, additive.clamp(0.0, 1.0)),
        }
    }

    /// Build a distortion vertex for `vfx_distort.wgsl` modes `RingNormal` (0) or `HeatHaze` (1)
    /// (1:1 with `MeshWriter.dv`).
    pub fn for_distort(
        position: [f32; 3],
        strength: f32,
        uv: [f32; 2],
        noise_uv: [f32; 2],
        mode: DistortMode,
    ) -> Self {
        let a = (strength.clamp(0.0, 1.0) * 255.0) as u8;
        Self {
            position,
            color: [255, 255, 255, a],
            uv0: uv,
            uv1: [pack_unit(noise_uv[0]), pack_unit(noise_uv[1])],
            uv2: [mode as i16, 0],
            normal: pack_normal(0.0, 1.0, 0.0),
        }
    }

    /// Build a distortion vertex for `vfx_distort.wgsl` modes `Glass` (2) or `Band` (3)
    /// where the screen-space bend direction `(ox, oy)` in `-1..=1` rides in `color.rg`
    /// (1:1 with `MeshWriter.distortVertex`).
    pub fn for_distort_offset(
        position: [f32; 3],
        ox: f32,
        oy: f32,
        strength: f32,
        v: f32,
        mode: DistortMode,
    ) -> Self {
        let r = ((ox * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
        let g = ((oy * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
        let a = (strength.clamp(0.0, 1.0) * 255.0) as u8;
        Self {
            position,
            color: [r, g, 0, a],
            uv0: [0.5, v],
            uv1: [0, 0],
            uv2: [mode as i16, 0],
            normal: pack_normal(0.0, 1.0, 0.0),
        }
    }

    /// Build a six-way relit smoke puff vertex for `vfx_smoke.wgsl`
    /// (1:1 with `SmokeParticles.v`).
    ///
    /// - `sky_ambient_rgba`: sky ambient RGB (pre-scaled by sky access & albedo) + puff opacity in A.
    /// - `roll_cos`, `roll_sin`: cos/sin of sprite roll in the view plane.
    #[allow(clippy::too_many_arguments)]
    pub fn for_smoke(
        position: [f32; 3],
        sky_ambient_rgba: [u8; 4],
        uv_a: [f32; 2],
        uv_b: [f32; 2],
        blend: f32,
        erosion: f32,
        soft: f32,
        key_light: f32,
        block_light: f32,
        roll_cos: f32,
        roll_sin: f32,
    ) -> Self {
        Self {
            position,
            color: sky_ambient_rgba,
            uv0: uv_a,
            uv1: [pack_unit(uv_b[0]), pack_unit(uv_b[1])],
            uv2: [
                pack_smoke_uv2_x(blend, erosion, soft),
                pack_smoke_uv2_y(key_light, block_light),
            ],
            normal: pack_normal(roll_cos, roll_sin, 0.0),
        }
    }

    /// Build a cylindrical smoke tube vertex for `vfx_smoke_tube.wgsl`
    /// (1:1 with `SmokeTubes.v`).
    #[allow(clippy::too_many_arguments)]
    pub fn for_smoke_tube(
        position: [f32; 3],
        sky_ambient_rgba: [u8; 4],
        uv: [f32; 2],
        erosion: f32,
        density: f32,
        albedo: f32,
        key_light: f32,
        block_light: f32,
        side_axis: [f32; 3],
    ) -> Self {
        let alb = ((albedo * 255.0) as i32).clamp(0, 255) as i16;
        Self {
            position,
            color: sky_ambient_rgba,
            uv0: uv,
            uv1: [pack_unit(erosion), pack_unit(density)],
            uv2: [alb, pack_smoke_uv2_y(key_light, block_light)],
            normal: pack_normal(side_axis[0], side_axis[1], side_axis[2]),
        }
    }

    /// Build a standard entity vertex (`logia_body`, `yomi_soul_body`, `vfx_dissolve`).
    pub fn for_entity(
        position: [f32; 3],
        color_rgba: [u8; 4],
        uv: [f32; 2],
        overlay_uv: [u8; 2],
        block_light: u8,
        sky_light: u8,
        normal: [f32; 3],
    ) -> Self {
        Self {
            position,
            color: color_rgba,
            uv0: uv,
            uv1: pack_overlay_uv1(overlay_uv[0], overlay_uv[1]),
            uv2: pack_light_uv2(block_light, sky_light),
            normal: pack_normal(normal[0], normal[1], normal[2]),
        }
    }

    /// Build a Hie Hie glacier ice vertex (`hie_ice.wgsl`, 1:1 with `IceShader`).
    ///
    /// - `grow`: `0.0..=1.0` crystallisation progress packed in vertex alpha.
    /// - `melt`: `0.0..=1.0` melt sheen packed in `uv1.x` (`0..=15`).
    /// - `inner_glow`: `0.0..=1.0` subsurface core glow packed in `uv1.y` (`0..=15`).
    #[allow(clippy::too_many_arguments)]
    pub fn for_ice(
        position: [f32; 3],
        rgb: [u8; 3],
        grow: f32,
        uv: [f32; 2],
        melt: f32,
        inner_glow: f32,
        block_light: u8,
        sky_light: u8,
        normal: [f32; 3],
    ) -> Self {
        let a = (grow.clamp(0.0, 1.0) * 255.0).round() as u8;
        let u_melt = (melt.clamp(0.0, 1.0) * 15.0).round() as u8;
        let v_glow = (inner_glow.clamp(0.0, 1.0) * 15.0).round() as u8;
        Self::for_entity(
            position,
            [rgb[0], rgb[1], rgb[2], a],
            uv,
            [u_melt, v_glow],
            block_light,
            sky_light,
            normal,
        )
    }

    /// Build a Magu Magu magma vertex (`magu_magma.wgsl`, 1:1 with `MagmaShader`).
    ///
    /// - `grow`: `0.0..=1.0` magma swell progress packed in vertex alpha.
    /// - `cool`: `0.0..=1.0` cooling factor packed in `uv1.x` (`0..=15`).
    /// - `glow`: `0.0..=1.0` heat/glow intensity packed in `uv1.y` (`0..=15`).
    #[allow(clippy::too_many_arguments)]
    pub fn for_magma(
        position: [f32; 3],
        rgb: [u8; 3],
        grow: f32,
        uv: [f32; 2],
        cool: f32,
        glow: f32,
        block_light: u8,
        sky_light: u8,
        normal: [f32; 3],
    ) -> Self {
        let a = (grow.clamp(0.0, 1.0) * 255.0).round() as u8;
        let u_cool = (cool.clamp(0.0, 1.0) * 15.0).round() as u8;
        let v_glow = (glow.clamp(0.0, 1.0) * 15.0).round() as u8;
        Self::for_entity(
            position,
            [rgb[0], rgb[1], rgb[2], a],
            uv,
            [u_cool, v_glow],
            block_light,
            sky_light,
            normal,
        )
    }
}
