//! Unit tests for Grand Line 0.4.0 VFX shaders (WGSL parsing, Naga IR validation,
//! Apple Metal MSL 2.1 codegen, and Rust/WGSL uniform byte-size parity).

use super::{ALL_SHADERS, VfxQuality, VfxShaderId};

#[test]
fn all_15_vfx_shaders_parse_validate_and_compile_to_metal_msl() {
    assert_eq!(ALL_SHADERS.len(), 15);
    for shader in ALL_SHADERS {
        let src = shader.wgsl_source();
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("WGSL parse failed for {}: {e}", shader.name()));

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("Naga validation failed for {}: {e}", shader.name()));

        // Verify entry points `vs_main` and `fs_main` exist
        let has_vs = module.entry_points.iter().any(|ep| ep.name == "vs_main");
        let has_fs = module.entry_points.iter().any(|ep| ep.name == "fs_main");
        assert!(has_vs, "{} missing vs_main entry point", shader.name());
        assert!(has_fs, "{} missing fs_main entry point", shader.name());

        // Check uniform struct byte size matches WGSL layout
        let struct_name = shader.uniform_struct_name();
        let wgsl_size = module
            .types
            .iter()
            .find_map(|(_, ty)| {
                if ty.name.as_deref() == Some(struct_name) {
                    if let naga::TypeInner::Struct { span, .. } = ty.inner {
                        return Some(span as usize);
                    }
                }
                None
            })
            .unwrap_or_else(|| panic!("{} missing uniform struct {}", shader.name(), struct_name));
        assert_eq!(
            shader.uniform_byte_size(),
            wgsl_size,
            "Uniform size mismatch for {} ({})",
            shader.name(),
            struct_name
        );

        // Compile to Apple Metal Shading Language (MSL 2.1 for iOS) with explicit Metal bind targets
        let mut binding_map = naga::back::msl::BindingMap::default();
        for (_, var) in module.global_variables.iter() {
            if let Some(ref br) = var.binding {
                let idx = br.binding as u8;
                binding_map.insert(
                    br.clone(),
                    naga::back::msl::BindTarget {
                        buffer: Some(idx),
                        texture: Some(idx),
                        sampler: Some(naga::back::msl::BindSamplerTarget::Resource(idx)),
                        ..Default::default()
                    },
                );
            }
        }
        let mut per_entry_point_map = naga::back::msl::EntryPointResourceMap::default();
        for ep in &module.entry_points {
            per_entry_point_map.insert(
                ep.name.clone(),
                naga::back::msl::EntryPointResources {
                    resources: binding_map.clone(),
                    ..Default::default()
                },
            );
        }
        let options = naga::back::msl::Options {
            lang_version: (2, 1),
            per_entry_point_map,
            fake_missing_bindings: false,
            zero_initialize_workgroup_memory: true,
            force_loop_bounding: true,
            ..Default::default()
        };
        let pipeline_options = naga::back::msl::PipelineOptions::default();
        let (msl, _) = naga::back::msl::write_string(&module, &info, &options, &pipeline_options)
            .unwrap_or_else(|e| panic!("Metal MSL codegen failed for {}: {e}", shader.name()));
        assert!(
            msl.contains("vs_main") && msl.contains("fs_main"),
            "Generated Metal MSL for {} missing entry points",
            shader.name()
        );
    }
}

#[test]
fn vfx_quality_tiers_scale_correctly_under_thermal_load() {
    assert!(!VfxQuality::Off.has_post());
    assert!(!VfxQuality::Low.has_bloom());
    assert!(VfxQuality::Medium.has_bloom());
    assert!(VfxQuality::High.has_distortion());
    assert_eq!(VfxQuality::High.bloom_mips(false), 5);
    assert_eq!(VfxQuality::High.bloom_mips(true), 3);
    assert_eq!(VfxQuality::High.smoke_volume_steps(false), 26.0);
    assert_eq!(VfxQuality::High.smoke_volume_steps(true), 14.0);
    assert!(VfxShaderId::VfxComposite.is_screen_space());
    assert!(!VfxShaderId::LogiaBody.is_screen_space());
}
