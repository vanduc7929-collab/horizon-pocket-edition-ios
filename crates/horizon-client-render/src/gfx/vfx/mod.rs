//! Grand Line 0.4.0 VFX shader suite (all 15 WGSL programs) and wgpu/Metal multi-pass frame graph.
//!
//! Ported 1:1 from `assets/devilfruits/shaders/core/*` and `com.devilfruits.client.vfx.*`
//! for Horizon Pocket Edition iOS (`wgpu 30.0.1`, Metal 2.1+, reversed-Z `Depth32Float`).

pub mod frame_graph;
pub mod targets;
pub mod uniforms;
pub mod vertex;

#[cfg(test)]
mod tests;

pub use frame_graph::{
    ADDITIVE_BLEND, CompositeParams, EntityDrawBatch, EntityShaderVariant, PREMULTIPLIED_BLEND,
    SmokeVolumePassParams, TextureMcmeta, TextureSamplerSpec, VfxBindGroupLayouts, VfxDrawBatch,
    VfxFrameGraph, VfxFrameInputs, VfxFrameStats, VfxPipelineSet, VfxSamplerCache,
};
pub use targets::{
    DISTORT_FORMAT, HDR_FORMAT, MAX_BLOOM_MIPS, VfxTargets, VfxTextureTarget,
};
pub use uniforms::{
    DepthParams, FogShape, HieIceUniform, LogiaBodyUniform, MaguMagmaUniform, SmokeVolumeSlot,
    VfxBloomDownUniform, VfxBloomUpUniform, VfxCompositeUniform, VfxDissolveUniform,
    VfxDistortUniform, VfxMeshUniform, VfxParticleUniform, VfxSmokeCompositeUniform,
    VfxSmokeTubeUniform, VfxSmokeUniform, VfxSmokeVolumeUniform, YomiSoulBodyUniform,
};
pub use vertex::{DistortMode, VfxVertex};

/// iOS-tuned Grand Line VFX quality tiers (maps onto Grand Line's Low / Medium / High / Ultra).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VfxQuality {
    /// All Grand Line VFX offscreen passes and custom shaders disabled (zero GPU memory).
    Off,
    /// Minimal tier: no bloom, no distortion, no ray-marched volumes, no scene refraction copy.
    Low,
    /// Balanced mobile tier: half-res emissive, 3-mip bloom, soft depth fade, lite blended ice.
    Medium,
    /// Full cinematic tier: full-res emissive (half-res under thermal load), up to 5-6 bloom mips,
    /// half-res Rg16Float distortion, half-res ray-marched smoke volumes, refracting glacier ice.
    #[default]
    High,
}

impl VfxQuality {
    /// Whether the post-process composite pass (bloom / distortion / lens effects) is active.
    #[inline]
    pub const fn has_post(self) -> bool {
        matches!(self, Self::Medium | Self::High)
    }

    /// Whether the emissive bloom chain is allocated and rendered.
    #[inline]
    pub const fn has_bloom(self) -> bool {
        matches!(self, Self::Medium | Self::High)
    }

    /// Number of active bloom mip levels (`0..=MAX_BLOOM_MIPS`), stepping down under thermal load.
    #[inline]
    pub const fn bloom_mips(self, under_load: bool) -> usize {
        match self {
            Self::Off | Self::Low => 0,
            Self::Medium => 3,
            Self::High => {
                if under_load {
                    3
                } else {
                    5
                }
            }
        }
    }

    /// Emissive buffer resolution divisor (`1` = full-res, `2` = half-res, `4` = quarter-res, `0` = off).
    #[inline]
    pub const fn emissive_divisor(self, under_load: bool) -> u32 {
        match self {
            Self::Off | Self::Low => 0,
            Self::Medium => {
                if under_load {
                    4
                } else {
                    2
                }
            }
            Self::High => {
                if under_load {
                    2
                } else {
                    1
                }
            }
        }
    }

    /// Whether screen-space distortion (`vfx_distort`) is enabled.
    #[inline]
    pub const fn has_distortion(self) -> bool {
        matches!(self, Self::High)
    }

    /// Whether radial zoom blur & chromatic aberration are enabled in `vfx_composite`.
    #[inline]
    pub const fn has_lens_effects(self) -> bool {
        matches!(self, Self::High)
    }

    /// Whether soft depth fade against scene geometry is enabled.
    #[inline]
    pub const fn has_soft_depth(self) -> bool {
        matches!(self, Self::Medium | Self::High)
    }

    /// Whether half-res volumetric smoke ray-marching (`vfx_smoke_volume`) is enabled.
    #[inline]
    pub const fn has_smoke_volume(self) -> bool {
        matches!(self, Self::High)
    }

    /// Ray-march step count for `vfx_smoke_volume` (`26` normal, `14` under thermal load).
    #[inline]
    pub const fn smoke_volume_steps(self, under_load: bool) -> f32 {
        if !self.has_smoke_volume() {
            0.0
        } else if under_load {
            14.0
        } else {
            26.0
        }
    }

    /// Whether `hie_ice` samples the copied scene color buffer for refraction (`Refract = 1.0`).
    #[inline]
    pub const fn has_ice_refraction(self) -> bool {
        matches!(self, Self::High)
    }

    /// `Parallax` uniform for `magu_magma` (`1.0` on High, `0.0` on Medium/Low).
    #[inline]
    pub const fn magma_parallax(self) -> f32 {
        if matches!(self, Self::High) { 1.0 } else { 0.0 }
    }
}

/// Identifier for all 15 Grand Line 0.4.0 WGSL shader programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VfxShaderId {
    LogiaBody,
    HieIce,
    MaguMagma,
    YomiSoulBody,
    VfxMesh,
    VfxParticle,
    VfxDistort,
    VfxDissolve,
    VfxSmoke,
    VfxSmokeTube,
    VfxBloomDown,
    VfxBloomUp,
    VfxComposite,
    VfxSmokeVolume,
    VfxSmokeComposite,
}

/// Complete ordered list of all 15 Grand Line VFX shader programs.
pub const ALL_SHADERS: [VfxShaderId; 15] = [
    VfxShaderId::LogiaBody,
    VfxShaderId::HieIce,
    VfxShaderId::MaguMagma,
    VfxShaderId::YomiSoulBody,
    VfxShaderId::VfxMesh,
    VfxShaderId::VfxParticle,
    VfxShaderId::VfxDistort,
    VfxShaderId::VfxDissolve,
    VfxShaderId::VfxSmoke,
    VfxShaderId::VfxSmokeTube,
    VfxShaderId::VfxBloomDown,
    VfxShaderId::VfxBloomUp,
    VfxShaderId::VfxComposite,
    VfxShaderId::VfxSmokeVolume,
    VfxShaderId::VfxSmokeComposite,
];

impl VfxShaderId {
    /// Canonical shader program name matching `assets/devilfruits/shaders/core/<name>.json`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::LogiaBody => "logia_body",
            Self::HieIce => "hie_ice",
            Self::MaguMagma => "magu_magma",
            Self::YomiSoulBody => "yomi_soul_body",
            Self::VfxMesh => "vfx_mesh",
            Self::VfxParticle => "vfx_particle",
            Self::VfxDistort => "vfx_distort",
            Self::VfxDissolve => "vfx_dissolve",
            Self::VfxSmoke => "vfx_smoke",
            Self::VfxSmokeTube => "vfx_smoke_tube",
            Self::VfxBloomDown => "vfx_bloom_down",
            Self::VfxBloomUp => "vfx_bloom_up",
            Self::VfxComposite => "vfx_composite",
            Self::VfxSmokeVolume => "vfx_smoke_volume",
            Self::VfxSmokeComposite => "vfx_smoke_composite",
        }
    }

    /// Embedded WGSL source for this shader program.
    pub const fn wgsl_source(self) -> &'static str {
        match self {
            Self::LogiaBody => include_str!("shaders/logia_body.wgsl"),
            Self::HieIce => include_str!("shaders/hie_ice.wgsl"),
            Self::MaguMagma => include_str!("shaders/magu_magma.wgsl"),
            Self::YomiSoulBody => include_str!("shaders/yomi_soul_body.wgsl"),
            Self::VfxMesh => include_str!("shaders/vfx_mesh.wgsl"),
            Self::VfxParticle => include_str!("shaders/vfx_particle.wgsl"),
            Self::VfxDistort => include_str!("shaders/vfx_distort.wgsl"),
            Self::VfxDissolve => include_str!("shaders/vfx_dissolve.wgsl"),
            Self::VfxSmoke => include_str!("shaders/vfx_smoke.wgsl"),
            Self::VfxSmokeTube => include_str!("shaders/vfx_smoke_tube.wgsl"),
            Self::VfxBloomDown => include_str!("shaders/vfx_bloom_down.wgsl"),
            Self::VfxBloomUp => include_str!("shaders/vfx_bloom_up.wgsl"),
            Self::VfxComposite => include_str!("shaders/vfx_composite.wgsl"),
            Self::VfxSmokeVolume => include_str!("shaders/vfx_smoke_volume.wgsl"),
            Self::VfxSmokeComposite => include_str!("shaders/vfx_smoke_composite.wgsl"),
        }
    }

    /// Whether this shader uses a vertex-buffer-less fullscreen triangle (`vs_main(@builtin(vertex_index))`).
    pub const fn is_screen_space(self) -> bool {
        matches!(
            self,
            Self::VfxBloomDown
                | Self::VfxBloomUp
                | Self::VfxComposite
                | Self::VfxSmokeVolume
                | Self::VfxSmokeComposite
        )
    }

    /// Name of the uniform struct declared inside the WGSL module.
    pub const fn uniform_struct_name(self) -> &'static str {
        match self {
            Self::LogiaBody => "LogiaBodyUniform",
            Self::HieIce => "HieIceUniform",
            Self::MaguMagma => "MaguMagmaUniform",
            Self::YomiSoulBody => "YomiSoulBodyUniform",
            Self::VfxMesh => "VfxMeshUniform",
            Self::VfxParticle => "VfxParticleUniform",
            Self::VfxDistort => "VfxDistortUniform",
            Self::VfxDissolve => "VfxDissolveUniform",
            Self::VfxSmoke => "VfxSmokeUniform",
            Self::VfxSmokeTube => "VfxSmokeTubeUniform",
            Self::VfxBloomDown => "VfxBloomDownUniform",
            Self::VfxBloomUp => "VfxBloomUpUniform",
            Self::VfxComposite => "VfxCompositeUniform",
            Self::VfxSmokeVolume => "VfxSmokeVolumeUniform",
            Self::VfxSmokeComposite => "VfxSmokeCompositeUniform",
        }
    }

    /// Exact Rust `#[repr(C)]` byte size of the corresponding UBO struct.
    pub const fn uniform_byte_size(self) -> usize {
        match self {
            Self::LogiaBody => std::mem::size_of::<LogiaBodyUniform>(),
            Self::HieIce => std::mem::size_of::<HieIceUniform>(),
            Self::MaguMagma => std::mem::size_of::<MaguMagmaUniform>(),
            Self::YomiSoulBody => std::mem::size_of::<YomiSoulBodyUniform>(),
            Self::VfxMesh => std::mem::size_of::<VfxMeshUniform>(),
            Self::VfxParticle => std::mem::size_of::<VfxParticleUniform>(),
            Self::VfxDistort => std::mem::size_of::<VfxDistortUniform>(),
            Self::VfxDissolve => std::mem::size_of::<VfxDissolveUniform>(),
            Self::VfxSmoke => std::mem::size_of::<VfxSmokeUniform>(),
            Self::VfxSmokeTube => std::mem::size_of::<VfxSmokeTubeUniform>(),
            Self::VfxBloomDown => std::mem::size_of::<VfxBloomDownUniform>(),
            Self::VfxBloomUp => std::mem::size_of::<VfxBloomUpUniform>(),
            Self::VfxComposite => std::mem::size_of::<VfxCompositeUniform>(),
            Self::VfxSmokeVolume => std::mem::size_of::<VfxSmokeVolumeUniform>(),
            Self::VfxSmokeComposite => std::mem::size_of::<VfxSmokeCompositeUniform>(),
        }
    }
}
