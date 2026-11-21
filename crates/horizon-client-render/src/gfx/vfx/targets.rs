//! Offscreen GPU render target pool for Grand Line's VFX frame graph (`VfxTargets`).
//!
//! Allocates and resizes:
//! - Half-resolution `Rgba16Float` volumetric smoke buffer (`vfx_smoke_volume`)
//! - Full-resolution (High) or half-resolution (Medium / thermal load) `Rgba16Float` emissive buffer
//! - Half-resolution `Rg16Float` distortion offset buffer (`vfx_distort`)
//! - Bloom mip chain of up to [`MAX_BLOOM_MIPS`] (`6`) `Rgba16Float` levels
//! - Full-resolution scene color copy (shared by `hie_ice` refraction and `vfx_composite`)
//! - Full-resolution `Depth32Float` scene depth copy (for soft depth fade & manual depth tests)
//!
//! Old targets are dropped before allocating new ones so peak iOS memory never doubles on resize.

use super::VfxQuality;

/// HDR floating-point format used for emissive, bloom chain, and volumetric smoke buffers.
pub const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Signed half-float 2-channel format used for half-res screen-space distortion offsets.
pub const DISTORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// Depth texture format matching Horizon PE's reversed-Z world depth buffer.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Maximum number of bloom mip levels supported by [`VfxTargets`].
pub const MAX_BLOOM_MIPS: usize = 6;

/// Single 2D offscreen texture + default view + dimensions.
pub struct VfxTextureTarget {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: [u32; 2],
    pub format: wgpu::TextureFormat,
}

impl VfxTextureTarget {
    fn new(
        device: &wgpu::Device,
        label: &str,
        size: [u32; 2],
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        let size = [size[0].max(1), size[1].max(1)];
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
            format,
        }
    }

    /// Texel inverse size `(1.0 / width, 1.0 / height)`.
    #[inline]
    pub fn inv_size(&self) -> [f32; 2] {
        [1.0 / self.size[0] as f32, 1.0 / self.size[1] as f32]
    }

    /// Estimated GPU memory footprint in bytes.
    pub fn byte_size(&self) -> usize {
        let bpp = match self.format {
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rg16Float | wgpu::TextureFormat::Depth32Float => 4,
            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
            | wgpu::TextureFormat::Rgb10a2Unorm => 4,
            _ => 4,
        };
        self.size[0] as usize * self.size[1] as usize * bpp
    }
}

/// Manages all offscreen render targets for Grand Line's VFX pipeline with iOS quality scaling.
pub struct VfxTargets {
    scene_size: [u32; 2],
    scene_format: wgpu::TextureFormat,
    quality: VfxQuality,
    under_load: bool,
    requested_bloom_mips: usize,

    smoke_volume: Option<VfxTextureTarget>,
    emissive: Option<VfxTextureTarget>,
    distort: Option<VfxTextureTarget>,
    bloom: Vec<VfxTextureTarget>,
    scene_copy: Option<VfxTextureTarget>,
    depth_copy: Option<VfxTextureTarget>,

    // 1x1 zeroed fallback targets used when optional passes (bloom/distort) are disabled
    // so `vfx_composite` always binds valid views with matching formats.
    fallback_hdr: Option<VfxTextureTarget>,
    fallback_distort: Option<VfxTextureTarget>,
}

impl Default for VfxTargets {
    fn default() -> Self {
        Self::new()
    }
}

impl VfxTargets {
    /// Create an empty target pool; call [`Self::ensure`] before rendering a frame.
    pub fn new() -> Self {
        Self {
            scene_size: [0, 0],
            scene_format: wgpu::TextureFormat::Bgra8Unorm,
            quality: VfxQuality::Off,
            under_load: false,
            requested_bloom_mips: 0,
            smoke_volume: None,
            emissive: None,
            distort: None,
            bloom: Vec::new(),
            scene_copy: None,
            depth_copy: None,
            fallback_hdr: None,
            fallback_distort: None,
        }
    }

    /// Release all offscreen GPU textures immediately (e.g. quality set to `Off` or world exit).
    pub fn clear(&mut self) {
        self.smoke_volume = None;
        self.emissive = None;
        self.distort = None;
        self.bloom.clear();
        self.scene_copy = None;
        self.depth_copy = None;
        self.scene_size = [0, 0];
        self.requested_bloom_mips = 0;
    }

    /// Ensure all offscreen targets match the given viewport size, scene color format,
    /// quality tier, and thermal load state. No-op when configuration is unchanged.
    pub fn ensure(
        &mut self,
        device: &wgpu::Device,
        scene_size: [u32; 2],
        scene_format: wgpu::TextureFormat,
        quality: VfxQuality,
        under_load: bool,
    ) {
        let mips = quality.bloom_mips(under_load);
        self.ensure_with_bloom_mips(device, scene_size, scene_format, quality, under_load, mips);
    }

    /// Same as [`Self::ensure`], with an explicit bloom mip count clamped to `0..=MAX_BLOOM_MIPS`.
    pub fn ensure_with_bloom_mips(
        &mut self,
        device: &wgpu::Device,
        scene_size: [u32; 2],
        scene_format: wgpu::TextureFormat,
        quality: VfxQuality,
        under_load: bool,
        bloom_mips: usize,
    ) {
        let clamped_size = [scene_size[0].max(1), scene_size[1].max(1)];
        let mips = if quality.has_bloom() {
            bloom_mips.min(MAX_BLOOM_MIPS)
        } else {
            0
        };

        if quality == VfxQuality::Off {
            self.clear();
            self.quality = VfxQuality::Off;
            self.under_load = under_load;
            return;
        }

        if self.scene_size == clamped_size
            && self.scene_format == scene_format
            && self.quality == quality
            && self.under_load == under_load
            && self.requested_bloom_mips == mips
            && self.fallback_hdr.is_some()
        {
            return;
        }

        // Drop previous targets BEFORE allocating new ones to prevent memory spikes on iOS.
        self.clear();

        self.scene_size = clamped_size;
        self.scene_format = scene_format;
        self.quality = quality;
        self.under_load = under_load;
        self.requested_bloom_mips = mips;

        let rt_sampled = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST;

        if self.fallback_hdr.is_none() {
            self.fallback_hdr = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX fallback HDR",
                [1, 1],
                HDR_FORMAT,
                rt_sampled,
            ));
        }
        if self.fallback_distort.is_none() {
            self.fallback_distort = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX fallback distort",
                [1, 1],
                DISTORT_FORMAT,
                rt_sampled,
            ));
        }

        let [w, h] = clamped_size;
        let half = [(w / 2).max(1), (h / 2).max(1)];

        // Depth copy is needed for soft particles, emissive depth test, distortion, and smoke volumes.
        if quality.has_soft_depth() || quality.has_distortion() || quality.has_smoke_volume() {
            self.depth_copy = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX depth copy",
                clamped_size,
                DEPTH_FORMAT,
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            ));
        }

        // Scene color copy: shared by `hie_ice` refraction and `vfx_composite`.
        if quality.has_post() || quality.has_ice_refraction() {
            self.scene_copy = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX scene copy",
                clamped_size,
                scene_format,
                rt_sampled,
            ));
        }

        // Half-res Rgba16Float volumetric smoke ray-march buffer.
        if quality.has_smoke_volume() {
            self.smoke_volume = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX smoke volume",
                half,
                HDR_FORMAT,
                rt_sampled,
            ));
        }

        // Emissive HDR target (full-res on High, half-res on Medium / thermal load).
        let emissive_div = quality.emissive_divisor(under_load);
        if emissive_div > 0 {
            let e_size = [(w / emissive_div).max(1), (h / emissive_div).max(1)];
            self.emissive = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX emissive",
                e_size,
                HDR_FORMAT,
                rt_sampled,
            ));
        }

        // Half-res Rg16Float distortion buffer.
        if quality.has_distortion() {
            self.distort = Some(VfxTextureTarget::new(
                device,
                "Grand Line VFX distort",
                half,
                DISTORT_FORMAT,
                rt_sampled,
            ));
        }

        // Bloom mip chain: up to 6 progressively halved Rgba16Float levels.
        // Level 0 starts at half of the emissive target (or half screen), halving each step.
        if mips > 0 {
            let base = self.emissive.as_ref().map_or(clamped_size, |e| e.size);
            self.bloom.reserve(mips);
            for i in 0..mips {
                let shift = (i + 1) as u32;
                let mw = (base[0] >> shift).max(1);
                let mh = (base[1] >> shift).max(1);
                self.bloom.push(VfxTextureTarget::new(
                    device,
                    &format!("Grand Line VFX bloom mip {i}"),
                    [mw, mh],
                    HDR_FORMAT,
                    rt_sampled,
                ));
            }
        }
    }

    /// Copy the world color texture into `scene_copy` (used for `hie_ice` refraction and `vfx_composite`).
    /// Returns `true` if the copy was encoded.
    pub fn copy_scene_color(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::Texture,
    ) -> bool {
        let Some(dst) = &self.scene_copy else {
            return false;
        };
        let copy_size = wgpu::Extent3d {
            width: dst.size[0].min(src.width()),
            height: dst.size[1].min(src.height()),
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(src.as_image_copy(), dst.texture.as_image_copy(), copy_size);
        true
    }

    /// Copy the world `Depth32Float` buffer into `depth_copy` so VFX shaders can read scene depth
    /// while rendering against the main depth attachment without read-write hazards.
    /// Returns `true` if the copy was encoded.
    pub fn copy_scene_depth(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        src_depth: &wgpu::Texture,
    ) -> bool {
        let Some(dst) = &self.depth_copy else {
            return false;
        };
        let copy_size = wgpu::Extent3d {
            width: dst.size[0].min(src_depth.width()),
            height: dst.size[1].min(src_depth.height()),
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            src_depth.as_image_copy(),
            dst.texture.as_image_copy(),
            copy_size,
        );
        true
    }

    #[inline]
    pub fn scene_size(&self) -> [u32; 2] {
        self.scene_size
    }

    #[inline]
    pub fn scene_format(&self) -> wgpu::TextureFormat {
        self.scene_format
    }

    #[inline]
    pub fn quality(&self) -> VfxQuality {
        self.quality
    }

    #[inline]
    pub fn under_load(&self) -> bool {
        self.under_load
    }

    #[inline]
    pub fn smoke_volume(&self) -> Option<&VfxTextureTarget> {
        self.smoke_volume.as_ref()
    }

    #[inline]
    pub fn emissive(&self) -> Option<&VfxTextureTarget> {
        self.emissive.as_ref()
    }

    #[inline]
    pub fn distort(&self) -> Option<&VfxTextureTarget> {
        self.distort.as_ref()
    }

    #[inline]
    pub fn bloom_mips(&self) -> &[VfxTextureTarget] {
        &self.bloom
    }

    #[inline]
    pub fn bloom_mip(&self, level: usize) -> Option<&VfxTextureTarget> {
        self.bloom.get(level)
    }

    #[inline]
    pub fn scene_copy(&self) -> Option<&VfxTextureTarget> {
        self.scene_copy.as_ref()
    }

    #[inline]
    pub fn depth_copy(&self) -> Option<&VfxTextureTarget> {
        self.depth_copy.as_ref()
    }

    /// View for `EmissiveSampler` in `vfx_composite`, falling back to the 1x1 black HDR texture.
    #[inline]
    pub fn emissive_or_fallback(&self) -> Option<&wgpu::TextureView> {
        self.emissive
            .as_ref()
            .or(self.fallback_hdr.as_ref())
            .map(|t| &t.view)
    }

    /// View for `BloomSampler` (mip 0) in `vfx_composite`, falling back to 1x1 black HDR texture.
    #[inline]
    pub fn bloom0_or_fallback(&self) -> Option<&wgpu::TextureView> {
        self.bloom
            .first()
            .or(self.fallback_hdr.as_ref())
            .map(|t| &t.view)
    }

    /// View for `DistortSampler` in `vfx_composite`, falling back to 1x1 zero `Rg16Float` texture.
    #[inline]
    pub fn distort_or_fallback(&self) -> Option<&wgpu::TextureView> {
        self.distort
            .as_ref()
            .or(self.fallback_distort.as_ref())
            .map(|t| &t.view)
    }

    /// Compute bloom mip dimensions for a given base size and mip count without allocating GPU memory.
    pub fn compute_bloom_mip_sizes(base: [u32; 2], mips: usize) -> Vec<[u32; 2]> {
        let count = mips.min(MAX_BLOOM_MIPS);
        let b = [base[0].max(1), base[1].max(1)];
        (0..count)
            .map(|i| {
                let shift = (i + 1) as u32;
                [(b[0] >> shift).max(1), (b[1] >> shift).max(1)]
            })
            .collect()
    }

    /// Total estimated VRAM footprint of currently allocated VFX targets in bytes.
    pub fn estimated_bytes(&self) -> usize {
        let mut total = 0usize;
        for t in [
            self.smoke_volume.as_ref(),
            self.emissive.as_ref(),
            self.distort.as_ref(),
            self.scene_copy.as_ref(),
            self.depth_copy.as_ref(),
            self.fallback_hdr.as_ref(),
            self.fallback_distort.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            total += t.byte_size();
        }
        for m in &self.bloom {
            total += m.byte_size();
        }
        total
    }
}
