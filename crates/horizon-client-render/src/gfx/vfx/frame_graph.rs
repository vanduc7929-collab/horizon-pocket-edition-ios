//! Complete Grand Line 0.4.0 wgpu/Metal VFX pipeline set, `.mcmeta` sampler cache,
//! and multi-pass frame graph (`VfxPipelineSet` + `VfxFrameGraph`).
//!
//! Executes the exact Grand Line 0.4.0 pass order on Horizon PE's reversed-Z Metal/wgpu renderer:
//! 1. Optional scene color snapshot for `hie_ice` refraction + custom entity passes
//! 2. Scene `Depth32Float` snapshot into `VfxTargets::depth_copy`
//! 3. Half-res volumetric smoke ray-march (`vfx_smoke_volume`) + 5-tap tent upsample (`vfx_smoke_composite`)
//! 4. Scene VFX geometry pass: `vfx_smoke_tube`, `vfx_smoke`, solid `vfx_mesh`, `vfx_particle`, translucent `vfx_mesh`
//! 5. HDR `Rgba16Float` emissive pass: `vfx_mesh` + `vfx_particle` (`Pass = 1.0`)
//! 6. Half-res `Rg16Float` additive distortion pass: `vfx_distort`
//! 7. Jimenez 13-tap bloom downsample (`vfx_bloom_down`) + 9-tap additive tent upsample (`vfx_bloom_up`)
//! 8. Scene color copy + fullscreen cinematic composite (`vfx_composite`)

use super::{
    ALL_SHADERS, VfxQuality, VfxShaderId,
    targets::{DEPTH_FORMAT, DISTORT_FORMAT, HDR_FORMAT, MAX_BLOOM_MIPS, VfxTargets},
    uniforms::{
        HieIceUniform, LogiaBodyUniform, MaguMagmaUniform, VfxBloomDownUniform, VfxBloomUpUniform,
        VfxCompositeUniform, VfxDissolveUniform, VfxDistortUniform, VfxMeshUniform,
        VfxParticleUniform, VfxSmokeCompositeUniform, VfxSmokeTubeUniform, VfxSmokeUniform,
        VfxSmokeVolumeUniform, YomiSoulBodyUniform,
    },
    vertex::VfxVertex,
};
use serde::Deserialize;

/// Premultiplied-alpha blend state (`ONE`, `ONE_MINUS_SRC_ALPHA` for both color and alpha),
/// matching `RenderSystem.blendFuncSeparate(ONE, ONE_MINUS_SRC_ALPHA, ONE, ONE_MINUS_SRC_ALPHA)`.
pub const PREMULTIPLIED_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Pure additive blend state (`ONE`, `ONE`), used by `vfx_distort` and `vfx_bloom_up`.
pub const ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Parsed `.png.mcmeta` sampler flags matching Minecraft's `TextureMetadataSection`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureSamplerSpec {
    /// `true` -> `FilterMode::Linear`, `false` -> `FilterMode::Nearest`.
    pub blur: bool,
    /// `true` -> `AddressMode::ClampToEdge`, `false` -> `AddressMode::Repeat`.
    pub clamp: bool,
}

impl TextureSamplerSpec {
    pub const NEAREST_REPEAT: Self = Self {
        blur: false,
        clamp: false,
    };
    pub const NEAREST_CLAMP: Self = Self {
        blur: false,
        clamp: true,
    };
    pub const LINEAR_REPEAT: Self = Self {
        blur: true,
        clamp: false,
    };
    pub const LINEAR_CLAMP: Self = Self {
        blur: true,
        clamp: true,
    };

    #[inline]
    pub const fn filter_mode(self) -> wgpu::FilterMode {
        if self.blur {
            wgpu::FilterMode::Linear
        } else {
            wgpu::FilterMode::Nearest
        }
    }

    #[inline]
    pub const fn address_mode(self) -> wgpu::AddressMode {
        if self.clamp {
            wgpu::AddressMode::ClampToEdge
        } else {
            wgpu::AddressMode::Repeat
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawMcmetaFile {
    #[serde(default)]
    texture: RawTextureSection,
}

#[derive(Debug, Default, Deserialize)]
struct RawTextureSection {
    #[serde(default)]
    blur: bool,
    #[serde(default)]
    clamp: bool,
}

/// Parser for Minecraft `.png.mcmeta` files controlling texture filtering (`blur`) and wrapping (`clamp`).
pub struct TextureMcmeta;

impl TextureMcmeta {
    /// Parse a `.png.mcmeta` JSON string into a [`TextureSamplerSpec`].
    /// Missing `texture`, `blur`, or `clamp` keys default to `false` (Nearest + Repeat),
    /// matching vanilla Minecraft's `TextureMetadataSection`.
    pub fn parse(json: &str) -> Result<TextureSamplerSpec, serde_json::Error> {
        let raw: RawMcmetaFile = serde_json::from_str(json)?;
        Ok(TextureSamplerSpec {
            blur: raw.texture.blur,
            clamp: raw.texture.clamp,
        })
    }

    /// Parse optional `.mcmeta` text, falling back to [`TextureSamplerSpec::NEAREST_REPEAT`]
    /// if absent or invalid.
    pub fn parse_or_default(json: Option<&str>) -> TextureSamplerSpec {
        json.and_then(|s| Self::parse(s).ok())
            .unwrap_or(TextureSamplerSpec::NEAREST_REPEAT)
    }
}

/// Pool of all 4 color sampler combinations from `.mcmeta` plus the non-filtering depth sampler.
pub struct VfxSamplerCache {
    pub nearest_repeat: wgpu::Sampler,
    pub nearest_clamp: wgpu::Sampler,
    pub linear_repeat: wgpu::Sampler,
    pub linear_clamp: wgpu::Sampler,
    /// Non-filtering nearest clamp sampler required for sampling `Depth32Float` in WGSL/Metal.
    pub depth_nearest_clamp: wgpu::Sampler,
}

impl VfxSamplerCache {
    pub fn new(device: &wgpu::Device) -> Self {
        let make = |label, spec: TextureSamplerSpec| {
            let filter = spec.filter_mode();
            let addr = spec.address_mode();
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: addr,
                address_mode_v: addr,
                address_mode_w: addr,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };
        Self {
            nearest_repeat: make("VFX nearest repeat", TextureSamplerSpec::NEAREST_REPEAT),
            nearest_clamp: make("VFX nearest clamp", TextureSamplerSpec::NEAREST_CLAMP),
            linear_repeat: make("VFX linear repeat", TextureSamplerSpec::LINEAR_REPEAT),
            linear_clamp: make("VFX linear clamp", TextureSamplerSpec::LINEAR_CLAMP),
            depth_nearest_clamp: make("VFX depth nearest clamp", TextureSamplerSpec::NEAREST_CLAMP),
        }
    }

    /// Look up the cached sampler matching a parsed [`TextureSamplerSpec`].
    #[inline]
    pub fn sampler_for(&self, spec: TextureSamplerSpec) -> &wgpu::Sampler {
        match (spec.blur, spec.clamp) {
            (false, false) => &self.nearest_repeat,
            (false, true) => &self.nearest_clamp,
            (true, false) => &self.linear_repeat,
            (true, true) => &self.linear_clamp,
        }
    }

    /// Look up the cached sampler directly from optional `.png.mcmeta` JSON contents.
    #[inline]
    pub fn sampler_for_mcmeta(&self, mcmeta_json: Option<&str>) -> &wgpu::Sampler {
        self.sampler_for(TextureMcmeta::parse_or_default(mcmeta_json))
    }
}

fn ubo_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    byte_size: usize,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(byte_size as u64),
        },
        count: None,
    }
}

fn tex2d_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    filterable: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn depth_tex_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    filtering: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Sampler(if filtering {
            wgpu::SamplerBindingType::Filtering
        } else {
            wgpu::SamplerBindingType::NonFiltering
        }),
        count: None,
    }
}

/// Bind group layouts (group 0) for all 15 Grand Line WGSL shaders.
pub struct VfxBindGroupLayouts {
    pub logia_body: wgpu::BindGroupLayout,
    pub hie_ice: wgpu::BindGroupLayout,
    pub magu_magma: wgpu::BindGroupLayout,
    pub yomi_soul_body: wgpu::BindGroupLayout,
    pub vfx_mesh: wgpu::BindGroupLayout,
    pub vfx_particle: wgpu::BindGroupLayout,
    pub vfx_distort: wgpu::BindGroupLayout,
    pub vfx_dissolve: wgpu::BindGroupLayout,
    pub vfx_smoke: wgpu::BindGroupLayout,
    pub vfx_smoke_tube: wgpu::BindGroupLayout,
    pub vfx_bloom_down: wgpu::BindGroupLayout,
    pub vfx_bloom_up: wgpu::BindGroupLayout,
    pub vfx_composite: wgpu::BindGroupLayout,
    pub vfx_smoke_volume: wgpu::BindGroupLayout,
    pub vfx_smoke_composite: wgpu::BindGroupLayout,
}

impl VfxBindGroupLayouts {
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::ShaderStages as S;
        let vf = S::VERTEX_FRAGMENT;
        let v = S::VERTEX;
        let f = S::FRAGMENT;

        let make = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            })
        };

        Self {
            logia_body: make(
                "Grand Line logia_body layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<LogiaBodyUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, v, false),
                    tex2d_entry(4, v, false),
                    tex2d_entry(5, f, true),
                    sampler_entry(6, f, true),
                ],
            ),
            hie_ice: make(
                "Grand Line hie_ice layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<HieIceUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, v, false),
                    tex2d_entry(4, f, true),
                    sampler_entry(5, f, true),
                    tex2d_entry(6, f, true),
                    sampler_entry(7, f, true),
                ],
            ),
            magu_magma: make(
                "Grand Line magu_magma layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<MaguMagmaUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, v, false),
                    tex2d_entry(4, f, true),
                    sampler_entry(5, f, true),
                    tex2d_entry(6, f, true),
                    sampler_entry(7, f, true),
                    tex2d_entry(8, f, true),
                    sampler_entry(9, f, true),
                ],
            ),
            yomi_soul_body: make(
                "Grand Line yomi_soul_body layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<YomiSoulBodyUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                ],
            ),
            vfx_mesh: make(
                "Grand Line vfx_mesh layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxMeshUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                    tex2d_entry(5, f, true),
                    sampler_entry(6, f, true),
                    depth_tex_entry(7, f),
                    sampler_entry(8, f, false),
                ],
            ),
            vfx_particle: make(
                "Grand Line vfx_particle layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxParticleUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                    tex2d_entry(5, f, true),
                    sampler_entry(6, f, true),
                    tex2d_entry(7, f, true),
                    sampler_entry(8, f, true),
                    depth_tex_entry(9, f),
                    sampler_entry(10, f, false),
                ],
            ),
            vfx_distort: make(
                "Grand Line vfx_distort layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxDistortUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                    depth_tex_entry(5, f),
                    sampler_entry(6, f, false),
                ],
            ),
            vfx_dissolve: make(
                "Grand Line vfx_dissolve layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxDissolveUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, v, false),
                    tex2d_entry(4, v, false),
                ],
            ),
            vfx_smoke: make(
                "Grand Line vfx_smoke layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxSmokeUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                    tex2d_entry(5, f, true),
                    sampler_entry(6, f, true),
                    depth_tex_entry(7, f),
                    sampler_entry(8, f, false),
                ],
            ),
            vfx_smoke_tube: make(
                "Grand Line vfx_smoke_tube layout",
                &[
                    ubo_entry(0, vf, std::mem::size_of::<VfxSmokeTubeUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    depth_tex_entry(3, f),
                    sampler_entry(4, f, false),
                ],
            ),
            vfx_bloom_down: make(
                "Grand Line vfx_bloom_down layout",
                &[
                    ubo_entry(0, f, std::mem::size_of::<VfxBloomDownUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                ],
            ),
            vfx_bloom_up: make(
                "Grand Line vfx_bloom_up layout",
                &[
                    ubo_entry(0, f, std::mem::size_of::<VfxBloomUpUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                ],
            ),
            vfx_composite: make(
                "Grand Line vfx_composite layout",
                &[
                    ubo_entry(0, f, std::mem::size_of::<VfxCompositeUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                    tex2d_entry(5, f, true),
                    sampler_entry(6, f, true),
                    tex2d_entry(7, f, true),
                    sampler_entry(8, f, true),
                ],
            ),
            vfx_smoke_volume: make(
                "Grand Line vfx_smoke_volume layout",
                &[
                    ubo_entry(0, f, std::mem::size_of::<VfxSmokeVolumeUniform>()),
                    depth_tex_entry(1, f),
                    sampler_entry(2, f, false),
                    tex2d_entry(3, f, true),
                    sampler_entry(4, f, true),
                ],
            ),
            vfx_smoke_composite: make(
                "Grand Line vfx_smoke_composite layout",
                &[
                    ubo_entry(0, f, std::mem::size_of::<VfxSmokeCompositeUniform>()),
                    tex2d_entry(1, f, true),
                    sampler_entry(2, f, true),
                ],
            ),
        }
    }

    /// Retrieve the bind group layout for a specific [`VfxShaderId`].
    pub fn layout_for(&self, id: VfxShaderId) -> &wgpu::BindGroupLayout {
        match id {
            VfxShaderId::LogiaBody => &self.logia_body,
            VfxShaderId::HieIce => &self.hie_ice,
            VfxShaderId::MaguMagma => &self.magu_magma,
            VfxShaderId::YomiSoulBody => &self.yomi_soul_body,
            VfxShaderId::VfxMesh => &self.vfx_mesh,
            VfxShaderId::VfxParticle => &self.vfx_particle,
            VfxShaderId::VfxDistort => &self.vfx_distort,
            VfxShaderId::VfxDissolve => &self.vfx_dissolve,
            VfxShaderId::VfxSmoke => &self.vfx_smoke,
            VfxShaderId::VfxSmokeTube => &self.vfx_smoke_tube,
            VfxShaderId::VfxBloomDown => &self.vfx_bloom_down,
            VfxShaderId::VfxBloomUp => &self.vfx_bloom_up,
            VfxShaderId::VfxComposite => &self.vfx_composite,
            VfxShaderId::VfxSmokeVolume => &self.vfx_smoke_volume,
            VfxShaderId::VfxSmokeComposite => &self.vfx_smoke_composite,
        }
    }
}

/// All compiled `wgpu::RenderPipeline`s covering all 15 Grand Line VFX & custom entity shaders
/// (including solid/translucent/emissive and refract/lite render-state variants).
pub struct VfxPipelineSet {
    pub scene_format: wgpu::TextureFormat,
    pub depth_compare: wgpu::CompareFunction,
    pub layouts: VfxBindGroupLayouts,
    pub samplers: VfxSamplerCache,

    pub logia_body: wgpu::RenderPipeline,
    pub hie_ice_refract: wgpu::RenderPipeline,
    pub hie_ice_lite: wgpu::RenderPipeline,
    pub magu_magma: wgpu::RenderPipeline,
    pub yomi_soul_body: wgpu::RenderPipeline,
    pub vfx_dissolve: wgpu::RenderPipeline,

    pub vfx_mesh_solid: wgpu::RenderPipeline,
    pub vfx_mesh_translucent: wgpu::RenderPipeline,
    pub vfx_mesh_emissive: wgpu::RenderPipeline,

    pub vfx_particle_scene: wgpu::RenderPipeline,
    pub vfx_particle_emissive: wgpu::RenderPipeline,

    pub vfx_distort: wgpu::RenderPipeline,
    pub vfx_smoke: wgpu::RenderPipeline,
    pub vfx_smoke_tube: wgpu::RenderPipeline,
    pub vfx_smoke_volume: wgpu::RenderPipeline,
    pub vfx_smoke_composite: wgpu::RenderPipeline,

    pub vfx_bloom_down: wgpu::RenderPipeline,
    pub vfx_bloom_up: wgpu::RenderPipeline,
    pub vfx_composite: wgpu::RenderPipeline,
}

impl VfxPipelineSet {
    /// Compile all 15 Grand Line shader programs using Horizon PE's reversed-Z (`GreaterEqual`) depth test.
    pub fn new(device: &wgpu::Device, scene_format: wgpu::TextureFormat) -> Self {
        Self::with_depth_compare(device, scene_format, wgpu::CompareFunction::GreaterEqual)
    }

    /// Compile all 15 Grand Line shader programs with a configurable depth compare function
    /// (`GreaterEqual` for Horizon PE reversed-Z, `LessEqual` for standard-Z).
    pub fn with_depth_compare(
        device: &wgpu::Device,
        scene_format: wgpu::TextureFormat,
        depth_compare: wgpu::CompareFunction,
    ) -> Self {
        let layouts = VfxBindGroupLayouts::new(device);
        let samplers = VfxSamplerCache::new(device);

        let mut modules: Vec<wgpu::ShaderModule> = Vec::with_capacity(ALL_SHADERS.len());
        for id in ALL_SHADERS {
            modules.push(device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(id.name()),
                source: wgpu::ShaderSource::Wgsl(id.wgsl_source().into()),
            }));
        }
        let module_of = |id: VfxShaderId| -> &wgpu::ShaderModule {
            let idx = ALL_SHADERS
                .iter()
                .position(|&s| s == id)
                .expect("every VfxShaderId is listed in ALL_SHADERS");
            &modules[idx]
        };

        let make_depth = |write: bool| {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(write),
                depth_compare: Some(depth_compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        };

        let build = |label: &str,
                     id: VfxShaderId,
                     target_format: wgpu::TextureFormat,
                     blend: Option<wgpu::BlendState>,
                     cull_mode: Option<wgpu::Face>,
                     depth_stencil: Option<wgpu::DepthStencilState>| {
            let bgl = layouts.layout_for(id);
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(bgl)],
                ..Default::default()
            });
            let vbuf = [Some(VfxVertex::layout())];
            let buffers: &[Option<wgpu::VertexBufferLayout<'_>>] = if id.is_screen_space() {
                &[]
            } else {
                &vbuf
            };
            let shader = module_of(id);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        Self {
            scene_format,
            depth_compare,
            logia_body: build(
                "Grand Line logia_body",
                VfxShaderId::LogiaBody,
                scene_format,
                Some(wgpu::BlendState::ALPHA_BLENDING),
                None,
                make_depth(true),
            ),
            hie_ice_refract: build(
                "Grand Line hie_ice (refract)",
                VfxShaderId::HieIce,
                scene_format,
                None,
                Some(wgpu::Face::Back),
                make_depth(true),
            ),
            hie_ice_lite: build(
                "Grand Line hie_ice (lite)",
                VfxShaderId::HieIce,
                scene_format,
                Some(wgpu::BlendState::ALPHA_BLENDING),
                Some(wgpu::Face::Back),
                make_depth(false),
            ),
            magu_magma: build(
                "Grand Line magu_magma",
                VfxShaderId::MaguMagma,
                scene_format,
                None,
                None,
                make_depth(true),
            ),
            yomi_soul_body: build(
                "Grand Line yomi_soul_body",
                VfxShaderId::YomiSoulBody,
                scene_format,
                None,
                Some(wgpu::Face::Back),
                make_depth(true),
            ),
            vfx_dissolve: build(
                "Grand Line vfx_dissolve",
                VfxShaderId::VfxDissolve,
                scene_format,
                None,
                None,
                make_depth(true),
            ),
            vfx_mesh_solid: build(
                "Grand Line vfx_mesh (solid)",
                VfxShaderId::VfxMesh,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                make_depth(true),
            ),
            vfx_mesh_translucent: build(
                "Grand Line vfx_mesh (translucent)",
                VfxShaderId::VfxMesh,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                make_depth(false),
            ),
            vfx_mesh_emissive: build(
                "Grand Line vfx_mesh (emissive)",
                VfxShaderId::VfxMesh,
                HDR_FORMAT,
                Some(PREMULTIPLIED_BLEND),
                None,
                None,
            ),
            vfx_particle_scene: build(
                "Grand Line vfx_particle (scene)",
                VfxShaderId::VfxParticle,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                make_depth(false),
            ),
            vfx_particle_emissive: build(
                "Grand Line vfx_particle (emissive)",
                VfxShaderId::VfxParticle,
                HDR_FORMAT,
                Some(PREMULTIPLIED_BLEND),
                None,
                None,
            ),
            vfx_distort: build(
                "Grand Line vfx_distort",
                VfxShaderId::VfxDistort,
                DISTORT_FORMAT,
                Some(ADDITIVE_BLEND),
                None,
                None,
            ),
            vfx_smoke: build(
                "Grand Line vfx_smoke",
                VfxShaderId::VfxSmoke,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                make_depth(false),
            ),
            vfx_smoke_tube: build(
                "Grand Line vfx_smoke_tube",
                VfxShaderId::VfxSmokeTube,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                make_depth(false),
            ),
            vfx_smoke_volume: build(
                "Grand Line vfx_smoke_volume",
                VfxShaderId::VfxSmokeVolume,
                HDR_FORMAT,
                None,
                None,
                None,
            ),
            vfx_smoke_composite: build(
                "Grand Line vfx_smoke_composite",
                VfxShaderId::VfxSmokeComposite,
                scene_format,
                Some(PREMULTIPLIED_BLEND),
                None,
                None,
            ),
            vfx_bloom_down: build(
                "Grand Line vfx_bloom_down",
                VfxShaderId::VfxBloomDown,
                HDR_FORMAT,
                None,
                None,
                None,
            ),
            vfx_bloom_up: build(
                "Grand Line vfx_bloom_up",
                VfxShaderId::VfxBloomUp,
                HDR_FORMAT,
                Some(ADDITIVE_BLEND),
                None,
                None,
            ),
            vfx_composite: build(
                "Grand Line vfx_composite",
                VfxShaderId::VfxComposite,
                scene_format,
                None,
                None,
                None,
            ),
            layouts,
            samplers,
        }
    }

    /// Create a bind group for `logia_body`.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_logia_body(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        skin: &wgpu::TextureView,
        skin_sampler: &wgpu::Sampler,
        overlay: &wgpu::TextureView,
        lightmap: &wgpu::TextureView,
        limb_map: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line logia_body BG"),
            layout: &self.layouts.logia_body,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(skin),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(skin_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(overlay),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(limb_map),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `hie_ice`.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_hie_ice(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        diffuse: &wgpu::TextureView,
        diffuse_sampler: &wgpu::Sampler,
        lightmap: &wgpu::TextureView,
        scene_copy: &wgpu::TextureView,
        detail: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line hie_ice BG"),
            layout: &self.layouts.hie_ice,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(diffuse),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(scene_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(detail),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.nearest_repeat),
                },
            ],
        })
    }

    /// Create a bind group for `magu_magma`.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_magu_magma(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        diffuse: &wgpu::TextureView,
        diffuse_sampler: &wgpu::Sampler,
        lightmap: &wgpu::TextureView,
        emissive_mask: &wgpu::TextureView,
        detail: &wgpu::TextureView,
        lut: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line magu_magma BG"),
            layout: &self.layouts.magu_magma,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(diffuse),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(emissive_mask),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(detail),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(lut),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `yomi_soul_body`.
    pub fn bind_yomi_soul_body(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        diffuse: &wgpu::TextureView,
        diffuse_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line yomi_soul_body BG"),
            layout: &self.layouts.yomi_soul_body,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(diffuse),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_dissolve`.
    pub fn bind_vfx_dissolve(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        diffuse: &wgpu::TextureView,
        diffuse_sampler: &wgpu::Sampler,
        overlay: &wgpu::TextureView,
        lightmap: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_dissolve BG"),
            layout: &self.layouts.vfx_dissolve,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(diffuse),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(overlay),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_mesh`.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_vfx_mesh(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        main_tex: &wgpu::TextureView,
        main_sampler: &wgpu::Sampler,
        noise_tex: &wgpu::TextureView,
        lut_tex: &wgpu::TextureView,
        depth_copy: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_mesh BG"),
            layout: &self.layouts.vfx_mesh,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(main_tex),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(main_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(noise_tex),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(lut_tex),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(depth_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_particle`.
    #[allow(clippy::too_many_arguments)]
    pub fn bind_vfx_particle(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        atlas: &wgpu::TextureView,
        pack: &wgpu::TextureView,
        pack2: &wgpu::TextureView,
        lut: &wgpu::TextureView,
        depth_copy: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_particle BG"),
            layout: &self.layouts.vfx_particle,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(pack),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(pack2),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(lut),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(depth_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_distort`.
    pub fn bind_vfx_distort(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        atlas: &wgpu::TextureView,
        heat_normal: &wgpu::TextureView,
        depth_copy: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_distort BG"),
            layout: &self.layouts.vfx_distort,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(heat_normal),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(depth_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_smoke`.
    pub fn bind_vfx_smoke(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        pos_atlas: &wgpu::TextureView,
        neg_atlas: &wgpu::TextureView,
        smoke_noise: &wgpu::TextureView,
        depth_copy: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_smoke BG"),
            layout: &self.layouts.vfx_smoke,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(pos_atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(neg_atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(smoke_noise),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(depth_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_smoke_tube`.
    pub fn bind_vfx_smoke_tube(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        smoke_noise: &wgpu::TextureView,
        depth_copy: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_smoke_tube BG"),
            layout: &self.layouts.vfx_smoke_tube,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(smoke_noise),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_copy),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_bloom_down`.
    pub fn bind_bloom_down(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        source: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_bloom_down BG"),
            layout: &self.layouts.vfx_bloom_down,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_bloom_up`.
    pub fn bind_bloom_up(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        source: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_bloom_up BG"),
            layout: &self.layouts.vfx_bloom_up,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_composite`.
    pub fn bind_composite(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        scene: &wgpu::TextureView,
        distort: &wgpu::TextureView,
        emissive: &wgpu::TextureView,
        bloom: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_composite BG"),
            layout: &self.layouts.vfx_composite,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(scene),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(distort),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(emissive),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::TextureView(bloom),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_smoke_volume`.
    pub fn bind_smoke_volume(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        depth: &wgpu::TextureView,
        volume_noise: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_smoke_volume BG"),
            layout: &self.layouts.vfx_smoke_volume,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.depth_nearest_clamp),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(volume_noise),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_repeat),
                },
            ],
        })
    }

    /// Create a bind group for `vfx_smoke_composite`.
    pub fn bind_smoke_composite(
        &self,
        device: &wgpu::Device,
        ubo: &wgpu::Buffer,
        smoke_half: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grand Line vfx_smoke_composite BG"),
            layout: &self.layouts.vfx_smoke_composite,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(smoke_half),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.linear_clamp),
                },
            ],
        })
    }
}

/// Geometry draw command for a VFX vertex/index batch with a pre-built bind group.
pub struct VfxDrawBatch<'a> {
    pub bind_group: &'a wgpu::BindGroup,
    pub vertex_buffer: wgpu::BufferSlice<'a>,
    pub index_buffer: Option<(wgpu::BufferSlice<'a>, wgpu::IndexFormat)>,
    /// Number of vertices (when non-indexed) or indices (when indexed).
    pub count: u32,
}

impl<'a> VfxDrawBatch<'a> {
    fn submit(&self, pass: &mut wgpu::RenderPass<'a>) {
        if self.count == 0 {
            return;
        }
        pass.set_bind_group(0, self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer);
        if let Some((ib, fmt)) = self.index_buffer {
            pass.set_index_buffer(ib, fmt);
            pass.draw_indexed(0..self.count, 0, 0..1);
        } else {
            pass.draw(0..self.count, 0..1);
        }
    }
}

/// Custom Grand Line entity shader pass variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityShaderVariant {
    LogiaBody,
    HieIceRefract,
    HieIceLite,
    MaguMagma,
    YomiSoulBody,
    VfxDissolve,
}

/// Draw batch for custom Grand Line entity passes.
pub struct EntityDrawBatch<'a> {
    pub variant: EntityShaderVariant,
    pub batch: VfxDrawBatch<'a>,
}

/// Inputs for the half-res volumetric smoke pass (`vfx_smoke_volume` + `vfx_smoke_composite`).
pub struct SmokeVolumePassParams<'a> {
    pub uniform: VfxSmokeVolumeUniform,
    pub volume_noise_view: &'a wgpu::TextureView,
}

/// Parameters controlling the bloom + final cinematic composite passes (`VfxPost`).
#[derive(Clone, Copy, Debug)]
pub struct CompositeParams {
    /// Master bloom strength (`0.0` disables bloom downsample/upsample).
    pub bloom_strength: f32,
    /// Soft-knee luminance threshold for the first bloom downsample pass (`0.22` in Grand Line).
    pub bloom_threshold: f32,
    /// Tent upsample filter radius (`1.0` in Grand Line).
    pub bloom_radius: f32,
    /// Per-level upsample accumulation intensity (`0.9` in Grand Line).
    pub bloom_up_intensity: f32,
    /// Composite shader uniforms (`InvSize`, `DistortOn`, `BloomIntensity` are finalized automatically).
    pub composite: VfxCompositeUniform,
}

impl Default for CompositeParams {
    fn default() -> Self {
        Self {
            bloom_strength: 1.0,
            bloom_threshold: 0.22,
            bloom_radius: 1.0,
            bloom_up_intensity: 0.9,
            composite: VfxCompositeUniform::default(),
        }
    }
}

impl CompositeParams {
    /// Whether any full-screen pulse (radial blur, chromatic aberration, flash, grade, vignette, desat) is active.
    pub fn has_screen_pulses(&self) -> bool {
        let c = &self.composite;
        c.radial_chroma[2] > 0.0005
            || c.radial_chroma[3].abs() > 1.0e-5
            || c.flash[3] > 0.0
            || c.grade[3] > 0.0
            || c.bloom_distort_vig_desat[2] > 0.0
            || c.bloom_distort_vig_desat[3] > 0.0
    }
}

/// All per-frame geometry batches and parameters consumed by [`VfxFrameGraph::execute`].
pub struct VfxFrameInputs<'a> {
    pub scene_color_texture: &'a wgpu::Texture,
    pub scene_color_view: &'a wgpu::TextureView,
    pub scene_depth_texture: &'a wgpu::Texture,
    pub scene_depth_view: &'a wgpu::TextureView,
    pub scene_size: [u32; 2],
    pub quality: VfxQuality,
    pub under_load: bool,

    pub smoke_volume: Option<SmokeVolumePassParams<'a>>,
    pub smoke_tubes: &'a [VfxDrawBatch<'a>],
    pub smoke_puffs: &'a [VfxDrawBatch<'a>],

    pub meshes_solid_scene: &'a [VfxDrawBatch<'a>],
    pub particles_scene: &'a [VfxDrawBatch<'a>],
    pub meshes_translucent_scene: &'a [VfxDrawBatch<'a>],

    pub meshes_emissive: &'a [VfxDrawBatch<'a>],
    pub particles_emissive: &'a [VfxDrawBatch<'a>],

    pub distort: &'a [VfxDrawBatch<'a>],
    pub post: CompositeParams,
}

/// Summary of which passes executed during a frame (useful for profiling and tests).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VfxFrameStats {
    pub smoke_volume_drawn: bool,
    pub scene_vfx_drawn: bool,
    pub emissive_drawn: bool,
    pub distort_drawn: bool,
    pub bloom_mips_executed: usize,
    pub composite_drawn: bool,
}

/// Multi-pass Grand Line 0.4.0 VFX frame graph owning pipelines, offscreen targets,
/// and persistent per-pass uniform buffers.
pub struct VfxFrameGraph {
    pub pipelines: VfxPipelineSet,
    pub targets: VfxTargets,

    smoke_volume_ubo: wgpu::Buffer,
    smoke_composite_ubo: wgpu::Buffer,
    bloom_down_ubos: [wgpu::Buffer; MAX_BLOOM_MIPS],
    bloom_up_ubos: [wgpu::Buffer; MAX_BLOOM_MIPS],
    composite_ubo: wgpu::Buffer,
}

impl VfxFrameGraph {
    /// Create all 15 shader pipelines, sampler cache, and reusable pass UBOs.
    pub fn new(device: &wgpu::Device, scene_format: wgpu::TextureFormat) -> Self {
        Self::with_depth_compare(device, scene_format, wgpu::CompareFunction::GreaterEqual)
    }

    /// Create the frame graph with a custom depth compare function.
    pub fn with_depth_compare(
        device: &wgpu::Device,
        scene_format: wgpu::TextureFormat,
        depth_compare: wgpu::CompareFunction,
    ) -> Self {
        let make_ubo = |label: &str, size: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let bloom_down_ubos = std::array::from_fn(|i| {
            make_ubo(
                &format!("Grand Line bloom_down UBO {i}"),
                std::mem::size_of::<VfxBloomDownUniform>(),
            )
        });
        let bloom_up_ubos = std::array::from_fn(|i| {
            make_ubo(
                &format!("Grand Line bloom_up UBO {i}"),
                std::mem::size_of::<VfxBloomUpUniform>(),
            )
        });

        Self {
            pipelines: VfxPipelineSet::with_depth_compare(device, scene_format, depth_compare),
            targets: VfxTargets::new(),
            smoke_volume_ubo: make_ubo(
                "Grand Line smoke_volume UBO",
                std::mem::size_of::<VfxSmokeVolumeUniform>(),
            ),
            smoke_composite_ubo: make_ubo(
                "Grand Line smoke_composite UBO",
                std::mem::size_of::<VfxSmokeCompositeUniform>(),
            ),
            bloom_down_ubos,
            bloom_up_ubos,
            composite_ubo: make_ubo(
                "Grand Line composite UBO",
                std::mem::size_of::<VfxCompositeUniform>(),
            ),
        }
    }

    /// Ensure offscreen targets are allocated for `scene_size` and `quality` and copy the
    /// opaque world color texture into `targets.scene_copy` for `hie_ice` refraction.
    pub fn prepare_ice_refraction(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_color_texture: &wgpu::Texture,
        scene_size: [u32; 2],
        quality: VfxQuality,
        under_load: bool,
    ) -> bool {
        self.targets.ensure(
            device,
            scene_size,
            self.pipelines.scene_format,
            quality,
            under_load,
        );
        if quality.has_ice_refraction() {
            self.targets.copy_scene_color(encoder, scene_color_texture)
        } else {
            false
        }
    }

    /// Draw custom Grand Line entity batches (`logia_body`, `hie_ice`, `magu_magma`,
    /// `yomi_soul_body`, `vfx_dissolve`) inside the caller's active world render pass.
    pub fn draw_entities<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        batches: &'a [EntityDrawBatch<'a>],
    ) {
        for item in batches {
            let pipeline = match item.variant {
                EntityShaderVariant::LogiaBody => &self.pipelines.logia_body,
                EntityShaderVariant::HieIceRefract => &self.pipelines.hie_ice_refract,
                EntityShaderVariant::HieIceLite => &self.pipelines.hie_ice_lite,
                EntityShaderVariant::MaguMagma => &self.pipelines.magu_magma,
                EntityShaderVariant::YomiSoulBody => &self.pipelines.yomi_soul_body,
                EntityShaderVariant::VfxDissolve => &self.pipelines.vfx_dissolve,
            };
            pass.set_pipeline(pipeline);
            item.batch.submit(pass);
        }
    }

    /// Execute the complete multi-pass Grand Line VFX frame graph for one frame.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: &VfxFrameInputs<'_>,
    ) -> VfxFrameStats {
        let mut stats = VfxFrameStats::default();
        if inputs.quality == VfxQuality::Off {
            self.targets.clear();
            return stats;
        }

        self.targets.ensure(
            device,
            inputs.scene_size,
            self.pipelines.scene_format,
            inputs.quality,
            inputs.under_load,
        );

        let has_smoke_volume = inputs.quality.has_smoke_volume()
            && inputs
                .smoke_volume
                .as_ref()
                .is_some_and(|v| v.uniform.full_size_count_steps[2] > 0.5);
        let has_scene_vfx = !inputs.smoke_tubes.is_empty()
            || !inputs.smoke_puffs.is_empty()
            || !inputs.meshes_solid_scene.is_empty()
            || !inputs.particles_scene.is_empty()
            || !inputs.meshes_translucent_scene.is_empty();
        let has_emissive_geom =
            !inputs.meshes_emissive.is_empty() || !inputs.particles_emissive.is_empty();
        let has_distort = inputs.quality.has_distortion() && !inputs.distort.is_empty();
        let has_pulses = inputs.post.has_screen_pulses();

        if !has_smoke_volume && !has_scene_vfx && !has_emissive_geom && !has_distort && !has_pulses
        {
            return stats;
        }

        // 1. Copy scene depth into `depth_copy` so shaders can sample scene depth cleanly.
        if has_smoke_volume || has_scene_vfx || has_emissive_geom || has_distort {
            self.targets
                .copy_scene_depth(encoder, inputs.scene_depth_texture);
        }

        // 2. Half-res volumetric smoke ray-march + 5-tap tent upsample over the scene.
        if has_smoke_volume {
            if let (Some(sv_params), Some(half_target), Some(depth_target)) = (
                &inputs.smoke_volume,
                self.targets.smoke_volume(),
                self.targets.depth_copy(),
            ) {
                queue.write_buffer(
                    &self.smoke_volume_ubo,
                    0,
                    bytemuck::bytes_of(&sv_params.uniform),
                );
                let vol_bg = self.pipelines.bind_smoke_volume(
                    device,
                    &self.smoke_volume_ubo,
                    &depth_target.view,
                    sv_params.volume_noise_view,
                );
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Grand Line VFX smoke volume raymarch"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &half_target.view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&self.pipelines.vfx_smoke_volume);
                    pass.set_bind_group(0, &vol_bg, &[]);
                    pass.draw(0..3, 0..1);
                }

                let comp_ubo_data = VfxSmokeCompositeUniform {
                    src_texel: [half_target.inv_size()[0], half_target.inv_size()[1], 0.0, 0.0],
                };
                queue.write_buffer(
                    &self.smoke_composite_ubo,
                    0,
                    bytemuck::bytes_of(&comp_ubo_data),
                );
                let comp_bg = self.pipelines.bind_smoke_composite(
                    device,
                    &self.smoke_composite_ubo,
                    &half_target.view,
                );
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Grand Line VFX smoke volume composite"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: inputs.scene_color_view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&self.pipelines.vfx_smoke_composite);
                    pass.set_bind_group(0, &comp_bg, &[]);
                    pass.draw(0..3, 0..1);
                }
                stats.smoke_volume_drawn = true;
            }
        }

        // 3. Scene VFX geometry pass (smoke tubes -> smoke puffs -> solid meshes -> particles -> translucent meshes).
        if has_scene_vfx {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Grand Line VFX scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: inputs.scene_color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: inputs.scene_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            if !inputs.smoke_tubes.is_empty() {
                pass.set_pipeline(&self.pipelines.vfx_smoke_tube);
                for batch in inputs.smoke_tubes {
                    batch.submit(&mut pass);
                }
            }
            if !inputs.smoke_puffs.is_empty() {
                pass.set_pipeline(&self.pipelines.vfx_smoke);
                for batch in inputs.smoke_puffs {
                    batch.submit(&mut pass);
                }
            }
            if !inputs.meshes_solid_scene.is_empty() {
                pass.set_pipeline(&self.pipelines.vfx_mesh_solid);
                for batch in inputs.meshes_solid_scene {
                    batch.submit(&mut pass);
                }
            }
            if !inputs.particles_scene.is_empty() {
                pass.set_pipeline(&self.pipelines.vfx_particle_scene);
                for batch in inputs.particles_scene {
                    batch.submit(&mut pass);
                }
            }
            if !inputs.meshes_translucent_scene.is_empty() {
                pass.set_pipeline(&self.pipelines.vfx_mesh_translucent);
                for batch in inputs.meshes_translucent_scene {
                    batch.submit(&mut pass);
                }
            }
            stats.scene_vfx_drawn = true;
        }

        // 4. HDR emissive pass (`Pass = 1.0`, manual depth test in fragment shader).
        let run_emissive = inputs.quality.has_post()
            && inputs.post.bloom_strength > 0.0
            && has_emissive_geom;
        if run_emissive {
            if let Some(emissive_target) = self.targets.emissive() {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Grand Line VFX emissive pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &emissive_target.view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                if !inputs.meshes_emissive.is_empty() {
                    pass.set_pipeline(&self.pipelines.vfx_mesh_emissive);
                    for batch in inputs.meshes_emissive {
                        batch.submit(&mut pass);
                    }
                }
                if !inputs.particles_emissive.is_empty() {
                    pass.set_pipeline(&self.pipelines.vfx_particle_emissive);
                    for batch in inputs.particles_emissive {
                        batch.submit(&mut pass);
                    }
                }
                stats.emissive_drawn = true;
            }
        }

        // 5. Half-res additive distortion pass (`Rg16Float`).
        if has_distort {
            if let Some(distort_target) = self.targets.distort() {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Grand Line VFX distort pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &distort_target.view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&self.pipelines.vfx_distort);
                for batch in inputs.distort {
                    batch.submit(&mut pass);
                }
                stats.distort_drawn = true;
            }
        }

        // 6. Jimenez bloom downsample + additive tent upsample chain.
        let bloom_mips = self.targets.bloom_mips().len();
        let run_bloom = stats.emissive_drawn && inputs.post.bloom_strength > 0.0 && bloom_mips > 0;
        if run_bloom {
            if let Some(emissive_target) = self.targets.emissive() {
                // Downsample: emissive -> bloom[0] -> ... -> bloom[mips - 1]
                let mut src_view = &emissive_target.view;
                let mut src_inv = emissive_target.inv_size();
                for i in 0..bloom_mips {
                    let dst = &self.targets.bloom_mips()[i];
                    let down_ubo = VfxBloomDownUniform {
                        params: [
                            src_inv[0],
                            src_inv[1],
                            inputs.post.bloom_threshold,
                            if i == 0 { 1.0 } else { 0.0 },
                        ],
                    };
                    queue.write_buffer(&self.bloom_down_ubos[i], 0, bytemuck::bytes_of(&down_ubo));
                    let bg = self.pipelines.bind_bloom_down(
                        device,
                        &self.bloom_down_ubos[i],
                        src_view,
                    );
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Grand Line VFX bloom down"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &dst.view,
                                resolve_target: None,
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&self.pipelines.vfx_bloom_down);
                        pass.set_bind_group(0, &bg, &[]);
                        pass.draw(0..3, 0..1);
                    }
                    src_view = &dst.view;
                    src_inv = dst.inv_size();
                }

                // Upsample: bloom[mips - 1] -> bloom[mips - 2] -> ... -> bloom[0] (additive)
                if bloom_mips >= 2 {
                    for i in (0..bloom_mips - 1).rev() {
                        let src = &self.targets.bloom_mips()[i + 1];
                        let dst = &self.targets.bloom_mips()[i];
                        let up_ubo = VfxBloomUpUniform {
                            params: [
                                src.inv_size()[0],
                                src.inv_size()[1],
                                inputs.post.bloom_radius,
                                inputs.post.bloom_up_intensity,
                            ],
                        };
                        queue.write_buffer(&self.bloom_up_ubos[i], 0, bytemuck::bytes_of(&up_ubo));
                        let bg = self.pipelines.bind_bloom_up(
                            device,
                            &self.bloom_up_ubos[i],
                            &src.view,
                        );
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Grand Line VFX bloom up"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &dst.view,
                                resolve_target: None,
                                depth_slice: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(&self.pipelines.vfx_bloom_up);
                        pass.set_bind_group(0, &bg, &[]);
                        pass.draw(0..3, 0..1);
                    }
                }
                stats.bloom_mips_executed = bloom_mips;
            }
        }

        // 7. Final cinematic composite pass (copies scene -> `scene_copy`, writes composite -> `scene_color_view`).
        if inputs.quality.has_post() && (run_bloom || stats.distort_drawn || has_pulses) {
            if self
                .targets
                .copy_scene_color(encoder, inputs.scene_color_texture)
            {
                if let (Some(scene_copy), Some(distort_view), Some(emissive_view), Some(bloom_view)) = (
                    self.targets.scene_copy(),
                    self.targets.distort_or_fallback(),
                    self.targets.emissive_or_fallback(),
                    self.targets.bloom0_or_fallback(),
                ) {
                    let mut comp = inputs.post.composite;
                    if !inputs.quality.has_lens_effects() {
                        comp.radial_chroma[2] = 0.0;
                        comp.radial_chroma[3] = 0.0;
                    }
                    if !run_bloom {
                        comp.bloom_distort_vig_desat[0] = 0.0;
                    }
                    comp.bloom_distort_vig_desat[1] = if stats.distort_drawn { 1.0 } else { 0.0 };
                    comp.inv_size = [scene_copy.inv_size()[0], scene_copy.inv_size()[1], 0.0, 0.0];

                    queue.write_buffer(&self.composite_ubo, 0, bytemuck::bytes_of(&comp));
                    let bg = self.pipelines.bind_composite(
                        device,
                        &self.composite_ubo,
                        &scene_copy.view,
                        distort_view,
                        emissive_view,
                        bloom_view,
                    );
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Grand Line VFX composite"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: inputs.scene_color_view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&self.pipelines.vfx_composite);
                    pass.set_bind_group(0, &bg, &[]);
                    pass.draw(0..3, 0..1);
                    stats.composite_drawn = true;
                }
            }
        }

        stats
    }
}
