//! `#[repr(C)]` `bytemuck::Pod` uniform buffer structs matching all 15 Grand Line WGSL shaders
//! byte-for-byte under WGSL `uniform` / std140 / Apple Metal constant-buffer layout rules.
//!
//! Every field is 16-byte aligned (`[[f32; 4]; 4]`, `[f32; 4]`, or `[[f32; 4]; N]`) so there is
//! zero implicit padding between Rust, WGSL, and Metal.

use glam::{Mat4, Vec3, Vec4};

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Minecraft fog distance metric (`FogShape` uniform).
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FogShape {
    #[default]
    Spherical = 0,
    Cylindrical = 1,
}

impl FogShape {
    #[inline]
    pub const fn as_f32(self) -> f32 {
        match self {
            Self::Spherical => 0.0,
            Self::Cylindrical => 1.0,
        }
    }
}

/// Depth linearization parameters (`depth_params: vec4<f32>`) supporting Horizon PE's
/// reversed-Z `[1, 0]` buffer, standard wgpu/Metal `[0, 1]`, and legacy OpenGL `[-1, 1]`.
///
/// Layout: `[ndc_z_scale, ndc_z_bias, reverse_z_flag, far_plane]`.
pub struct DepthParams;

impl DepthParams {
    /// Horizon PE default: reversed-Z perspective (`Mat4::perspective_rh(fov, aspect, far, near)`),
    /// where `d = 1.0` at the near plane and `d = 0.0` at the far plane.
    #[inline]
    pub const fn reversed_z(far_plane: f32) -> [f32; 4] {
        [1.0, 0.0, 1.0, far_plane]
    }

    /// Standard wgpu / Metal `[0, 1]` perspective (`Mat4::perspective_rh(fov, aspect, near, far)`),
    /// where `d = 0.0` at the near plane and `d = 1.0` at the far plane.
    #[inline]
    pub const fn standard_z(far_plane: f32) -> [f32; 4] {
        [1.0, 0.0, 0.0, far_plane]
    }

    /// Legacy OpenGL `[-1, 1]` NDC projection (`Mat4::perspective_rh_gl`), matching the original GLSL.
    #[inline]
    pub const fn opengl_ndc(far_plane: f32) -> [f32; 4] {
        [2.0, -1.0, 0.0, far_plane]
    }

    /// CPU mirror of the WGSL `linear_depth` helper for validation and unit testing.
    #[inline]
    pub fn linearize(depth: f32, proj: Mat4, params: [f32; 4]) -> f32 {
        let cols = proj.to_cols_array_2d();
        let denom = (depth * params[0] + params[1]) + cols[2][2];
        if denom.abs() < 1.0e-7 {
            params[3]
        } else {
            cols[3][2] / denom
        }
    }
}

/// 1. `logia_body.wgsl` (`LogiaBodyUniform`) — 720 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LogiaBodyUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub color_modulator: [f32; 4],
    /// `xyz = Light0_Direction`.
    pub light0_dir: [f32; 4],
    /// `xyz = Light1_Direction`.
    pub light1_dir: [f32; 4],
    pub fog_color: [f32; 4],
    /// `x = FogStart`, `y = FogEnd`, `z = FogShape`, `w = GameTime`.
    pub fog_params: [f32; 4],
    /// `H0A..H7A`: `xyz = entry point`, `w = radius`.
    pub hole_a: [[f32; 4]; 8],
    /// `H0B..H7B`: `xyz = exit point`.
    pub hole_b: [[f32; 4]; 8],
    /// `L0R..L5R`: `xyz = limb root`, `w = conversion level (0..1)`.
    pub limb_r: [[f32; 4]; 6],
    /// `L0T..L5T`: `xyz = limb tip`.
    pub limb_t: [[f32; 4]; 6],
    /// `xyz = EdgeColor`, `w = EdgeGlow`.
    pub edge_color_glow: [f32; 4],
    /// `x = HoleCount`, `y = Facet`, `z = Crystal`, `w = Molten`.
    pub flags: [f32; 4],
    /// `xyz = CracksA` (limbs 0..2).
    pub cracks_a: [f32; 4],
    /// `xyz = CracksB` (limbs 3..5).
    pub cracks_b: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<LogiaBodyUniform>() == 720);

impl Default for LogiaBodyUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            color_modulator: [1.0, 1.0, 1.0, 1.0],
            light0_dir: [0.0; 4],
            light1_dir: [0.0; 4],
            fog_color: [0.0; 4],
            fog_params: [0.0, 1.0, 0.0, 0.0],
            hole_a: [[0.0; 4]; 8],
            hole_b: [[0.0; 4]; 8],
            limb_r: [[0.0; 4]; 6],
            limb_t: [[0.0; 4]; 6],
            edge_color_glow: [0.6, 0.45, 0.25, 0.0],
            flags: [0.0; 4],
            cracks_a: [0.0; 4],
            cracks_b: [0.0; 4],
        }
    }
}

impl LogiaBodyUniform {
    /// Set wound capsule `index` (`0..8`) with entry `a`, exit `b`, and `radius`.
    pub fn set_hole(&mut self, index: usize, a: Vec3, b: Vec3, radius: f32) {
        if index < 8 {
            self.hole_a[index] = [a.x, a.y, a.z, radius];
            self.hole_b[index] = [b.x, b.y, b.z, 0.0];
        }
    }

    /// Set limb `index` (`0..6`) axis from `root` to `tip` and elemental conversion `level` (`0..1`).
    pub fn set_limb(&mut self, index: usize, root: Vec3, tip: Vec3, level: f32) {
        if index < 6 {
            self.limb_r[index] = [root.x, root.y, root.z, level];
            self.limb_t[index] = [tip.x, tip.y, tip.z, 0.0];
        }
    }

    /// Set solid-body (ice) crack stages (`0..1`) for all 6 limbs.
    pub fn set_cracks(&mut self, stages: [f32; 6]) {
        self.cracks_a = [stages[0], stages[1], stages[2], 0.0];
        self.cracks_b = [stages[3], stages[4], stages[5], 0.0];
    }
}

/// 2. `hie_ice.wgsl` (`HieIceUniform`) — 224 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HieIceUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub color_modulator: [f32; 4],
    pub light0_dir: [f32; 4],
    pub light1_dir: [f32; 4],
    pub fog_color: [f32; 4],
    /// `x = FogStart`, `y = FogEnd`, `z = FogShape`, `w = GameTime`.
    pub fog_params: [f32; 4],
    /// `xy = ScreenSize`, `z = Refract (1.0 or 0.0)`, `w = 0.0`.
    pub screen_refract: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<HieIceUniform>() == 224);

impl Default for HieIceUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            color_modulator: [1.0, 1.0, 1.0, 1.0],
            light0_dir: [0.0; 4],
            light1_dir: [0.0; 4],
            fog_color: [0.0; 4],
            fog_params: [0.0, 1.0, 0.0, 0.0],
            screen_refract: [1.0, 1.0, 1.0, 0.0],
        }
    }
}

/// 3. `magu_magma.wgsl` (`MaguMagmaUniform`) — 224 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaguMagmaUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub color_modulator: [f32; 4],
    pub light0_dir: [f32; 4],
    pub light1_dir: [f32; 4],
    pub fog_color: [f32; 4],
    /// `x = FogStart`, `y = FogEnd`, `z = FogShape`, `w = GameTime`.
    pub fog_params: [f32; 4],
    /// `x = Parallax (0.0 on Medium, 1.0 on High)`, `yzw = 0.0`.
    pub magma_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<MaguMagmaUniform>() == 224);

impl Default for MaguMagmaUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            color_modulator: [1.0, 1.0, 1.0, 1.0],
            light0_dir: [0.0; 4],
            light1_dir: [0.0; 4],
            fog_color: [0.0; 4],
            fog_params: [0.0, 1.0, 0.0, 0.0],
            magma_params: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// 4. `yomi_soul_body.wgsl` (`YomiSoulBodyUniform`) — 208 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct YomiSoulBodyUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub color_modulator: [f32; 4],
    pub fog_color: [f32; 4],
    /// `x = FogStart`, `y = FogEnd`, `z = FogShape`, `w = 0.0`.
    pub fog_params: [f32; 4],
    /// `xyz = Origin`, `w = Yaw` (radians).
    pub origin_yaw: [f32; 4],
    /// `x = Time`, `y = Burn`, `zw = 0.0`.
    pub time_burn: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<YomiSoulBodyUniform>() == 208);

impl Default for YomiSoulBodyUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            color_modulator: [1.0, 1.0, 1.0, 1.0],
            fog_color: [0.0; 4],
            fog_params: [0.0, 1.0, 0.0, 0.0],
            origin_yaw: [0.0; 4],
            time_burn: [0.0, 1.0, 0.0, 0.0],
        }
    }
}

/// 5. `vfx_mesh.wgsl` (`VfxMeshUniform`) — 176 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxMeshUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// `xy = TargetSize`, `z = Pass (0 scene / 1 emissive)`, `w = SoftScale`.
    pub target_pass_soft: [f32; 4],
    /// `x = MinAlpha`, `y = AlphaCut`, `z = EmissiveScale`, `w = 0.0`.
    pub alpha_emissive: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxMeshUniform>() == 176);

impl Default for VfxMeshUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            target_pass_soft: [1.0, 1.0, 0.0, 1.0],
            alpha_emissive: [0.0, 0.002, 1.0, 0.0],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// 6. `vfx_particle.wgsl` (`VfxParticleUniform`) — 176 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxParticleUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// `xy = TargetSize`, `z = Pass (0 scene / 1 emissive)`, `w = SoftScale`.
    pub target_pass_soft: [f32; 4],
    /// `x = MinAlpha`, `y = EmissiveScale`, `zw = 0.0`.
    pub alpha_emissive: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxParticleUniform>() == 176);

impl Default for VfxParticleUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            target_pass_soft: [1.0, 1.0, 0.0, 1.0],
            alpha_emissive: [0.0, 1.0, 0.0, 0.0],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// 7. `vfx_distort.wgsl` (`VfxDistortUniform`) — 160 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxDistortUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// `xy = TargetSize`, `z = Time`, `w = 0.0`.
    pub target_time: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxDistortUniform>() == 160);

impl Default for VfxDistortUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            target_time: [1.0, 1.0, 0.0, 0.0],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// 8. `vfx_dissolve.wgsl` (`VfxDissolveUniform`) — 208 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxDissolveUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub color_modulator: [f32; 4],
    pub light0_dir: [f32; 4],
    pub light1_dir: [f32; 4],
    pub fog_color: [f32; 4],
    /// `x = FogStart`, `y = FogEnd`, `z = FogShape`, `w = 0.0`.
    pub fog_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxDissolveUniform>() == 208);

impl Default for VfxDissolveUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            color_modulator: [1.0, 1.0, 1.0, 1.0],
            light0_dir: [0.0; 4],
            light1_dir: [0.0; 4],
            fog_color: [0.0; 4],
            fog_params: [0.0, 1.0, 0.0, 0.0],
        }
    }
}

/// 9. `vfx_smoke.wgsl` (`VfxSmokeUniform`) — 224 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxSmokeUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// `xy = TargetSize`, `z = SoftScale`, `w = Time`.
    pub target_soft_time: [f32; 4],
    /// `xyz = SunDir`.
    pub sun_dir: [f32; 4],
    /// `xyz = SunColor`.
    pub sun_color: [f32; 4],
    /// `xyz = BlockColor`.
    pub block_color: [f32; 4],
    /// `xyz = CamPos` (modulo 512 for precision).
    pub cam_pos: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxSmokeUniform>() == 224);

impl Default for VfxSmokeUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            target_soft_time: [1.0, 1.0, 1.0, 0.0],
            sun_dir: [0.0, 1.0, 0.0, 0.0],
            sun_color: [1.0, 1.0, 1.0, 0.0],
            block_color: [1.0, 0.64, 0.34, 0.0],
            cam_pos: [0.0; 4],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// 10. `vfx_smoke_tube.wgsl` (`VfxSmokeTubeUniform`) — 208 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxSmokeTubeUniform {
    pub model_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// `xy = TargetSize`, `z = SoftScale`, `w = 0.0`.
    pub target_soft: [f32; 4],
    /// `xyz = SunDir`.
    pub sun_dir: [f32; 4],
    /// `xyz = SunColor`.
    pub sun_color: [f32; 4],
    /// `xyz = BlockColor`.
    pub block_color: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxSmokeTubeUniform>() == 208);

impl Default for VfxSmokeTubeUniform {
    fn default() -> Self {
        Self {
            model_view: IDENTITY,
            proj: IDENTITY,
            target_soft: [1.0, 1.0, 1.0, 0.0],
            sun_dir: [0.0, 1.0, 0.0, 0.0],
            sun_color: [1.0, 1.0, 1.0, 0.0],
            block_color: [1.0, 0.64, 0.34, 0.0],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// 11. `vfx_bloom_down.wgsl` (`VfxBloomDownUniform`) — 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxBloomDownUniform {
    /// `xy = SrcTexel`, `z = Threshold`, `w = FirstPass`.
    pub params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxBloomDownUniform>() == 16);

impl Default for VfxBloomDownUniform {
    fn default() -> Self {
        Self {
            params: [0.0, 0.0, 0.22, 0.0],
        }
    }
}

/// 12. `vfx_bloom_up.wgsl` (`VfxBloomUpUniform`) — 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxBloomUpUniform {
    /// `xy = SrcTexel`, `z = Radius`, `w = Intensity`.
    pub params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxBloomUpUniform>() == 16);

impl Default for VfxBloomUpUniform {
    fn default() -> Self {
        Self {
            params: [0.0, 0.0, 1.0, 0.9],
        }
    }
}

/// 13. `vfx_composite.wgsl` (`VfxCompositeUniform`) — 80 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxCompositeUniform {
    /// `xy = RadialCenter` (wgpu Y-down screen UV), `z = RadialStrength`, `w = Chromatic`.
    pub radial_chroma: [f32; 4],
    /// `x = BloomIntensity`, `y = DistortOn`, `z = Vignette`, `w = Desaturate`.
    pub bloom_distort_vig_desat: [f32; 4],
    /// `rgb = Flash color`, `a = Flash amount`.
    pub flash: [f32; 4],
    /// `rgb = Grade tint`, `a = Grade amount`.
    pub grade: [f32; 4],
    /// `xy = InvSize (1/w, 1/h)`, `zw = 0.0`.
    pub inv_size: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxCompositeUniform>() == 80);

impl Default for VfxCompositeUniform {
    fn default() -> Self {
        Self {
            radial_chroma: [0.5, 0.5, 0.0, 0.0],
            bloom_distort_vig_desat: [0.0, 0.0, 0.0, 0.0],
            flash: [0.0, 0.0, 0.0, 0.0],
            grade: [1.0, 1.0, 1.0, 0.0],
            inv_size: [0.001, 0.001, 0.0, 0.0],
        }
    }
}

impl VfxCompositeUniform {
    /// Project a camera-relative 3D world point to wgpu Y-down screen UV `(0,0) = top-left`
    /// clamped to `[-0.2, 1.2]` (matches `VfxPost.apply` radial blur projection).
    pub fn project_radial_center(camera_relative: Vec3, view_proj: Mat4) -> [f32; 2] {
        let clip: Vec4 = view_proj * camera_relative.extend(1.0);
        if clip.w > 0.05 {
            let cx = (clip.x / clip.w * 0.5 + 0.5).clamp(-0.2, 1.2);
            let cy = (-clip.y / clip.w * 0.5 + 0.5).clamp(-0.2, 1.2);
            [cx, cy]
        } else {
            [0.5, 0.5]
        }
    }
}

/// 14. `vfx_smoke_volume.wgsl` (`VfxSmokeVolumeUniform`) — 544 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxSmokeVolumeUniform {
    pub inv_proj: [[f32; 4]; 4],
    pub inv_view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    /// Up to 4 volumes: `xyz = camera-relative centre`, `w = radius`.
    pub centers: [[f32; 4]; 4],
    /// Up to 4 volumes: `x = sigma`, `y = erosion`, `z = churn`, `w = seed`.
    pub params: [[f32; 4]; 4],
    /// Up to 4 volumes: `x = flatten`, `y = albedo`, `z = key_share`, `w = block_light`.
    pub shapes: [[f32; 4]; 4],
    /// Up to 4 volumes: `x = sky_share`, `yzw = 0.0`.
    pub lights: [[f32; 4]; 4],
    /// `xy = FullSize`, `z = Count (0..4)`, `w = Steps (14 under load, 26 normal)`.
    pub full_size_count_steps: [f32; 4],
    /// `xyz = SunDir`, `w = Time`.
    pub sun_dir_time: [f32; 4],
    /// `xyz = SunColor`.
    pub sun_color: [f32; 4],
    /// `xyz = SkyColor`.
    pub sky_color: [f32; 4],
    /// `xyz = BlockColor`.
    pub block_color: [f32; 4],
    /// Depth linearization parameters (`DepthParams`).
    pub depth_params: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxSmokeVolumeUniform>() == 544);

impl Default for VfxSmokeVolumeUniform {
    fn default() -> Self {
        Self {
            inv_proj: IDENTITY,
            inv_view: IDENTITY,
            proj: IDENTITY,
            centers: [[0.0; 4]; 4],
            params: [[0.0; 4]; 4],
            shapes: [[0.0; 4]; 4],
            lights: [[0.0; 4]; 4],
            full_size_count_steps: [1.0, 1.0, 0.0, 26.0],
            sun_dir_time: [0.0, 1.0, 0.0, 0.0],
            sun_color: [1.0, 1.0, 1.0, 0.0],
            sky_color: [0.5, 0.55, 0.65, 0.0],
            block_color: [1.0, 0.64, 0.34, 0.0],
            depth_params: DepthParams::reversed_z(1024.0),
        }
    }
}

/// Parameters for a single ray-marched smoke volume slot (`0..4`).
#[derive(Clone, Copy, Debug)]
pub struct SmokeVolumeSlot {
    pub center_rel: Vec3,
    pub radius: f32,
    pub sigma: f32,
    pub erosion: f32,
    pub churn: f32,
    pub seed: f32,
    pub flatten: f32,
    pub albedo: f32,
    pub key_share: f32,
    pub block_light: f32,
    pub sky_share: f32,
}

impl VfxSmokeVolumeUniform {
    /// Populate volume slot `index` (`0..4`) matching `SmokeRenderer.volumes`.
    pub fn set_volume(&mut self, index: usize, slot: &SmokeVolumeSlot) {
        if index < 4 {
            self.centers[index] = [
                slot.center_rel.x,
                slot.center_rel.y,
                slot.center_rel.z,
                slot.radius,
            ];
            self.params[index] = [slot.sigma, slot.erosion, slot.churn, slot.seed];
            self.shapes[index] = [slot.flatten, slot.albedo, slot.key_share, slot.block_light];
            self.lights[index] = [slot.sky_share, 0.0, 0.0, 0.0];
        }
    }
}

/// 15. `vfx_smoke_composite.wgsl` (`VfxSmokeCompositeUniform`) — 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VfxSmokeCompositeUniform {
    /// `xy = SrcTexel (1/half_w, 1/half_h)`, `zw = 0.0`.
    pub src_texel: [f32; 4],
}

const _: () = assert!(std::mem::size_of::<VfxSmokeCompositeUniform>() == 16);

impl Default for VfxSmokeCompositeUniform {
    fn default() -> Self {
        Self {
            src_texel: [1.0, 1.0, 0.0, 0.0],
        }
    }
}
