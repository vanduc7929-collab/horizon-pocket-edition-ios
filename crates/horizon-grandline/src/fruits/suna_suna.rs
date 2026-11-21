//! Suna Suna no Mi (Sand Logia) — Grand Line 0.4.0 1:1 port.
//!
//! Mirrors `fruits/suna_suna/{SunaSuna,SunaAbilities,SunaEffects,SunaFx,SunaGround,Desiccation,SandFunnel,SandStorm,SandTravel}.java`
//! and `fruits/suna_suna/ability/{SablesPesadoAbility,DesertSpadaAbility,SandCloudAbility,
//! BarjanAbility,GroundSeccoAbility,DesertGirasoleAbility,DesertLaSpadaAbility,
//! DesertEncierroAbility,SandTsunamiAbility,SablesAbility,SandTechnique}.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastBudget, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::logia::{LogiaManager, LogiaProfile, limbs};
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Canonical Suna-Suna constants (`SunaSuna.java`, `Desiccation.java`, `SandStorm.java`, abilities)
// ---------------------------------------------------------------------------

/// `SunaSuna.java` & `Desiccation.java` block drying settings.
pub const DESICCATION_RESTORE_SECONDS: f64 = 45.0;
pub const DESICCATION_RESTORE_TICKS: i32 = 900; // 45.0s * 20
pub const DESICCATION_BLOCKS_PER_CAST: usize = 6000;

/// `SunaSuna.java` wet counter constants (`LogiaCounter.builder("wet")`).
pub const SUNA_WET_WEAKEN_MULT: f32 = 0.6;
pub const SUNA_WET_SLOW_AMOUNT: f32 = 0.15;
pub const SUNA_WET_LINGER_TICKS: i32 = 100;

/// `SablesPesadoAbility.java` constants.
pub const SABLES_PESADO_ENERGY: f32 = 12.0;
pub const SABLES_PESADO_COOLDOWN_TICKS: i32 = 30;
pub const SABLES_PESADO_UNLOCK_MASTERY: u32 = 1;
pub const SABLES_PESADO_DAMAGE: f32 = 7.0;
pub const SABLES_PESADO_RANGE: f32 = 48.0;
pub const SABLES_PESADO_KNOCKBACK: f32 = 1.2;
pub const SABLES_PESADO_CHARGE_TICKS: i32 = 30;
pub const SABLES_PESADO_BLAST_SPEED: f64 = 2.2;
pub const SABLES_PESADO_BALL_SPEED: f64 = 1.4;
pub const SABLES_PESADO_SHOCK_RADIUS: f64 = 4.0;
pub const SABLES_PESADO_DEHYDRATE_SECONDS: i32 = 6;
pub const SABLES_PESADO_GRAVITY: f64 = 0.003;

/// `DesertSpadaAbility.java` constants.
pub const DESERT_SPADA_ENERGY: f32 = 22.0;
pub const DESERT_SPADA_COOLDOWN_TICKS: i32 = 80;
pub const DESERT_SPADA_UNLOCK_MASTERY: u32 = 2;
pub const DESERT_SPADA_DAMAGE: f32 = 11.0;
pub const DESERT_SPADA_RANGE: f32 = 42.0;
pub const DESERT_SPADA_KNOCKBACK: f32 = 0.9;
pub const DESERT_SPADA_SPEED: f64 = 2.2;
pub const DESERT_SPADA_CUT_WIDTH: f64 = 2.0;
pub const DESERT_SPADA_CUT_HEIGHT: i32 = 3;
pub const DESERT_SPADA_MAX_BLOCKS: usize = 420;

/// `SandCloudAbility.java` constants.
pub const SAND_CLOUD_ENERGY: f32 = 10.0;
pub const SAND_CLOUD_COOLDOWN_TICKS: i32 = 40;
pub const SAND_CLOUD_UNLOCK_MASTERY: u32 = 2;
pub const SAND_CLOUD_SPEED: f64 = 0.6;
pub const SAND_CLOUD_DRAIN_PER_SECOND: f32 = 4.0;
pub const SAND_CLOUD_MAX_SECONDS: f64 = 30.0;

/// `BarjanAbility.java` (`barjan` / `barchan`) constants.
pub const BARJAN_ENERGY: f32 = 20.0;
pub const BARJAN_COOLDOWN_TICKS: i32 = 80;
pub const BARJAN_UNLOCK_MASTERY: u32 = 3;
pub const BARJAN_DAMAGE: f32 = 9.0;
pub const BARJAN_RANGE: f32 = 26.0;
pub const BARJAN_KNOCKBACK: f32 = 1.2;
pub const BARJAN_SPEED: f64 = 2.2;
pub const BARJAN_SPAN: f64 = 9.0;
pub const BARJAN_DEHYDRATE_SECONDS: i32 = 6;

/// `GroundSeccoAbility.java` (`ground_secco` / `ground_death`) constants.
pub const GROUND_SECCO_ENERGY: f32 = 16.0;
pub const GROUND_SECCO_COOLDOWN_TICKS: i32 = 160;
pub const GROUND_SECCO_UNLOCK_MASTERY: u32 = 3;
pub const GROUND_SECCO_DAMAGE: f32 = 2.0;
pub const GROUND_SECCO_RANGE: f32 = 14.0;
pub const GROUND_SECCO_GROW_SECONDS: f64 = 3.0;
pub const GROUND_SECCO_GROW_TICKS: i32 = 60;
pub const GROUND_SECCO_MAX_SECONDS: f64 = 8.0;
pub const GROUND_SECCO_MAX_CHANNEL_TICKS: i32 = 160;
pub const GROUND_SECCO_DRAIN_PER_SECOND: f32 = 4.0;
pub const GROUND_SECCO_DEHYDRATE_SECONDS: i32 = 4;

/// `DesertGirasoleAbility.java` constants.
pub const DESERT_GIRASOLE_ENERGY: f32 = 25.0;
pub const DESERT_GIRASOLE_COOLDOWN_TICKS: i32 = 300;
pub const DESERT_GIRASOLE_UNLOCK_MASTERY: u32 = 4;
pub const DESERT_GIRASOLE_DAMAGE: f32 = 2.0;
pub const DESERT_GIRASOLE_RANGE: f32 = 24.0;
pub const DESERT_GIRASOLE_RADIUS: f64 = 20.0;
pub const DESERT_GIRASOLE_THROAT: f64 = 2.5;
pub const DESERT_GIRASOLE_THROAT_DEPTH: i32 = 3;
pub const DESERT_GIRASOLE_PULL: f64 = 0.055;
pub const DESERT_GIRASOLE_GROW_TICKS: i32 = 30; // 1.5s * 20
pub const DESERT_GIRASOLE_MAX_CHANNEL_TICKS: i32 = 200; // 10.0s * 20
pub const DESERT_GIRASOLE_DRAIN_PER_SECOND: f32 = 5.0;
pub const DESERT_GIRASOLE_DEHYDRATE_SECONDS: i32 = 4;

/// `DesertLaSpadaAbility.java` constants.
pub const DESERT_LA_SPADA_ENERGY: f32 = 28.0;
pub const DESERT_LA_SPADA_COOLDOWN_TICKS: i32 = 120;
pub const DESERT_LA_SPADA_UNLOCK_MASTERY: u32 = 5;
pub const DESERT_LA_SPADA_DAMAGE: f32 = 10.0;
pub const DESERT_LA_SPADA_RANGE: f32 = 18.0;
pub const DESERT_LA_SPADA_KNOCKBACK: f32 = 0.4;
pub const DESERT_LA_SPADA_SPEED: f64 = 1.6;
pub const DESERT_LA_SPADA_BASE_BLADES: usize = 3;
pub const DESERT_LA_SPADA_SPREAD_DEG: f64 = 7.0;
pub const DESERT_LA_SPADA_BLADE_DAMAGE_SHARE: f32 = 0.55;

/// `DesertEncierroAbility.java` constants.
pub const DESERT_ENCIERRO_ENERGY: f32 = 20.0;
pub const DESERT_ENCIERRO_COOLDOWN_TICKS: i32 = 200;
pub const DESERT_ENCIERRO_UNLOCK_MASTERY: u32 = 6;
pub const DESERT_ENCIERRO_DAMAGE: f32 = 3.0;
pub const DESERT_ENCIERRO_RANGE: f32 = 6.5;
pub const DESERT_ENCIERRO_MAX_CHANNEL_TICKS: i32 = 100; // 5.0s * 20
pub const DESERT_ENCIERRO_DRAIN_PER_SECOND: f32 = 6.0;
pub const DESERT_ENCIERRO_DEHYDRATE_SECONDS: i32 = 6;
pub const DESERT_ENCIERRO_HOLD_DIST: f64 = 1.7;

/// `SandTsunamiAbility.java` constants.
pub const SAND_TSUNAMI_ENERGY: f32 = 50.0;
pub const SAND_TSUNAMI_COOLDOWN_TICKS: i32 = 400;
pub const SAND_TSUNAMI_UNLOCK_MASTERY: u32 = 7;
pub const SAND_TSUNAMI_DAMAGE: f32 = 14.0;
pub const SAND_TSUNAMI_RANGE: f32 = 20.0;
pub const SAND_TSUNAMI_KNOCKBACK: f32 = 2.0;
pub const SAND_TSUNAMI_CHARGE_TICKS: i32 = 40;
pub const SAND_TSUNAMI_SPEED: f64 = 0.7;
pub const SAND_TSUNAMI_WIDTH: f64 = 13.0;
pub const SAND_TSUNAMI_HEIGHT: f64 = 5.0;
pub const SAND_TSUNAMI_DEHYDRATE_SECONDS: i32 = 6;

/// `SablesAbility.java` & `SandStorm.java` constants.
pub const SABLES_ENERGY: f32 = 70.0;
pub const SABLES_COOLDOWN_TICKS: i32 = 900;
pub const SABLES_UNLOCK_MASTERY: u32 = 8;
pub const SABLES_DAMAGE: f32 = 4.0;
pub const SABLES_RANGE: f32 = 24.0;
pub const SABLES_KNOCKBACK: f32 = 1.5;
pub const SABLES_RADIUS: f64 = 5.0;
pub const SABLES_LIFE_SECONDS: f64 = 12.0;
pub const SABLES_LIFE_TICKS: i32 = 240;
pub const SABLES_SPEED: f64 = 0.12;
pub const SABLES_PULL: f64 = 0.3;
pub const SABLES_PULL_RADIUS: f64 = 8.0;
pub const SABLES_LIFT: f64 = 0.16;
pub const SABLES_HIT_INTERVAL: i32 = 10;
pub const SABLES_BLOCKS_PER_SECOND: f64 = 4.0;
pub const SABLES_DEHYDRATE_SECONDS: i32 = 3;
pub const SABLES_CYCLONE_HEIGHT: f64 = 20.0;

/// Canonical Suna-Suna VFX resource IDs (`SunaFx.java`).
pub mod suna_fx {
    pub const AWAKEN: &str = "devilfruits:suna_suna/awaken";
    pub const LEVEL_UP: &str = "devilfruits:suna_suna/level_up";
    pub const REINCARNATE: &str = "devilfruits:suna_suna/reincarnate";
    pub const DESICCATE: &str = "devilfruits:suna_suna/desiccate";
    pub const PESADO: &str = "devilfruits:suna_suna/sables_pesado";
    pub const PESADO_IMPACT: &str = "devilfruits:suna_suna/sables_pesado_impact";
    pub const GIRASOLE: &str = "devilfruits:suna_suna/desert_girasole";
    pub const GIRASOLE_END: &str = "devilfruits:suna_suna/desert_girasole_end";
    pub const SPADA: &str = "devilfruits:suna_suna/desert_spada";
    pub const SPADA_END: &str = "devilfruits:suna_suna/desert_spada_end";
    pub const BARJAN: &str = "devilfruits:suna_suna/barjan";
    pub const BARJAN_END: &str = "devilfruits:suna_suna/barjan_end";
    pub const SECCO: &str = "devilfruits:suna_suna/ground_secco";
    pub const SECCO_END: &str = "devilfruits:suna_suna/ground_secco_end";
    pub const LA_SPADA: &str = "devilfruits:suna_suna/desert_la_spada";
    pub const LA_SPADA_IMPACT: &str = "devilfruits:suna_suna/desert_la_spada_impact";
    pub const ENCIERRO: &str = "devilfruits:suna_suna/desert_encierro";
    pub const ENCIERRO_END: &str = "devilfruits:suna_suna/desert_encierro_end";
    pub const TSUNAMI: &str = "devilfruits:suna_suna/sand_tsunami";
    pub const TSUNAMI_END: &str = "devilfruits:suna_suna/sand_tsunami_end";
    pub const SABLES: &str = "devilfruits:suna_suna/sables";
    pub const SABLES_END: &str = "devilfruits:suna_suna/sables_end";
    pub const SAND_BURST: &str = "devilfruits:suna_suna/sand_burst";
    pub const CRUMBLE: &str = "devilfruits:suna_suna/crumble";
    pub const CLOUD: &str = "devilfruits:suna_suna/sand_cloud";
    pub const CLOUD_END: &str = "devilfruits:suna_suna/sand_cloud_end";
}

/// Canonical Suna-Suna animation clip IDs.
pub mod suna_anim {
    pub const SABLES_PESADO: &str = "devilfruits:suna_suna/sables_pesado";
    pub const SABLES_PESADO_RELEASE: &str = "devilfruits:suna_suna/sables_pesado_release";
    pub const SABLES_PESADO_TAP: &str = "devilfruits:suna_suna/sables_pesado_tap";
    pub const DESERT_SPADA: &str = "devilfruits:suna_suna/desert_spada";
    pub const BARJAN: &str = "devilfruits:suna_suna/barjan";
    pub const GROUND_SECCO: &str = "devilfruits:suna_suna/ground_secco";
    pub const DESERT_GIRASOLE: &str = "devilfruits:suna_suna/desert_girasole";
    pub const DESERT_LA_SPADA: &str = "devilfruits:suna_suna/desert_la_spada";
    pub const DESERT_ENCIERRO: &str = "devilfruits:suna_suna/desert_encierro";
    pub const SAND_TSUNAMI: &str = "devilfruits:suna_suna/sand_tsunami";
    pub const SAND_TSUNAMI_RELEASE: &str = "devilfruits:suna_suna/sand_tsunami_release";
    pub const SABLES: &str = "devilfruits:suna_suna/sables";
}

/// Suna Suna no Mi Devil Fruit module (`SunaSuna.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SunaSunaModule;

impl SunaSunaModule {
    pub const fn new() -> Self {
        Self
    }

    /// Exact blade count for `DesertLaSpadaAbility.blades(base, mastery)`: `base + max(0, (mastery - 5) / 2)`.
    pub const fn la_spada_blade_count(mastery: u32) -> usize {
        let bonus = if mastery > 5 {
            ((mastery - 5) / 2) as usize
        } else {
            0
        };
        DESERT_LA_SPADA_BASE_BLADES + bonus
    }

    /// Exact cyclone lift & tangential swirl velocity inside `SandStorm.tick` (`Sables`).
    pub fn sandstorm_cyclone_velocity(
        storm_center: DVec3,
        target_pos: DVec3,
        current_vel: DVec3,
        radius: f64,
        pull_radius: f64,
        pull_strength: f64,
        lift_strength: f64,
    ) -> DVec3 {
        let rx = target_pos.x - storm_center.x;
        let rz = target_pos.z - storm_center.z;
        let d = (rx * rx + rz * rz).sqrt();
        let inward = if d < 0.4 {
            DVec3::ZERO
        } else {
            DVec3::new(-rx / d, 0.0, -rz / d)
        };
        let swirl = DVec3::new(-inward.z, 0.0, inward.x);
        if d > radius {
            let drag = 0.6 + 0.4 * (pull_radius - d) / (pull_radius - radius).max(0.5);
            current_vel * 0.7 + inward * (pull_strength * drag) + swirl * (0.2 * drag)
        } else {
            let strength = 1.0 - 0.5 * d / radius.max(1.0);
            let up = if target_pos.y < storm_center.y + 9.0 {
                lift_strength * strength
            } else {
                -0.03
            };
            current_vel * 0.55
                + inward * (pull_strength * strength)
                + swirl * (0.55 * strength)
                + DVec3::new(0.0, up, 0.0)
        }
    }

    /// Exact quicksand funnel pull & swirl vector (`SandFunnel.pull`).
    pub fn funnel_pull_velocity(
        centre: DVec3,
        at: DVec3,
        radius: f64,
        throat: f64,
        strength: f64,
    ) -> DVec3 {
        let dx = centre.x - at.x;
        let dz = centre.z - at.z;
        let d = (dx * dx + dz * dz).sqrt();
        if d > radius + 0.5 || d < 0.001 || radius <= 0.0 {
            return DVec3::ZERO;
        }
        let mut s = strength * (0.4 + 0.6 * (1.0 - d / radius).clamp(0.0, 1.0));
        if d < throat {
            s *= 0.35 + 0.65 * (d / throat);
        }
        let ix = dx / d;
        let iz = dz / d;
        let swirl = 0.7 * ((d - throat) / throat.max(0.5)).clamp(0.15, 1.0);
        DVec3::new(ix * s - iz * s * swirl, 0.0, iz * s + ix * s * swirl)
    }

    /// Performs circular terrain desiccation (`Desiccation.dry`), transforming soil into `minecraft:sand`
    /// and scheduling restoration after `DESICCATION_RESTORE_TICKS` (`900` ticks).
    fn desiccate_ring(
        ctx: &mut FruitContext<'_>,
        centre: DVec3,
        outer_radius: f64,
        inner_radius: f64,
        max_blocks: usize,
        out_transformed: &mut Vec<([i32; 3], String)>,
    ) -> usize {
        let now = ctx.ability_ctx.tick;
        let mut budget = CastBudget::new(max_blocks, 0);
        let r = (outer_radius.ceil() as i32).min(16);
        let cx = centre.x.floor() as i32;
        let cy = (centre.y - 1.0).floor() as i32;
        let cz = centre.z.floor() as i32;
        let outer2 = outer_radius * outer_radius;
        let inner2 = inner_radius * inner_radius;
        let mut count = 0usize;

        for ring in 0..=r {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if budget.blocks_remaining == 0 {
                        return count;
                    }
                    if dx.abs().max(dz.abs()) == ring {
                        let d2 = (dx * dx + dz * dz) as f64;
                        if d2 <= outer2 && (inner_radius <= 0.0 || d2 > inner2) {
                            let pos = [cx + dx, cy, cz + dz];
                            let jitter = (d2.sqrt() as i32) * 6;
                            if ctx.world_fx.transform_block(
                                pos,
                                "minecraft:grass_block",
                                "minecraft:sand",
                                0.6,
                                false,
                                &mut budget,
                                DESICCATION_RESTORE_TICKS + jitter,
                                None,
                                now,
                                ctx.config,
                            ) {
                                out_transformed.push((pos, "minecraft:sand".into()));
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    /// Computes active counter weaken multiplier (`0.6` when `"wet"` counter is active).
    fn counter_damage_scale(ctx: &FruitContext<'_>) -> f32 {
        match (LogiaProfile::for_fruit(FruitId::SunaSuna), ctx.logia.as_deref()) {
            (Some(profile), Some(logia)) => {
                LogiaManager::active_weaken_multiplier(&profile, logia, ctx.ability_ctx.tick)
            }
            _ => 1.0,
        }
    }

    /// Dissolves right arm or both arms (`SandTechnique.sandArm`).
    fn dissolve_arms(ctx: &mut FruitContext<'_>, limb_mask: u8, ticks: i32) {
        let now = ctx.ability_ctx.tick;
        if let Some(logia) = ctx.logia.as_deref_mut() {
            for idx in 0..limbs::COUNT {
                if (limb_mask & limbs::bit(idx)) != 0 {
                    logia.dissolve_limb(idx, now, ticks);
                }
            }
        }
    }

    /// Normalizes ability alias names so both `FruitMeta` (`"barchan"`) and Java/prompt names
    /// (`"barjan"`, `"ground_death"`, `"ground_secco"`) resolve cleanly.
    fn canonical_ability_name(name: &str) -> &str {
        let stripped = name
            .split(':')
            .next_back()
            .unwrap_or(name)
            .split('/')
            .next_back()
            .unwrap_or(name);
        match stripped {
            "barjan" => "barchan",
            "ground_death" => "ground_secco",
            other => other,
        }
    }
}

impl DevilFruitModule for SunaSunaModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::SunaSuna
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let allowed = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "sables_pesado"),
                display_name: "Sables: Pesado".into(),
                cast_type: CastType::Charged,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SABLES_PESADO_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SABLES_PESADO_COOLDOWN_TICKS,
                    unlock_mastery: SABLES_PESADO_UNLOCK_MASTERY,
                    damage: SABLES_PESADO_DAMAGE,
                    range: SABLES_PESADO_RANGE,
                    knockback: SABLES_PESADO_KNOCKBACK,
                    max_charge_ticks: SABLES_PESADO_CHARGE_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "desert_spada"),
                display_name: "Desert Spada".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: DESERT_SPADA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: DESERT_SPADA_COOLDOWN_TICKS,
                    unlock_mastery: DESERT_SPADA_UNLOCK_MASTERY,
                    damage: DESERT_SPADA_DAMAGE,
                    range: DESERT_SPADA_RANGE,
                    knockback: DESERT_SPADA_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "sand_cloud"),
                display_name: "Sand Cloud".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SAND_CLOUD_ENERGY,
                    energy_per_second: SAND_CLOUD_DRAIN_PER_SECOND,
                    cooldown_ticks: SAND_CLOUD_COOLDOWN_TICKS,
                    unlock_mastery: SAND_CLOUD_UNLOCK_MASTERY,
                    damage: 0.0,
                    range: 0.0,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "barchan"),
                display_name: "Barjan".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: BARJAN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: BARJAN_COOLDOWN_TICKS,
                    unlock_mastery: BARJAN_UNLOCK_MASTERY,
                    damage: BARJAN_DAMAGE,
                    range: BARJAN_RANGE,
                    knockback: BARJAN_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "ground_secco"),
                display_name: "Ground Secco".into(),
                cast_type: CastType::Held,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GROUND_SECCO_ENERGY,
                    energy_per_second: GROUND_SECCO_DRAIN_PER_SECOND,
                    cooldown_ticks: GROUND_SECCO_COOLDOWN_TICKS,
                    unlock_mastery: GROUND_SECCO_UNLOCK_MASTERY,
                    damage: GROUND_SECCO_DAMAGE,
                    range: GROUND_SECCO_RANGE,
                    knockback: 0.0,
                    max_channel_ticks: GROUND_SECCO_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "desert_girasole"),
                display_name: "Desert Girasole".into(),
                cast_type: CastType::Held,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: DESERT_GIRASOLE_ENERGY,
                    energy_per_second: DESERT_GIRASOLE_DRAIN_PER_SECOND,
                    cooldown_ticks: DESERT_GIRASOLE_COOLDOWN_TICKS,
                    unlock_mastery: DESERT_GIRASOLE_UNLOCK_MASTERY,
                    damage: DESERT_GIRASOLE_DAMAGE,
                    range: DESERT_GIRASOLE_RANGE,
                    knockback: 0.0,
                    max_channel_ticks: DESERT_GIRASOLE_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "desert_la_spada"),
                display_name: "Desert: La Spada".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: DESERT_LA_SPADA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: DESERT_LA_SPADA_COOLDOWN_TICKS,
                    unlock_mastery: DESERT_LA_SPADA_UNLOCK_MASTERY,
                    damage: DESERT_LA_SPADA_DAMAGE,
                    range: DESERT_LA_SPADA_RANGE,
                    knockback: DESERT_LA_SPADA_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "desert_encierro"),
                display_name: "Desert Encierro".into(),
                cast_type: CastType::Held,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: DESERT_ENCIERRO_ENERGY,
                    energy_per_second: DESERT_ENCIERRO_DRAIN_PER_SECOND,
                    cooldown_ticks: DESERT_ENCIERRO_COOLDOWN_TICKS,
                    unlock_mastery: DESERT_ENCIERRO_UNLOCK_MASTERY,
                    damage: DESERT_ENCIERRO_DAMAGE,
                    range: DESERT_ENCIERRO_RANGE,
                    knockback: 0.0,
                    max_channel_ticks: DESERT_ENCIERRO_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "sand_tsunami"),
                display_name: "Sand Tsunami".into(),
                cast_type: CastType::Charged,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SAND_TSUNAMI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SAND_TSUNAMI_COOLDOWN_TICKS,
                    unlock_mastery: SAND_TSUNAMI_UNLOCK_MASTERY,
                    damage: SAND_TSUNAMI_DAMAGE,
                    range: SAND_TSUNAMI_RANGE,
                    knockback: SAND_TSUNAMI_KNOCKBACK,
                    max_charge_ticks: SAND_TSUNAMI_CHARGE_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::SunaSuna, "sables"),
                display_name: "Sables".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SABLES_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SABLES_COOLDOWN_TICKS,
                    unlock_mastery: SABLES_UNLOCK_MASTERY,
                    damage: SABLES_DAMAGE,
                    range: SABLES_RANGE,
                    knockback: SABLES_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
        ]
    }

    fn ability(&self, name: &str) -> Option<AbilityDef> {
        let canonical = Self::canonical_ability_name(name);
        self.abilities()
            .into_iter()
            .find(|a| a.id.name == canonical || a.id.full_key() == name)
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::SunaSand),
            ..AbilityExecutionResult::default()
        };

        let raw_name = match event {
            AbilityEvent::CastInstant { ability, .. }
            | AbilityEvent::ChargeStarted { ability, .. }
            | AbilityEvent::ChargeReleased { ability, .. }
            | AbilityEvent::ChargeCancelled { ability }
            | AbilityEvent::ChannelStarted { ability }
            | AbilityEvent::ChannelTick { ability, .. }
            | AbilityEvent::ChannelEnded { ability, .. }
            | AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::ToggleTick { ability }
            | AbilityEvent::ComboStep { ability, .. }
            | AbilityEvent::FormSwitchRequested { ability } => ability.as_str(),
        };
        let ability_name = Self::canonical_ability_name(raw_name);
        let Some(def) = self.ability(ability_name) else {
            return res;
        };

        let now = ctx.ability_ctx.tick;
        let eye = ctx.ability_ctx.eye_pos;
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let horiz_look = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let weaken = Self::counter_damage_scale(ctx);
        let base_scaled_dmg = def.stats.scaled_damage(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) * weaken;
        let scaled_range = def.stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) as f64;

        match ability_name {
            "sables_pesado" => {
                if let AbilityEvent::ChargeStarted { .. } = event {
                    res.cast_result = Some(CastResult::Charging);
                    res.animation_clip = Some(suna_anim::SABLES_PESADO.into());
                    Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 15);
                    return res;
                }
                let charge = match event {
                    AbilityEvent::ChargeReleased { charge, .. } => *charge,
                    _ => ctx.ability_ctx.charge,
                }
                .clamp(0.0, 1.0);
                let heavy = charge >= 0.3;
                res.animation_clip = Some(
                    if heavy {
                        suna_anim::SABLES_PESADO_RELEASE
                    } else {
                        suna_anim::SABLES_PESADO_TAP
                    }
                    .into(),
                );
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, if heavy { 12 } else { 8 });

                let speed = if heavy {
                    SABLES_PESADO_BALL_SPEED
                } else {
                    SABLES_PESADO_BLAST_SPEED
                };
                let size = if heavy { 0.55 + 0.65 * charge } else { 0.3 };
                let lifetime = ((scaled_range / speed).ceil() as i32).max(1);
                let start = eye + look * 0.55 + DVec3::new(0.0, -0.3, 0.0);
                let end = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(start + look * scaled_range);

                ctx.world_fx.emit_fx(
                    suna_fx::PESADO,
                    0,
                    start,
                    look * speed,
                    size,
                    now,
                    lifetime * if heavy { 1 } else { -1 },
                    128.0,
                );

                if !heavy {
                    let hits = CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        start,
                        end,
                        (0.45 * size + 0.2) as f64,
                    );
                    if let Some(t) = hits.first() {
                        let push = look * (def.stats.knockback as f64 * 0.5)
                            + DVec3::new(0.0, 0.15, 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg * 0.55,
                            damage_type: GrandLineDamageType::SunaSand,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                    ctx.world_fx.emit_fx(
                        suna_fx::PESADO_IMPACT,
                        0,
                        end,
                        look * speed,
                        0.8,
                        now,
                        0,
                        96.0,
                    );
                } else {
                    let radius = SABLES_PESADO_SHOCK_RADIUS * (0.45 + 0.55 * (charge as f64));
                    let hits = CombatTargeting::in_radius(
                        ctx.nearby_targets,
                        ctx.player_id,
                        end,
                        radius,
                    );
                    for t in hits {
                        let falloff = (1.0 - t.center.distance(end) / (radius * 1.4))
                            .clamp(0.35, 1.0) as f32;
                        let out = DVec3::new(t.center.x - end.x, 0.0, t.center.z - end.z)
                            .normalize_or_zero();
                        let push = out
                            * (def.stats.knockback as f64
                                * (1.0 + charge as f64)
                                * (falloff as f64))
                            + DVec3::new(0.0, 0.35 * (falloff as f64), 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg * (0.6 + 0.6 * charge) * falloff,
                            damage_type: GrandLineDamageType::SunaSand,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                    ctx.world_fx
                        .emit_shake(end, 0.2 + 0.3 * charge, 24);
                    res.screen_shake = Some((0.2 + 0.3 * charge, 24));
                    ctx.world_fx.emit_fx(
                        suna_fx::PESADO_IMPACT,
                        0,
                        end,
                        look * speed,
                        radius as f32,
                        now,
                        1,
                        96.0,
                    );
                }
            }
            "desert_spada" => {
                // `DesertSpadaAbility.java`: ground-cleaving sand blade (`range = 42.0`, `cutWidth = 2.0`, `maxBlocks = 420`).
                res.animation_clip = Some(suna_anim::DESERT_SPADA.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 14);

                let dir = if horiz_look.length_squared() > 1e-6 {
                    horiz_look
                } else {
                    DVec3::Z
                };
                let side = DVec3::new(-dir.z, 0.0, dir.x);
                let start = eye + dir * 1.2 + DVec3::new(0.0, -1.6, 0.0);
                let end = start + dir * scaled_range;
                let lifetime = ((scaled_range / DESERT_SPADA_SPEED).ceil() as i32).max(1);

                ctx.world_fx.emit_fx(
                    suna_fx::SPADA,
                    0,
                    start,
                    dir * DESERT_SPADA_SPEED,
                    DESERT_SPADA_CUT_WIDTH as f32,
                    now,
                    lifetime,
                    128.0,
                );

                // Tear out trench blocks along the blade path (`DesertSpadaAbility.cut`).
                let mut spada_budget = CastBudget::new(DESERT_SPADA_MAX_BLOCKS, 40);
                let steps = (scaled_range / 2.0).ceil() as i32;
                let base_y = start.y.floor() as i32;
                for step in 0..=steps {
                    let p = start + dir * (step as f64 * 2.0);
                    let pos = [p.x.floor() as i32, base_y, p.z.floor() as i32];
                    let fling = side * 0.15 + dir * 0.1 + DVec3::new(0.0, 0.38, 0.0);
                    if ctx.world_fx.tear_out(
                        pos,
                        "minecraft:sandstone",
                        0.8,
                        false,
                        Some(fling),
                        &mut spada_budget,
                        (step % 10) as i64,
                        true,
                        true,
                        now,
                        ctx.config,
                        ctx.game_rules,
                    ) {
                        res.destroyed_blocks.push(pos);
                    }
                }

                // Capsule hits along the ground blade.
                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start + DVec3::new(0.0, 1.2, 0.0),
                    end + DVec3::new(0.0, 1.2, 0.0),
                    DESERT_SPADA_CUT_WIDTH,
                );
                for t in hits {
                    let s = if (t.center - start).dot(side) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let push = side * (s * def.stats.knockback as f64)
                        + dir * 0.2
                        + DVec3::new(0.0, 0.45, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::SunaSand,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    suna_fx::SPADA_END,
                    0,
                    end,
                    DVec3::ZERO,
                    1.0,
                    now,
                    0,
                    128.0,
                );
            }
            "sand_cloud" => {
                if let AbilityEvent::Toggled { enabled, .. } = event {
                    res.cast_result = Some(CastResult::Toggled);
                    ctx.form_runtime.set_flag(form_flags::FLYING, *enabled);
                    if *enabled {
                        ctx.world_fx.emit_fx(
                            suna_fx::CLOUD,
                            0,
                            eye,
                            DVec3::ZERO,
                            SAND_CLOUD_SPEED as f32,
                            now,
                            0,
                            64.0,
                        );
                    } else {
                        ctx.world_fx.emit_fx(
                            suna_fx::CLOUD_END,
                            0,
                            eye,
                            DVec3::ZERO,
                            1.0,
                            now,
                            0,
                            64.0,
                        );
                    }
                }
            }
            "barchan" => {
                // `BarjanAbility.java`: wide dehydrating crescent slash (`span = 9.0`, `speed = 2.2`, `range = 26.0`) + ground desiccation.
                res.animation_clip = Some(suna_anim::BARJAN.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 12);

                let start = eye + DVec3::new(0.0, -0.8, 0.0) + look * 1.4;
                let vel = look * BARJAN_SPEED;
                let lifetime = ((scaled_range / BARJAN_SPEED).ceil() as i32).max(1);
                let end = start + look * scaled_range;
                let half_span = BARJAN_SPAN * 0.5;
                let flat = if horiz_look.length_squared() > 1e-6 {
                    horiz_look
                } else {
                    DVec3::Z
                };

                ctx.world_fx.emit_fx(
                    suna_fx::BARJAN,
                    0,
                    start,
                    vel,
                    BARJAN_SPAN as f32,
                    now,
                    lifetime,
                    96.0,
                );

                // Desiccate ground under the crescent (`Desiccation.dry`).
                let dried_count = Self::desiccate_ring(
                    ctx,
                    start,
                    half_span.min(2.5),
                    0.0,
                    64,
                    &mut res.transformed_blocks,
                );
                if dried_count > 0 {
                    ctx.world_fx.emit_fx(
                        suna_fx::DESICCATE,
                        0,
                        start,
                        DVec3::ZERO,
                        half_span.min(2.5) as f32,
                        now,
                        dried_count as i32,
                        64.0,
                    );
                }

                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    half_span,
                );
                let push = flat * (def.stats.knockback as f64) + DVec3::new(0.0, 0.3, 0.0);
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::SunaSand,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    suna_fx::BARJAN_END,
                    0,
                    end,
                    vel,
                    BARJAN_SPAN as f32,
                    now,
                    0,
                    96.0,
                );
            }
            "ground_secco" => {
                // `GroundSeccoAbility.java` (`ground_secco` / `ground_death`): desiccates expanding ring up to `range = 14.0`
                // and deals armor-bypassing `SunaDehydration` damage to grounded enemies.
                res.damage_type = Some(GrandLineDamageType::SunaDehydration);
                res.animation_clip = Some(suna_anim::GROUND_SECCO.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 20);

                let centre = DVec3::new(eye.x, eye.y - 1.6, eye.z);
                match event {
                    AbilityEvent::ChannelEnded { .. } => {
                        ctx.world_fx.emit_fx(
                            suna_fx::SECCO_END,
                            0,
                            centre,
                            DVec3::ZERO,
                            scaled_range as f32,
                            now,
                            0,
                            64.0 + scaled_range,
                        );
                    }
                    _ => {
                        res.cast_result = Some(CastResult::Channelling);
                        ctx.world_fx.emit_fx(
                            suna_fx::SECCO,
                            0,
                            centre,
                            DVec3::new(GROUND_SECCO_GROW_TICKS as f64, 0.0, 0.0),
                            scaled_range as f32,
                            now,
                            GROUND_SECCO_MAX_CHANNEL_TICKS,
                            64.0 + scaled_range,
                        );

                        // Desiccate blocks into sand (`Desiccation.dry`).
                        let changed = Self::desiccate_ring(
                            ctx,
                            centre,
                            scaled_range,
                            0.0,
                            DESICCATION_BLOCKS_PER_CAST,
                            &mut res.transformed_blocks,
                        );
                        ctx.world_fx.emit_fx(
                            suna_fx::DESICCATE,
                            0,
                            centre,
                            DVec3::ZERO,
                            scaled_range as f32,
                            now,
                            changed as i32,
                            64.0,
                        );

                        let hits = CombatTargeting::in_radius(
                            ctx.nearby_targets,
                            ctx.player_id,
                            eye,
                            scaled_range,
                        );
                        for t in hits {
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: base_scaled_dmg,
                                damage_type: GrandLineDamageType::SunaDehydration,
                                knockback_impulse: DVec3::ZERO,
                                stun_ticks: 0,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }
                }
            }
            "desert_girasole" => {
                // `DesertGirasoleAbility.java` & `SandFunnel.java`: quicksand vortex (`radius = 20.0`, `throat = 2.5`).
                res.damage_type = Some(GrandLineDamageType::SunaQuicksand);
                res.animation_clip = Some(suna_anim::DESERT_GIRASOLE.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 20);

                let centre = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(eye + horiz_look * (scaled_range * 0.5) + DVec3::new(0.0, -1.6, 0.0));

                if let AbilityEvent::ChannelEnded { .. } = event {
                    ctx.world_fx.emit_fx(
                        suna_fx::GIRASOLE_END,
                        0,
                        centre,
                        DVec3::ZERO,
                        DESERT_GIRASOLE_RADIUS as f32,
                        now,
                        0,
                        128.0 + DESERT_GIRASOLE_RADIUS,
                    );
                } else {
                    res.cast_result = Some(CastResult::Channelling);
                    let params = DVec3::new(
                        DESERT_GIRASOLE_PULL,
                        DESERT_GIRASOLE_THROAT,
                        (DESERT_GIRASOLE_THROAT_DEPTH + DESERT_GIRASOLE_GROW_TICKS * 10) as f64,
                    );
                    ctx.world_fx.emit_fx(
                        suna_fx::GIRASOLE,
                        0,
                        centre,
                        params,
                        DESERT_GIRASOLE_RADIUS as f32,
                        now,
                        DESERT_GIRASOLE_MAX_CHANNEL_TICKS,
                        128.0 + DESERT_GIRASOLE_RADIUS,
                    );
                    ctx.world_fx
                        .emit_shake(centre, 0.25, (DESERT_GIRASOLE_RADIUS as i32) + 12);
                    res.screen_shake = Some((0.25, (DESERT_GIRASOLE_RADIUS as i32) + 12));

                    // Transform throat blocks into quicksand and outer ring into sand.
                    let mut qs_budget = CastBudget::new(64, 0);
                    let qpos = [
                        centre.x.floor() as i32,
                        centre.y.floor() as i32,
                        centre.z.floor() as i32,
                    ];
                    if ctx.world_fx.transform_block(
                        qpos,
                        "minecraft:sand",
                        "devilfruits:quicksand",
                        0.5,
                        false,
                        &mut qs_budget,
                        DESERT_GIRASOLE_MAX_CHANNEL_TICKS + 200,
                        Some("minecraft:sand"),
                        now,
                        ctx.config,
                    ) {
                        res.transformed_blocks
                            .push((qpos, "devilfruits:quicksand".into()));
                    }
                    Self::desiccate_ring(
                        ctx,
                        centre,
                        DESERT_GIRASOLE_RADIUS,
                        DESERT_GIRASOLE_THROAT,
                        256,
                        &mut res.transformed_blocks,
                    );

                    // Pull enemies inward with tangential swirl (`SandFunnel.pull`).
                    let hits = CombatTargeting::in_radius(
                        ctx.nearby_targets,
                        ctx.player_id,
                        centre,
                        DESERT_GIRASOLE_RADIUS,
                    );
                    for t in hits {
                        let pull_vel = Self::funnel_pull_velocity(
                            centre,
                            t.center,
                            DESERT_GIRASOLE_RADIUS,
                            DESERT_GIRASOLE_THROAT,
                            DESERT_GIRASOLE_PULL,
                        );
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::SunaQuicksand,
                            knockback_impulse: pull_vel,
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                }
            }
            "desert_la_spada" => {
                res.animation_clip = Some(suna_anim::DESERT_LA_SPADA.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 20);
                let blades = Self::la_spada_blade_count(ctx.player.mastery);
                res.spawned_projectiles = blades;

                let blade_dmg = base_scaled_dmg * DESERT_LA_SPADA_BLADE_DAMAGE_SHARE;
                let lifetime = ((scaled_range / DESERT_LA_SPADA_SPEED).ceil() as i32).max(1);
                let end = eye + look * scaled_range;

                for i in 0..blades {
                    ctx.world_fx.emit_fx(
                        suna_fx::LA_SPADA,
                        0,
                        eye,
                        look * DESERT_LA_SPADA_SPEED,
                        1.0,
                        now + i as i64,
                        lifetime,
                        96.0,
                    );
                }

                let hits = CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    (DESERT_LA_SPADA_SPREAD_DEG * (blades as f64)).to_radians(),
                );
                let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.12, 0.0);
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: blade_dmg * (blades as f32),
                        damage_type: GrandLineDamageType::SunaSand,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    suna_fx::LA_SPADA_IMPACT,
                    0,
                    end,
                    look * DESERT_LA_SPADA_SPEED,
                    1.0,
                    now,
                    if res.hits.is_empty() { 0 } else { 1 },
                    96.0,
                );
            }
            "desert_encierro" => {
                // `DesertEncierroAbility.java`: single-target moisture-draining grip (`SunaDehydration`).
                res.damage_type = Some(GrandLineDamageType::SunaDehydration);
                res.animation_clip = Some(suna_anim::DESERT_ENCIERRO.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 15);

                if let AbilityEvent::ChannelEnded { .. } = event {
                    ctx.world_fx.emit_fx(
                        suna_fx::ENCIERRO_END,
                        0,
                        eye + horiz_look * DESERT_ENCIERRO_HOLD_DIST,
                        DVec3::ZERO,
                        1.0,
                        now,
                        0,
                        96.0,
                    );
                } else {
                    res.cast_result = Some(CastResult::Channelling);
                    let victim = CombatTargeting::aimed(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        look,
                        scaled_range,
                        30.0_f64.to_radians(),
                    );
                    if let Some(t) = victim {
                        let hold_spot =
                            eye + horiz_look * (DESERT_ENCIERRO_HOLD_DIST + t.width * 0.5);
                        let pull = (hold_spot - t.center) * 0.45;
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::SunaDehydration,
                            knockback_impulse: pull,
                            stun_ticks: 20,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                        ctx.world_fx.emit_fx(
                            suna_fx::ENCIERRO,
                            0,
                            t.center,
                            DVec3::ZERO,
                            DESERT_ENCIERRO_MAX_CHANNEL_TICKS as f32,
                            now,
                            0,
                            96.0,
                        );
                    }
                }
            }
            "sand_tsunami" => {
                if let AbilityEvent::ChargeStarted { .. } = event {
                    res.cast_result = Some(CastResult::Charging);
                    res.animation_clip = Some(suna_anim::SAND_TSUNAMI.into());
                    Self::dissolve_arms(ctx, limbs::ARMS, 15);
                    return res;
                }
                let charge = match event {
                    AbilityEvent::ChargeReleased { charge, .. } => *charge,
                    _ => ctx.ability_ctx.charge,
                }
                .clamp(0.2, 1.0);

                res.animation_clip = Some(suna_anim::SAND_TSUNAMI_RELEASE.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::ARMS, 14);

                let width = SAND_TSUNAMI_WIDTH * (0.6 + 0.4 * (charge as f64));
                let height = SAND_TSUNAMI_HEIGHT * (0.5 + 0.5 * (charge as f64));
                let wave_dmg = base_scaled_dmg * (0.5 + 0.5 * charge);
                let dir = if horiz_look.length_squared() > 1e-6 {
                    horiz_look
                } else {
                    DVec3::Z
                };
                let start = eye + dir * 2.0 + DVec3::new(0.0, -1.6, 0.0);
                let end = start + dir * scaled_range;
                let lifetime = ((scaled_range / SAND_TSUNAMI_SPEED).ceil() as i32).max(1);
                let packed =
                    lifetime * 16 + ((height as f32 * 2.0).round() as i32).clamp(1, 15);

                ctx.world_fx.emit_fx(
                    suna_fx::TSUNAMI,
                    0,
                    start,
                    dir * SAND_TSUNAMI_SPEED,
                    width as f32,
                    now,
                    packed,
                    128.0,
                );
                ctx.world_fx.emit_shake(start, 0.3 + 0.3 * charge, 40);
                res.screen_shake = Some((0.3 + 0.3 * charge, 40));

                Self::desiccate_ring(
                    ctx,
                    start,
                    width * 0.5,
                    0.0,
                    128,
                    &mut res.transformed_blocks,
                );

                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    width * 0.5,
                );
                let push = dir * (def.stats.knockback as f64 * (0.6 + 0.4 * charge as f64))
                    + DVec3::new(0.0, 0.45 + 0.1 * height, 0.0);
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: wave_dmg,
                        damage_type: GrandLineDamageType::SunaSand,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_fx(
                    suna_fx::TSUNAMI_END,
                    0,
                    end,
                    dir * SAND_TSUNAMI_SPEED,
                    width as f32,
                    now,
                    0,
                    128.0,
                );
            }
            "sables" => {
                // `SablesAbility.java` & `SandStorm.java`: wandering sandstorm cyclone (`radius = 5.0`, `pullRadius = 8.0`,
                // `lift = 0.16`, `pull = 0.3`, `life = 240` ticks) that desiccates ground, rips debris, and lifts targets.
                res.animation_clip = Some(suna_anim::SABLES.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 30);

                let dir = if horiz_look.length_squared() > 1e-6 {
                    horiz_look
                } else {
                    DVec3::Z
                };
                let storm_pos = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(eye + dir * 6.0 + DVec3::new(0.0, -1.6, 0.0));

                ctx.world_fx.emit_fx(
                    suna_fx::SABLES,
                    0,
                    storm_pos,
                    dir * SABLES_SPEED,
                    SABLES_RADIUS as f32,
                    now,
                    SABLES_LIFE_TICKS,
                    160.0,
                );
                ctx.world_fx.emit_shake(storm_pos, 0.35, 48);
                res.screen_shake = Some((0.35, 48));

                // Desiccate ground under the cyclone (`SandStorm.tick`: `r * 0.8`).
                let changed = Self::desiccate_ring(
                    ctx,
                    storm_pos,
                    SABLES_RADIUS * 0.8,
                    0.0,
                    128,
                    &mut res.transformed_blocks,
                );
                if changed > 0 {
                    ctx.world_fx.emit_fx(
                        suna_fx::DESICCATE,
                        0,
                        storm_pos,
                        DVec3::ZERO,
                        (SABLES_RADIUS * 0.8) as f32,
                        now,
                        changed as i32,
                        64.0,
                    );
                }

                // Tear out surface debris (`SandStorm.rip`).
                let mut rip_budget = ctx.budget;
                let rip_pos = [
                    storm_pos.x.floor() as i32,
                    storm_pos.y.floor() as i32,
                    storm_pos.z.floor() as i32,
                ];
                if ctx.world_fx.tear_out(
                    rip_pos,
                    "minecraft:sand",
                    0.5,
                    false,
                    Some(DVec3::new(0.45, 0.85, 0.0)),
                    &mut rip_budget,
                    0,
                    false,
                    true,
                    now,
                    ctx.config,
                    ctx.game_rules,
                ) {
                    res.destroyed_blocks.push(rip_pos);
                }

                // Apply cyclone inward pull, tangential swirl, and upward lift (`SandStorm.tick`).
                let caught = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    storm_pos,
                    SABLES_PULL_RADIUS,
                );
                for t in caught {
                    let dy = t.center.y - storm_pos.y;
                    if (-2.0..=SABLES_CYCLONE_HEIGHT).contains(&dy) {
                        let cyclone_vel = Self::sandstorm_cyclone_velocity(
                            storm_pos,
                            t.center,
                            DVec3::ZERO,
                            SABLES_RADIUS,
                            SABLES_PULL_RADIUS,
                            SABLES_PULL,
                            SABLES_LIFT,
                        );
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::SunaSand,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                cyclone_vel,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                }
            }
            _ => {}
        }

        res.vfx_events = ctx.world_fx.drain_vfx();
        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let now = ctx.ability_ctx.tick;

        // `SunaSuna.java`: when in water / wet, trigger the `"wet"` LogiaCounter (`linger = 100` ticks,
        // `SuppressIntangibility`, `ForceReform`, `Weaken(0.6)`, `Slow(0.15)`).
        if ctx.submerged_in_water {
            if let (Some(profile), Some(logia)) = (
                LogiaProfile::for_fruit(FruitId::SunaSuna),
                ctx.logia.as_deref_mut(),
            ) {
                if let Some(wet_counter) = profile.counters.iter().find(|c| c.id == "wet") {
                    logia.trigger_counter(wet_counter, now);
                }
            }
            ctx.form_runtime.set_flag(form_flags::FLYING, false);
        }

        AbilityExecutionResult::default()
    }

    fn body_modifiers(
        &self,
        player: &PlayerFruitData,
        form_runtime: &FormRuntime,
        suppressed: bool,
        water_submersion: f64,
    ) -> EntityBodyModifiers {
        let mut mods = EntityBodyModifiers {
            devil_fruit_water_weakness: player.has_fruit(),
            water_submersion,
            ..EntityBodyModifiers::VANILLA
        };
        if suppressed {
            return mods;
        }
        if form_runtime.has_flag(form_flags::FLYING) && water_submersion <= 0.0 {
            mods.reset_above_ground_ticks = true;
            mods.passes_through_leaves = true;
        }
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ability::{AbilityContext, AbilityModifiers, TechniqueHeat};
    use crate::core::combat::TargetCandidate;
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::logia::{DamageCategory, LogiaConfig, LogiaData, LogiaState};
    use crate::core::rules::{CoreConfig, DFGameRules};
    use crate::core::world_fx::WorldFxManager;

    fn make_target(id_byte: u8, pos: DVec3) -> TargetCandidate {
        TargetCandidate {
            id: [id_byte; 16],
            center: pos,
            width: 0.6,
            height: 1.8,
            knockback_resistance: 0.0,
        }
    }

    #[test]
    fn suna_suna_abilities_and_wet_logia_counter_match_java() {
        let module = SunaSunaModule::new();
        assert_eq!(module.fruit_id(), FruitId::SunaSuna);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::SunaSand);

        let profile = module.logia_profile().expect("Suna-Suna is a Logia");
        assert_eq!(profile.element, "sand");
        assert_eq!(profile.elemental_drain_per_second, 3.0);
        assert_eq!(profile.dash_range, 16.0);
        assert_eq!(profile.dash_speed, 2.2);
        assert!(profile.environment.rain_opposes);

        let wet = profile
            .counters
            .iter()
            .find(|c| c.id == "wet")
            .expect("wet counter");
        assert!(wet.suppresses_intangibility());
        assert!(wet.forces_reform());
        assert_eq!(wet.damage_multiplier(), 0.6);
        assert_eq!(wet.slow_amount(), 0.15);
        assert_eq!(wet.linger_ticks, 100);

        let spada = module.ability("desert_spada").unwrap();
        assert_eq!(spada.stats.energy_cost, 22.0);
        assert_eq!(spada.stats.cooldown_ticks, 80);
        assert_eq!(spada.stats.unlock_mastery, 2);
        assert_eq!(spada.stats.damage, 11.0);
        assert_eq!(spada.stats.range, 42.0);

        // Both "barjan" and "barchan" resolve to Barjan.
        let barjan = module.ability("barjan").unwrap();
        assert_eq!(barjan.id.name, "barchan");
        assert_eq!(barjan.stats.energy_cost, 20.0);
        assert_eq!(barjan.stats.cooldown_ticks, 80);
        assert_eq!(barjan.stats.damage, 9.0);
        assert_eq!(barjan.stats.range, 26.0);

        // Both "ground_secco" and "ground_death" resolve to Ground Secco.
        let secco = module.ability("ground_death").unwrap();
        assert_eq!(secco.id.name, "ground_secco");
        assert_eq!(secco.cast_type, CastType::Held);
        assert_eq!(secco.stats.energy_cost, 16.0);
        assert_eq!(secco.stats.cooldown_ticks, 160);
        assert_eq!(secco.stats.range, 14.0);

        let sables = module.ability("sables").unwrap();
        assert!(sables.is_ultimate);
        assert_eq!(sables.stats.energy_cost, 70.0);
        assert_eq!(sables.stats.cooldown_ticks, 900);
        assert_eq!(sables.stats.unlock_mastery, 8);
        assert_eq!(sables.stats.damage, 4.0);
        assert_eq!(sables.stats.range, 24.0);
    }

    #[test]
    fn suna_suna_execute_abilities_desiccation_and_sandstorm_lift() {
        let module = SunaSunaModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::SunaSuna),
            mastery: 10,
            energy: 266.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = vec![make_target(2, DVec3::new(0.5, 63.0, 5.0))];

        let mut ctx = FruitContext {
            player_id: [1; 16],
            player: &mut player,
            form_runtime: &mut form_rt,
            gear_state: &mut gear,
            haki: &mut haki,
            logia: Some(&mut logia),
            heat: &mut heat,
            world_fx: &mut world_fx,
            budget: CastBudget::from_config(&cfg),
            ability_ctx: AbilityContext {
                caster: [1; 16],
                fruit: FruitId::SunaSuna,
                ability_name: "desert_spada".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 100,
                eye_pos: DVec3::new(0.0, 64.0, 0.0),
                look_dir: DVec3::Z,
                aim_point: Some(DVec3::new(0.0, 62.4, 5.0)),
                modifiers: AbilityModifiers::IDENTITY,
            },
            nearby_targets: &targets,
            config: &cfg,
            game_rules: DFGameRules::vanilla_default(),
            creative: false,
            submerged_in_water: false,
            suppressed: false,
            on_ground: true,
            in_lava: false,
        };

        // 1. Desert Spada tears out ground blocks and hits target in line.
        let r_spada = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "desert_spada".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_spada.hits.len(), 1);
        assert!(!r_spada.destroyed_blocks.is_empty());

        // 2. Barjan sweeps wide crescent and desiccates blocks into sand.
        let r_barjan = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "barjan".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_barjan.hits.len(), 1);
        assert!(!r_barjan.transformed_blocks.is_empty());

        // 3. Ground Secco / Ground Death transforms terrain to sand and deals armor-bypassing SunaDehydration damage.
        let r_secco = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ChannelStarted {
                ability: "ground_death".into(),
            },
        );
        assert_eq!(
            r_secco.damage_type,
            Some(GrandLineDamageType::SunaDehydration)
        );
        assert!(GrandLineDamageType::SunaDehydration.bypasses_armor());
        assert_eq!(r_secco.hits.len(), 1);
        assert!(
            r_secco
                .transformed_blocks
                .iter()
                .all(|(_, b)| b == "minecraft:sand")
        );

        // 4. Sables creates sandstorm cyclone that lifts target upward (`knockback_impulse.y > 0.0`),
        // desiccates ground, and rips debris.
        let r_sables = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "sables".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_sables.hits.len(), 1);
        assert!(
            r_sables.hits[0].knockback_impulse.y > 0.0,
            "Sables cyclone must lift targets upward"
        );
        assert!(!r_sables.transformed_blocks.is_empty());
        assert!(!r_sables.destroyed_blocks.is_empty());
    }

    #[test]
    fn suna_suna_moisture_vulnerability_suppresses_intangibility_and_weakens_damage() {
        let module = SunaSunaModule::new();
        let cfg = CoreConfig::default();
        let logia_cfg = LogiaConfig::default();
        let profile = module.logia_profile().unwrap();

        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::SunaSuna),
            mastery: 10,
            energy: 266.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData {
            state: LogiaState::ElementalBody,
            ..LogiaData::default()
        };
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = vec![make_target(2, DVec3::new(0.0, 64.0, 4.0))];

        let mut ctx = FruitContext {
            player_id: [1; 16],
            player: &mut player,
            form_runtime: &mut form_rt,
            gear_state: &mut gear,
            haki: &mut haki,
            logia: Some(&mut logia),
            heat: &mut heat,
            world_fx: &mut world_fx,
            budget: CastBudget::from_config(&cfg),
            ability_ctx: AbilityContext {
                caster: [1; 16],
                fruit: FruitId::SunaSuna,
                ability_name: "barjan".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 200,
                eye_pos: DVec3::new(0.0, 64.0, 0.0),
                look_dir: DVec3::Z,
                aim_point: None,
                modifiers: AbilityModifiers::IDENTITY,
            },
            nearby_targets: &targets,
            config: &cfg,
            game_rules: DFGameRules::vanilla_default(),
            creative: false,
            submerged_in_water: true,
            suppressed: false,
            on_ground: true,
            in_lava: false,
        };

        // Measure dry Barjan damage first.
        let dry_hit = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "barjan".into(),
                mastery: 10,
            },
        );
        let dry_dmg = dry_hit.hits[0].damage;

        // Tick while submerged_in_water -> triggers "wet" counter for 100 ticks and forces reform from ElementalBody.
        module.on_tick(&mut ctx);
        let logia_ref = ctx.logia.as_deref_mut().unwrap();
        assert!(logia_ref.is_counter_active("wet", 200));
        assert_eq!(logia_ref.state, LogiaState::Reforming);

        // Incoming melee damage now bypasses Logia intangibility because "wet" counter is active.
        let outcome = LogiaManager::evaluate_damage(
            &profile,
            ctx.player,
            logia_ref,
            DamageCategory::Melee,
            10.0,
            0,
            glam::Vec3::ZERO,
            glam::Vec3::Z,
            None,
            None,
            210,
            false,
            &cfg,
            &logia_cfg,
        );
        assert!(!outcome.passed_through);
        assert_eq!(outcome.final_damage, 10.0);

        // Outgoing ability damage is weakened by 0.6x while wet.
        let wet_hit = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "barjan".into(),
                mastery: 10,
            },
        );
        let wet_dmg = wet_hit.hits[0].damage;
        assert!((wet_dmg - dry_dmg * SUNA_WET_WEAKEN_MULT).abs() < 1e-3);
    }
}
