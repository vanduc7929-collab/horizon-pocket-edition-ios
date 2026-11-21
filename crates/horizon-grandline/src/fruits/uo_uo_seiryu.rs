//! Uo-Uo no Mi, Model: Seiryu (Fish-Fish Fruit, Model: Azure Dragon) — 1:1 port from
//! Grand Line 0.4.0 Java (`com.devilfruits.fruits.uo_uo_seiryu`).
//!
//! Includes:
//! - Three canonical forms (`"human"`, `"hybrid"`, `"dragon"`):
//!   - `"human"`: Base form (`meleeAuraDamage = 2.0`, `meleeIgniteChance = 0.25`).
//!   - `"hybrid"` (`hybrid_form` / `transform_hybrid`): `unlock_mastery = 3`, `enter_cost = 20.0`,
//!     `cooldown = 60`, `cast_ticks = 30`, `energy_regen = 0.35`, `charged_jump_max_height = 30.0`,
//!     `fire_immune = true`, `scale +1.0`, `max_health +20.0`, `attack +4.0`, `armor +6.0`,
//!     `toughness +3.0`, `knockback_resistance +0.4`, `hybridDamageReduction = 0.15`,
//!     `hybridBaseHit = 6.0`.
//!   - `"dragon"` (`dragon_form` / `transform_dragon`): `unlock_mastery = 6`, `enter_cost = 0.0`,
//!     `cooldown = 100`, `cast_ticks = 50`, `time_limit = 60.0s`, `free_abilities = true`,
//!     `flight = true`, `boostable = true`, `pass_through_leaves = true`, `fire_immune = true`,
//!     `max_health +60.0`, `attack +8.0`, `armor +10.0`, `toughness +4.0`,
//!     `knockback_resistance +1.0` (`immovable = true`, `EntityMassTier::Immovable`).
//! - Complete technique suite & canonical aliases:
//!   - `bolo_breath` (`boro_breath`) & `blast_breath`: Charged flame beams, fireball volleys (8) &
//!     meteor barrages (14) (`SeiryuFlame`, `burnSeconds = 4.0`, craters & screen shake).
//!   - `tatsumaki` (`dragon_twister`): Held vortex (`pull = 0.35`, `lift = 0.18`, `hitInterval = 10`,
//!     `maxTicks = 120`, radial launch on end).
//!   - `kaifu` (`tail_whip` / `draconic_tail_slam` / `roar`): Slashing wind / dragon tail & roar shockwave.
//!   - `kosanze_ragnaraku` (`thunder_bagua` / `draconic_cataclysm`): High-impact lightning club blitz
//!     & leap-slam (`SeiryuThunder` / `SeiryuDragon`, `clubBonus = 1.25`, Hybrid 4-target chain lightning
//!     & 30-tick stun).
//!   - `homuragumo` (`flame_cloud` / `serpentine_flight`): Flame cloud dash (`dashTicks = 6`,
//!     `lingerTicks = 40`, `burnSeconds = 4.0`) & flight toggle.
//!   - `kaen_daiko` (`flaming_dragon` / `thunder_storm` / `hellfire_ray`): Ultimate flaming dragon /
//!     16-strike thunderstorm (`SeiryuFlame` / `SeiryuThunder`, `durationTicks = 160`, `strikeRadius = 3.0`).
//!   - Hybrid moveset extras: `thunderclaw_rush`, `thunderclad_veil`, `draconic_ascension`,
//!     `draconic_thunderfall`, `dragon_dash`.

use glam::DVec3;
use horizon_world::grandline_hooks::{
    ENTITY_MASS_HEAVY_TARGET_KG, EntityBodyModifiers, EntityMassTier,
};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, FruitFormDef, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Grand Line 0.4.0 Java constants
// ---------------------------------------------------------------------------

/// `DragonShells` & `ThundercladVeilAbility` synced form flags (`FormFlags.fruit(0..=2)`).
pub const SEIRYU_FLAG_SLITHER: u32 = form_flags::fruit_bit(0);
pub const SEIRYU_FLAG_TWISTING: u32 = form_flags::fruit_bit(1);
pub const SEIRYU_FLAG_VEILED: u32 = form_flags::fruit_bit(2);

/// `UoUoSeiryu` fruit-wide settings (`UoUoSeiryu.java`).
pub const SEIRYU_MELEE_AURA_DAMAGE: f32 = 2.0;
pub const SEIRYU_MELEE_IGNITE_CHANCE: f32 = 0.25;
pub const SEIRYU_HYBRID_BASE_HIT: f32 = 6.0;
pub const SEIRYU_HYBRID_DAMAGE_REDUCTION: f32 = 0.15;
pub const SEIRYU_DRAGON_BODY_LENGTH: f64 = 140.0;
pub const SEIRYU_DRAGON_HITBOX_PARTS: usize = 42;
pub const SEIRYU_DRAGON_FLIGHT_SPEED: f64 = 2.7;
pub const SEIRYU_DRAGON_BOOST_SPEED: f64 = 5.1;
pub const SEIRYU_DRAGON_GROUND_SPEED: f64 = 0.42;

/// `BlastBreathAbility` & `BoloBreathAbility` (`bolo_breath` / `boro_breath`).
pub const BLAST_BREATH_ENERGY: f32 = 18.0;
pub const BLAST_BREATH_COOLDOWN_TICKS: i32 = 140;
pub const BLAST_BREATH_UNLOCK_MASTERY: u32 = 1;
pub const BLAST_BREATH_DAMAGE: f32 = 9.0;
pub const BLAST_BREATH_RANGE: f32 = 9.0;
pub const BLAST_BREATH_KNOCKBACK: f32 = 0.8;
pub const BLAST_BREATH_CHARGE_TICKS: i32 = 30;
pub const BLAST_BREATH_MIN_CHARGE: f32 = 0.35;
pub const BLAST_BREATH_WIDTH: f64 = 1.0;
pub const BLAST_BREATH_BURN_SECONDS: i32 = 4;
pub const BLAST_BREATH_HYBRID_BEAM_TICKS: i32 = 30;
pub const BLAST_BREATH_DRAGON_BEAM_TICKS: i32 = 120;
pub const BLAST_BREATH_DRAGON_BEAM_LENGTH: f64 = 200.0;

pub const BOLO_BREATH_ENERGY: f32 = 40.0;
pub const BOLO_BREATH_COOLDOWN_TICKS: i32 = 200;
pub const BOLO_BREATH_UNLOCK_MASTERY: u32 = 4;
pub const BOLO_BREATH_DAMAGE: f32 = 7.0;
pub const BOLO_BREATH_RANGE: f32 = 24.0;
pub const BOLO_BREATH_KNOCKBACK: f32 = 0.9;
pub const BOLO_BREATH_CHARGE_TICKS: i32 = 24;
pub const BOLO_BREATH_FIREBALLS: usize = 8;
pub const BOLO_BREATH_METEORS: usize = 14;
pub const BOLO_BREATH_IMPACT_RADIUS: f64 = 2.5;
pub const BOLO_BREATH_BURN_SECONDS: i32 = 4;

/// `DragonTwisterAbility` (`tatsumaki` / `dragon_twister`).
pub const DRAGON_TWISTER_ENERGY: f32 = 50.0;
pub const DRAGON_TWISTER_COOLDOWN_TICKS: i32 = 300;
pub const DRAGON_TWISTER_UNLOCK_MASTERY: u32 = 5;
pub const DRAGON_TWISTER_DAMAGE: f32 = 5.0;
pub const DRAGON_TWISTER_RANGE: f32 = 10.0;
pub const DRAGON_TWISTER_KNOCKBACK: f32 = 0.4;
pub const DRAGON_TWISTER_PULL: f64 = 0.35;
pub const DRAGON_TWISTER_LIFT: f64 = 0.18;
pub const DRAGON_TWISTER_HIT_INTERVAL: i32 = 10;
pub const DRAGON_TWISTER_MAX_TICKS: i32 = 120;

/// `ThunderBaguaAbility` & `DraconicCataclysmAbility` (`kosanze_ragnaraku`).
pub const THUNDER_BAGUA_ENERGY: f32 = 25.0;
pub const THUNDER_BAGUA_COOLDOWN_TICKS: i32 = 120;
pub const THUNDER_BAGUA_UNLOCK_MASTERY: u32 = 1;
pub const THUNDER_BAGUA_DAMAGE: f32 = 10.0;
pub const THUNDER_BAGUA_RANGE: f32 = 4.5;
pub const THUNDER_BAGUA_KNOCKBACK: f32 = 2.4;
pub const THUNDER_BAGUA_WINDUP_TICKS: i32 = 8;
pub const THUNDER_BAGUA_CONE_OPENING_DEG: f64 = 70.0;
pub const THUNDER_BAGUA_CLUB_BONUS: f32 = 1.25;
pub const THUNDER_BAGUA_CHAIN_TARGETS: usize = 4;
pub const THUNDER_BAGUA_CHAIN_RANGE: f64 = 7.0;
pub const THUNDER_BAGUA_CHAIN_DAMAGE_FACTOR: f32 = 0.5;

pub const DRACONIC_CATACLYSM_ENERGY: f32 = 60.0;
pub const DRACONIC_CATACLYSM_COOLDOWN_TICKS: i32 = 400;
pub const DRACONIC_CATACLYSM_UNLOCK_MASTERY: u32 = 5;
pub const DRACONIC_CATACLYSM_WEIGHT: f32 = 4.0;
pub const DRACONIC_CATACLYSM_RANGE: f32 = 8.0;
pub const DRACONIC_CATACLYSM_KNOCKBACK: f32 = 3.0;
pub const DRACONIC_CATACLYSM_LEAP_HEIGHT: f64 = 1.6;
pub const DRACONIC_CATACLYSM_STUN_TICKS: i32 = 30;
pub const DRACONIC_CATACLYSM_CRATER_BLOCKS: usize = 40;

/// `FlameCloudAbility` & `SerpentineFlightAbility` (`homuragumo`).
pub const FLAME_CLOUD_ENERGY: f32 = 20.0;
pub const FLAME_CLOUD_COOLDOWN_TICKS: i32 = 100;
pub const FLAME_CLOUD_UNLOCK_MASTERY: u32 = 1;
pub const FLAME_CLOUD_DAMAGE: f32 = 6.0;
pub const FLAME_CLOUD_RANGE: f32 = 8.0;
pub const FLAME_CLOUD_KNOCKBACK: f32 = 0.8;
pub const FLAME_CLOUD_DASH_TICKS: i32 = 6;
pub const FLAME_CLOUD_LINGER_TICKS: i32 = 40;
pub const FLAME_CLOUD_BURN_SECONDS: i32 = 4;

pub const SERPENTINE_FLIGHT_ENERGY: f32 = 5.0;
pub const SERPENTINE_FLIGHT_COOLDOWN_TICKS: i32 = 20;
pub const SERPENTINE_FLIGHT_UNLOCK_MASTERY: u32 = 6;

/// `Kaifu` / `TailWhipAbility` / `DraconicTailSlamAbility` / `RoarAbility`.
pub const KAIFU_ENERGY: f32 = 25.0;
pub const KAIFU_COOLDOWN_TICKS: i32 = 160;
pub const KAIFU_UNLOCK_MASTERY: u32 = 5;
pub const KAIFU_DAMAGE: f32 = 16.0;
pub const KAIFU_RANGE: f32 = 16.0;
pub const KAIFU_KNOCKBACK: f32 = 2.5;

/// `ThunderStormAbility` & `FlamingDragon` (`kaen_daiko` / `flaming_dragon`).
pub const KAEN_DAIKO_ENERGY: f32 = 80.0;
pub const KAEN_DAIKO_COOLDOWN_TICKS: i32 = 600;
pub const KAEN_DAIKO_UNLOCK_MASTERY: u32 = 6;
pub const KAEN_DAIKO_DAMAGE: f32 = 14.0;
pub const KAEN_DAIKO_RANGE: f32 = 18.0;
pub const KAEN_DAIKO_KNOCKBACK: f32 = 0.6;
pub const KAEN_DAIKO_STRIKES: usize = 16;
pub const KAEN_DAIKO_DURATION_TICKS: i32 = 160;
pub const KAEN_DAIKO_STRIKE_RADIUS: f64 = 3.0;
pub const KAEN_DAIKO_CLOUD_HEIGHT: f64 = 18.0;

/// Hybrid moveset extras (`HellfireRay`, `ThunderclawRush`, `ThundercladVeil`, `DraconicAscension`, `DraconicThunderfall`, `DragonDash`).
pub const DRAGON_DASH_ENERGY: f32 = 15.0;
pub const DRAGON_DASH_COOLDOWN_TICKS: i32 = 60;
pub const DRAGON_DASH_DAMAGE: f32 = 5.0;
pub const DRAGON_DASH_RANGE: f32 = 10.0;
pub const DRAGON_DASH_KNOCKBACK: f32 = 1.6;
pub const DRAGON_DASH_TICKS: i32 = 4;

pub const THUNDERCLAD_VEIL_ENERGY: f32 = 30.0;
pub const THUNDERCLAD_VEIL_COOLDOWN_TICKS: i32 = 400;
pub const THUNDERCLAD_VEIL_UNLOCK_MASTERY: u32 = 4;
pub const THUNDERCLAD_VEIL_DURATION_SECONDS: f64 = 30.0;
pub const THUNDERCLAD_VEIL_DAMAGE_BONUS: f32 = 0.10;
pub const THUNDERCLAD_VEIL_DAMAGE_REDUCTION: f32 = 0.25;

/// Canonical Seiryu VFX identifiers (`SeiryuFx.java`).
pub mod seiryu_fx {
    pub const TRANSFORM_HYBRID: &str = "devilfruits:uo_uo_seiryu/transform_hybrid";
    pub const TRANSFORM_DRAGON: &str = "devilfruits:uo_uo_seiryu/transform_dragon";
    pub const REVERT: &str = "devilfruits:uo_uo_seiryu/revert";
    pub const THUNDER_BAGUA: &str = "devilfruits:uo_uo_seiryu/thunder_bagua";
    pub const CHAIN_ARC: &str = "devilfruits:uo_uo_seiryu/chain_arc";
    pub const BREATH: &str = "devilfruits:uo_uo_seiryu/breath";
    pub const BREATH_IMPACT: &str = "devilfruits:uo_uo_seiryu/breath_impact";
    pub const FLAME_TRAIL: &str = "devilfruits:uo_uo_seiryu/flame_trail";
    pub const FLAME_CLOUD: &str = "devilfruits:uo_uo_seiryu/flame_cloud";
    pub const DASH_STREAK: &str = "devilfruits:uo_uo_seiryu/dash_streak";
    pub const FIREBALL: &str = "devilfruits:uo_uo_seiryu/fireball";
    pub const FIREBALL_IMPACT: &str = "devilfruits:uo_uo_seiryu/fireball_impact";
    pub const ROAR: &str = "devilfruits:uo_uo_seiryu/roar";
    pub const TWISTER: &str = "devilfruits:uo_uo_seiryu/twister";
    pub const STORM: &str = "devilfruits:uo_uo_seiryu/storm";
    pub const STRIKE: &str = "devilfruits:uo_uo_seiryu/strike";
    pub const TAIL: &str = "devilfruits:uo_uo_seiryu/tail";
    pub const VEIL: &str = "devilfruits:uo_uo_seiryu/veil";
    pub const RUSH: &str = "devilfruits:uo_uo_seiryu/rush";
    pub const THUNDERFALL: &str = "devilfruits:uo_uo_seiryu/thunderfall";
    pub const HELLFIRE: &str = "devilfruits:uo_uo_seiryu/hellfire";
    pub const ASCENSION: &str = "devilfruits:uo_uo_seiryu/ascension";
    pub const CATACLYSM: &str = "devilfruits:uo_uo_seiryu/cataclysm";
    pub const TAIL_SLAM: &str = "devilfruits:uo_uo_seiryu/tail_slam";
}

/// Computes incoming damage multiplier in Hybrid form (`SeiryuEvents.onIncomingDamage` + `HybridCombat.incomingFactor`):
/// `(1.0 - 0.15) * (veiled ? 0.75 : 1.0)`.
pub fn hybrid_incoming_damage_factor(in_hybrid: bool, veiled: bool) -> f32 {
    if !in_hybrid {
        return 1.0;
    }
    let base = 1.0 - SEIRYU_HYBRID_DAMAGE_REDUCTION;
    if veiled {
        base * (1.0 - THUNDERCLAD_VEIL_DAMAGE_REDUCTION)
    } else {
        base
    }
}

/// Computes outgoing damage multiplier while veiled (`HybridCombat.outgoingFactor`): `1.10` when veiled.
pub fn hybrid_outgoing_damage_factor(veiled: bool) -> f32 {
    if veiled {
        1.0 + THUNDERCLAD_VEIL_DAMAGE_BONUS
    } else {
        1.0
    }
}

/// Uo-Uo no Mi, Model: Seiryu module (`UoUoSeiryu.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct UoUoSeiryuModule;

impl UoUoSeiryuModule {
    pub const fn new() -> Self {
        Self
    }

    fn all_forms() -> Vec<String> {
        vec![
            "human".to_string(),
            "base".to_string(),
            "hybrid".to_string(),
            "dragon".to_string(),
        ]
    }

    fn make_ability(
        name: &str,
        display_name: &str,
        cast_type: CastType,
        allowed_forms: Vec<String>,
        stats: AbilityStats,
        is_ultimate: bool,
    ) -> AbilityDef {
        AbilityDef {
            id: AbilityId::new(FruitId::UoUoSeiryu, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms,
            stats,
            is_ultimate,
            stops_movement_while_charging: matches!(cast_type, CastType::Charged),
            allows_cancel_on_hurt: false,
        }
    }

    fn hybrid_form_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 20.0,
            energy_per_second: 0.0,
            cooldown_ticks: 60,
            unlock_mastery: 3,
            damage: 0.0,
            range: 1.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn dragon_form_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 0.0,
            energy_per_second: 0.0,
            cooldown_ticks: 100,
            unlock_mastery: 6,
            damage: 0.0,
            range: 1.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn bolo_breath_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: BOLO_BREATH_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: BOLO_BREATH_COOLDOWN_TICKS,
            unlock_mastery: BOLO_BREATH_UNLOCK_MASTERY,
            damage: BOLO_BREATH_DAMAGE,
            range: BOLO_BREATH_RANGE,
            knockback: BOLO_BREATH_KNOCKBACK,
            max_charge_ticks: BOLO_BREATH_CHARGE_TICKS,
            ..AbilityStats::default()
        }
    }

    fn blast_breath_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: BLAST_BREATH_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: BLAST_BREATH_COOLDOWN_TICKS,
            unlock_mastery: BLAST_BREATH_UNLOCK_MASTERY,
            damage: BLAST_BREATH_DAMAGE,
            range: BLAST_BREATH_RANGE,
            knockback: BLAST_BREATH_KNOCKBACK,
            max_charge_ticks: BLAST_BREATH_CHARGE_TICKS,
            ..AbilityStats::default()
        }
    }

    fn tatsumaki_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: DRAGON_TWISTER_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: DRAGON_TWISTER_COOLDOWN_TICKS,
            unlock_mastery: DRAGON_TWISTER_UNLOCK_MASTERY,
            damage: DRAGON_TWISTER_DAMAGE,
            range: DRAGON_TWISTER_RANGE,
            knockback: DRAGON_TWISTER_KNOCKBACK,
            max_channel_ticks: DRAGON_TWISTER_MAX_TICKS,
            min_channel_ticks: 0,
            ..AbilityStats::default()
        }
    }

    fn kaifu_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: KAIFU_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: KAIFU_COOLDOWN_TICKS,
            unlock_mastery: KAIFU_UNLOCK_MASTERY,
            damage: KAIFU_DAMAGE,
            range: KAIFU_RANGE,
            knockback: KAIFU_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn ragnaraku_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: THUNDER_BAGUA_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: THUNDER_BAGUA_COOLDOWN_TICKS,
            unlock_mastery: THUNDER_BAGUA_UNLOCK_MASTERY,
            damage: THUNDER_BAGUA_DAMAGE,
            range: THUNDER_BAGUA_RANGE,
            knockback: THUNDER_BAGUA_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn homuragumo_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: FLAME_CLOUD_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: FLAME_CLOUD_COOLDOWN_TICKS,
            unlock_mastery: FLAME_CLOUD_UNLOCK_MASTERY,
            damage: FLAME_CLOUD_DAMAGE,
            range: FLAME_CLOUD_RANGE,
            knockback: FLAME_CLOUD_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn kaen_daiko_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: KAEN_DAIKO_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: KAEN_DAIKO_COOLDOWN_TICKS,
            unlock_mastery: KAEN_DAIKO_UNLOCK_MASTERY,
            damage: KAEN_DAIKO_DAMAGE,
            range: KAEN_DAIKO_RANGE,
            knockback: KAEN_DAIKO_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn veil_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: THUNDERCLAD_VEIL_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: THUNDERCLAD_VEIL_COOLDOWN_TICKS,
            unlock_mastery: THUNDERCLAD_VEIL_UNLOCK_MASTERY,
            damage: 0.0,
            range: 0.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn canonical_name(ability: &str) -> &str {
        let stripped = ability
            .split(':')
            .next_back()
            .unwrap_or(ability)
            .split('/')
            .next_back()
            .unwrap_or(ability);
        match stripped {
            "transform_hybrid" => "hybrid_form",
            "transform_dragon" => "dragon_form",
            "boro_breath" | "blast_breath" => "bolo_breath",
            "dragon_twister" => "tatsumaki",
            "tail_whip" | "draconic_tail_slam" | "roar" => "kaifu",
            "thunder_bagua" | "draconic_cataclysm" | "thunderclaw_rush" | "draconic_ascension" => {
                "kosanze_ragnaraku"
            }
            "flame_cloud" | "serpentine_flight" | "dragon_dash" => "homuragumo",
            "flaming_dragon" | "thunder_storm" | "hellfire_ray" | "draconic_thunderfall" => {
                "kaen_daiko"
            }
            other => other,
        }
    }

    fn exec_form_switch(
        &self,
        ctx: &mut FruitContext<'_>,
        target_form: &str,
        res: &mut AbilityExecutionResult,
    ) {
        let is_same = ctx.player.active_form == target_form;
        if is_same {
            ctx.player.active_form = "human".to_string();
            ctx.form_runtime.set_flag(form_flags::FLYING, false);
            ctx.form_runtime.set_flag(form_flags::BOOSTING, false);
            ctx.form_runtime.time_limit_end_tick = None;
            ctx.form_runtime.safe_landing.arm();
            res.form_switch_to = Some("human".to_string());
            ctx.world_fx.emit_fx(
                seiryu_fx::REVERT,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        } else {
            ctx.player.active_form = target_form.to_string();
            let is_dragon = target_form == "dragon";
            ctx.form_runtime.set_flag(form_flags::FLYING, is_dragon);
            ctx.form_runtime.time_limit_end_tick = if is_dragon {
                Some(ctx.ability_ctx.tick + 1200)
            } else {
                None
            };
            res.form_switch_to = Some(target_form.to_string());
            res.animation_clip = Some(
                if is_dragon {
                    "devilfruits:uo_uo_seiryu/transform_dragon"
                } else {
                    "devilfruits:uo_uo_seiryu/transform_hybrid"
                }
                .to_string(),
            );
            ctx.world_fx.emit_fx(
                if is_dragon {
                    seiryu_fx::TRANSFORM_DRAGON
                } else {
                    seiryu_fx::TRANSFORM_HYBRID
                },
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                if is_dragon { 50 } else { 30 },
                96.0,
            );
            ctx.world_fx
                .emit_shake(ctx.ability_ctx.eye_pos, if is_dragon { 1.0 } else { 0.6 }, 32);
            res.screen_shake = Some((if is_dragon { 1.0 } else { 0.6 }, 32));
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_bolo_breath(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        res.damage_type = Some(GrandLineDamageType::SeiryuFlame);
        let stats = Self::bolo_breath_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;

        if matches!(event, AbilityEvent::ChargeStarted { .. }) {
            res.cast_result = Some(CastResult::Charging);
            res.animation_clip = Some("devilfruits:uo_uo_seiryu/blast_breath".to_string());
            return;
        }

        let charge = match event {
            AbilityEvent::ChargeReleased { charge, .. } => *charge,
            _ => ctx.ability_ctx.charge,
        }
        .max(BLAST_BREATH_MIN_CHARGE)
        .clamp(0.0, 1.0);

        let is_dragon = ctx.player.active_form == "dragon";
        let is_hybrid = ctx.player.active_form == "hybrid";
        let form_dmg_mult = if is_dragon {
            3.5
        } else if is_hybrid {
            2.0
        } else {
            1.0
        };
        let form_range_mult = if is_dragon {
            3.0
        } else if is_hybrid {
            1.6
        } else {
            1.0
        };
        let veiled = ctx.form_runtime.has_flag(SEIRYU_FLAG_VEILED);

        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * form_dmg_mult
            * hybrid_outgoing_damage_factor(veiled)
            * (0.4 + 0.6 * charge);
        let length = (stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) * form_range_mult) as f64;
        let width = BLAST_BREATH_WIDTH * (if is_dragon { 2.2 } else { 1.4 });
        let end = eye + look * length;

        res.animation_clip = Some("devilfruits:uo_uo_seiryu/blast_breath_release".to_string());
        res.spawned_projectiles = if is_dragon {
            BOLO_BREATH_METEORS
        } else if is_hybrid {
            BOLO_BREATH_FIREBALLS
        } else {
            1
        };

        let push = look * ((BOLO_BREATH_KNOCKBACK * charge) as f64) + DVec3::new(0.0, 0.25, 0.0);
        for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, eye, end, width) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::SeiryuFlame,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: BOLO_BREATH_BURN_SECONDS,
            });
        }

        let mut budget = ctx.budget;
        res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
            eye + look * (length.min(12.0) * 0.5),
            if is_dragon { 3.0 } else { 1.8 },
            &mut budget,
            false,
            ctx.ability_ctx.tick,
            ctx.config,
            ctx.game_rules,
            |_| ("minecraft:stone".to_string(), 1.5, false),
        );

        ctx.world_fx.emit_fx(
            seiryu_fx::BREATH,
            0,
            eye,
            look,
            length as f32,
            ctx.ability_ctx.tick,
            if is_dragon {
                BLAST_BREATH_DRAGON_BEAM_TICKS
            } else {
                BLAST_BREATH_HYBRID_BEAM_TICKS
            },
            128.0,
        );
        ctx.world_fx
            .emit_shake(end, if is_dragon { 0.85 } else { 0.45 }, 32);
        res.screen_shake = Some((if is_dragon { 0.85 } else { 0.45 }, 32));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_tatsumaki(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::tatsumaki_stats();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        match event {
            AbilityEvent::ChannelStarted { .. } => {
                ctx.form_runtime.set_flag(SEIRYU_FLAG_TWISTING, true);
                res.animation_clip =
                    Some("devilfruits:uo_uo_seiryu/dragon/dragon_twister".to_string());
                ctx.world_fx.emit_fx(
                    seiryu_fx::TWISTER,
                    0,
                    eye,
                    DVec3::ZERO,
                    range as f32,
                    ctx.ability_ctx.tick,
                    DRAGON_TWISTER_MAX_TICKS,
                    96.0,
                );
            }
            AbilityEvent::ChannelTick { elapsed_ticks, .. } => {
                ctx.form_runtime.set_flag(SEIRYU_FLAG_TWISTING, true);
                let pulse =
                    *elapsed_ticks > 0 && (*elapsed_ticks % DRAGON_TWISTER_HIT_INTERVAL) == 0;
                for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range) {
                    let rx = t.center.x - eye.x;
                    let rz = t.center.z - eye.z;
                    let h = (rx * rx + rz * rz).sqrt();
                    let strength = (1.0 - 0.5 * h / range.max(1.0)).max(0.2);
                    let in_vec = if h < 0.5 {
                        DVec3::ZERO
                    } else {
                        DVec3::new(-rx / h, 0.0, -rz / h)
                    };
                    let swirl = DVec3::new(-in_vec.z, 0.0, in_vec.x);
                    let vel = in_vec * (DRAGON_TWISTER_PULL * strength)
                        + swirl * (0.45 * strength)
                        + DVec3::new(0.0, DRAGON_TWISTER_LIFT * strength, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: if pulse { dmg } else { 0.0 },
                        damage_type: GrandLineDamageType::SeiryuDragon,
                        knockback_impulse: vel,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
            }
            AbilityEvent::CastInstant { .. } => {
                res.animation_clip =
                    Some("devilfruits:uo_uo_seiryu/dragon/dragon_twister".to_string());
                for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range) {
                    let out = DVec3::new(t.center.x - eye.x, 0.0, t.center.z - eye.z)
                        .normalize_or_zero();
                    let vel = out * (1.2 + DRAGON_TWISTER_KNOCKBACK as f64)
                        + DVec3::new(0.0, 0.6, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::SeiryuDragon,
                        knockback_impulse: vel,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_fx(
                    seiryu_fx::TWISTER,
                    0,
                    eye,
                    DVec3::ZERO,
                    range as f32,
                    ctx.ability_ctx.tick,
                    DRAGON_TWISTER_MAX_TICKS,
                    96.0,
                );
                ctx.world_fx.emit_shake(eye, 0.4, 32);
                res.screen_shake = Some((0.4, 32));
            }
            AbilityEvent::ChannelEnded { .. } => {
                ctx.form_runtime.set_flag(SEIRYU_FLAG_TWISTING, false);
                for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range) {
                    let out = DVec3::new(t.center.x - eye.x, 0.0, t.center.z - eye.z)
                        .normalize_or_zero();
                    let vel = out * (1.2 + DRAGON_TWISTER_KNOCKBACK as f64)
                        + DVec3::new(0.0, 0.6, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: 0.0,
                        damage_type: GrandLineDamageType::SeiryuDragon,
                        knockback_impulse: vel,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_shake(eye, 0.4, 32);
                res.screen_shake = Some((0.4, 32));
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_kaifu(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::kaifu_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        res.animation_clip = Some("devilfruits:uo_uo_seiryu/dragon/tail_whip".to_string());
        res.spawned_projectiles = 1;

        let push = look * (KAIFU_KNOCKBACK as f64) + DVec3::new(0.0, 0.55, 0.0);
        for t in CombatTargeting::in_cone(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            60.0_f64.to_radians(),
        ) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::SeiryuDragon,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            seiryu_fx::TAIL,
            0,
            eye,
            look,
            range as f32,
            ctx.ability_ctx.tick,
            26,
            96.0,
        );
        ctx.world_fx.emit_shake(eye, 0.55, 28);
        res.screen_shake = Some((0.55, 28));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_kosanze_ragnaraku(
        &self,
        ctx: &mut FruitContext<'_>,
        raw_name: &str,
        res: &mut AbilityExecutionResult,
    ) {
        let is_hybrid = ctx.player.active_form == "hybrid";
        let veiled = ctx.form_runtime.has_flag(SEIRYU_FLAG_VEILED);
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;

        let is_cataclysm = raw_name.contains("cataclysm") || raw_name.contains("ragnaraku");
        res.damage_type = Some(if is_cataclysm && is_hybrid {
            GrandLineDamageType::SeiryuDragon
        } else {
            GrandLineDamageType::SeiryuThunder
        });

        let stats = Self::ragnaraku_stats();
        let form_mult = if is_hybrid { 2.5 } else { 1.0 };
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * form_mult
            * THUNDER_BAGUA_CLUB_BONUS
            * hybrid_outgoing_damage_factor(veiled);
        let range = (stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) * (if is_hybrid { 1.8 } else { 1.0 })) as f64;

        res.animation_clip = Some(
            if is_cataclysm {
                "devilfruits:uo_uo_seiryu/hybrid/draconic_cataclysm"
            } else {
                "devilfruits:uo_uo_seiryu/thunder_bagua"
            }
            .to_string(),
        );
        res.caster_velocity_impulse =
            Some(flat * 2.4 + DVec3::new(0.0, DRACONIC_CATACLYSM_LEAP_HEIGHT * 0.5, 0.0));

        let impact = eye + flat * 2.4;
        let stun = if is_cataclysm || is_hybrid {
            DRACONIC_CATACLYSM_STUN_TICKS
        } else {
            0
        };

        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, impact, range) {
            let out_raw = DVec3::new(t.center.x - eye.x, 0.0, t.center.z - eye.z);
            let dir = if out_raw.length_squared() < 1e-4 {
                flat
            } else {
                out_raw.normalize()
            };
            let push = dir * (THUNDER_BAGUA_KNOCKBACK as f64) + DVec3::new(0.0, 0.6, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: res.damage_type.unwrap_or(GrandLineDamageType::SeiryuThunder),
                knockback_impulse: push,
                stun_ticks: stun,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        let mut budget = ctx.budget;
        res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
            impact,
            if is_hybrid { 2.5 } else { 1.6 },
            &mut budget,
            false,
            ctx.ability_ctx.tick,
            ctx.config,
            ctx.game_rules,
            |_| ("minecraft:stone".to_string(), 1.5, false),
        );

        ctx.world_fx.emit_fx(
            if is_cataclysm {
                seiryu_fx::CATACLYSM
            } else {
                seiryu_fx::THUNDER_BAGUA
            },
            0,
            impact,
            flat,
            range as f32,
            ctx.ability_ctx.tick,
            if is_hybrid { 1 } else { 0 },
            96.0,
        );
        let shake_str = if is_cataclysm {
            1.2
        } else if is_hybrid {
            0.9
        } else {
            0.45
        };
        ctx.world_fx.emit_shake(impact, shake_str, 48);
        res.screen_shake = Some((shake_str, 48));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_homuragumo(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        res.damage_type = Some(GrandLineDamageType::SeiryuFlame);
        if let AbilityEvent::Toggled { enabled, .. } = event {
            ctx.form_runtime.set_flag(SEIRYU_FLAG_SLITHER, *enabled);
            ctx.form_runtime.set_flag(form_flags::FLYING, !*enabled);
            return;
        }

        let stats = Self::homuragumo_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let dir = (flat + DVec3::new(0.0, look.y.clamp(0.0, 0.35), 0.0)).normalize_or_zero();
        let speed = range / (FLAME_CLOUD_DASH_TICKS as f64);

        res.animation_clip = Some("devilfruits:uo_uo_seiryu/flame_cloud".to_string());
        res.caster_velocity_impulse = Some(dir * speed + DVec3::new(0.0, 0.04, 0.0));
        ctx.form_runtime.set_flag(form_flags::FLYING, true);

        let end = eye + dir * range;
        let push = dir * (FLAME_CLOUD_KNOCKBACK as f64) + DVec3::new(0.0, 0.35, 0.0);
        for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, eye, end, 1.3) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::SeiryuFlame,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: FLAME_CLOUD_BURN_SECONDS,
            });
        }

        ctx.world_fx.emit_fx(
            seiryu_fx::FLAME_CLOUD,
            0,
            eye,
            dir,
            1.3,
            ctx.ability_ctx.tick,
            FLAME_CLOUD_DASH_TICKS,
            64.0,
        );
        ctx.world_fx.emit_fx(
            seiryu_fx::FLAME_TRAIL,
            0,
            eye,
            end,
            1.3,
            ctx.ability_ctx.tick,
            FLAME_CLOUD_LINGER_TICKS,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_kaen_daiko(
        &self,
        ctx: &mut FruitContext<'_>,
        raw_name: &str,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::kaen_daiko_stats();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let is_thunder = raw_name.contains("thunder");
        let dtype = if is_thunder {
            GrandLineDamageType::SeiryuThunder
        } else {
            GrandLineDamageType::SeiryuFlame
        };

        res.damage_type = Some(dtype);
        res.animation_clip = Some("devilfruits:uo_uo_seiryu/dragon/thunder_storm".to_string());

        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range) {
            let out = DVec3::new(t.center.x - eye.x, 0.0, t.center.z - eye.z).normalize_or_zero();
            let push = out * (KAEN_DAIKO_KNOCKBACK as f64) + DVec3::new(0.0, 0.35, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: dtype,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: if is_thunder { 0 } else { 6 },
            });
        }

        let mut budget = ctx.budget;
        res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
            eye,
            2.2,
            &mut budget,
            false,
            ctx.ability_ctx.tick,
            ctx.config,
            ctx.game_rules,
            |_| ("minecraft:stone".to_string(), 1.5, false),
        );

        ctx.world_fx.emit_fx(
            seiryu_fx::STORM,
            0,
            eye + DVec3::new(0.0, KAEN_DAIKO_CLOUD_HEIGHT, 0.0),
            DVec3::ZERO,
            range as f32,
            ctx.ability_ctx.tick,
            KAEN_DAIKO_DURATION_TICKS,
            192.0,
        );
        ctx.world_fx.emit_shake(eye, 0.9, 48);
        res.screen_shake = Some((0.9, 48));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_veil(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let active = match event {
            AbilityEvent::Toggled { enabled, .. } => *enabled,
            _ => !ctx.form_runtime.has_flag(SEIRYU_FLAG_VEILED),
        };
        ctx.form_runtime.set_flag(SEIRYU_FLAG_VEILED, active);
        res.animation_clip = Some("devilfruits:uo_uo_seiryu/hybrid/thunderclad_veil".to_string());
        ctx.world_fx.emit_fx(
            seiryu_fx::VEIL,
            0,
            ctx.ability_ctx.eye_pos,
            DVec3::ZERO,
            if active { 1.0 } else { 0.0 },
            ctx.ability_ctx.tick,
            if active {
                (THUNDERCLAD_VEIL_DURATION_SECONDS * 20.0) as i32
            } else {
                0
            },
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }
}

impl DevilFruitModule for UoUoSeiryuModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::UoUoSeiryu
    }

    fn forms(&self) -> Vec<FruitFormDef> {
        let mut forms = FruitFormDef::builtin_forms(FruitId::UoUoSeiryu);
        let all_names: Vec<String> = self.abilities().into_iter().map(|a| a.id.name).collect();
        for f in &mut forms {
            for ab in &all_names {
                if !f.abilities.contains(ab) {
                    f.abilities.push(ab.clone());
                }
            }
        }
        forms
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let all = Self::all_forms();
        vec![
            Self::make_ability(
                "hybrid_form",
                "Hybrid Form",
                CastType::FormSwitch,
                all.clone(),
                Self::hybrid_form_stats(),
                false,
            ),
            Self::make_ability(
                "dragon_form",
                "Azure Dragon Form",
                CastType::FormSwitch,
                all.clone(),
                Self::dragon_form_stats(),
                true,
            ),
            Self::make_ability(
                "bolo_breath",
                "Bolo Breath",
                CastType::Charged,
                all.clone(),
                Self::bolo_breath_stats(),
                false,
            ),
            Self::make_ability(
                "tatsumaki",
                "Dragon Twister (Tatsumaki)",
                CastType::Held,
                all.clone(),
                Self::tatsumaki_stats(),
                false,
            ),
            Self::make_ability(
                "kaifu",
                "Kaifu (Demolition Gust / Tail Whip)",
                CastType::Instant,
                all.clone(),
                Self::kaifu_stats(),
                false,
            ),
            Self::make_ability(
                "kosanze_ragnaraku",
                "Kosanze Ragnaraku (Thunder Bagua / Draconic Cataclysm)",
                CastType::Instant,
                all.clone(),
                Self::ragnaraku_stats(),
                false,
            ),
            Self::make_ability(
                "homuragumo",
                "Homuragumo (Flame Clouds)",
                CastType::Instant,
                all.clone(),
                Self::homuragumo_stats(),
                false,
            ),
            Self::make_ability(
                "kaen_daiko",
                "Kaen Daiko (Flaming Drum Dragon / Thunder Storm)",
                CastType::Instant,
                all.clone(),
                Self::kaen_daiko_stats(),
                true,
            ),
            Self::make_ability(
                "flaming_dragon",
                "Flaming Drum Dragon",
                CastType::Instant,
                all.clone(),
                Self::kaen_daiko_stats(),
                true,
            ),
            // Spec & Java class aliases
            Self::make_ability(
                "boro_breath",
                "Boro Breath",
                CastType::Charged,
                all.clone(),
                Self::bolo_breath_stats(),
                false,
            ),
            Self::make_ability(
                "blast_breath",
                "Blast Breath",
                CastType::Charged,
                all.clone(),
                Self::blast_breath_stats(),
                false,
            ),
            Self::make_ability(
                "dragon_twister",
                "Dragon Twister",
                CastType::Held,
                all.clone(),
                Self::tatsumaki_stats(),
                false,
            ),
            Self::make_ability(
                "thunder_bagua",
                "Thunder Bagua",
                CastType::Instant,
                all.clone(),
                Self::ragnaraku_stats(),
                false,
            ),
            Self::make_ability(
                "draconic_cataclysm",
                "Draconic Cataclysm",
                CastType::Instant,
                all.clone(),
                Self::ragnaraku_stats(),
                true,
            ),
            Self::make_ability(
                "flame_cloud",
                "Flame Cloud",
                CastType::Instant,
                all.clone(),
                Self::homuragumo_stats(),
                false,
            ),
            Self::make_ability(
                "serpentine_flight",
                "Serpentine Flight",
                CastType::Toggle,
                all.clone(),
                Self::homuragumo_stats(),
                false,
            ),
            Self::make_ability(
                "thunder_storm",
                "Thunder Storm",
                CastType::Instant,
                all.clone(),
                Self::kaen_daiko_stats(),
                true,
            ),
            Self::make_ability(
                "thunderclad_veil",
                "Thunderclad Veil",
                CastType::Toggle,
                all.clone(),
                Self::veil_stats(),
                false,
            ),
            Self::make_ability(
                "thunderclaw_rush",
                "Thunderclaw Rush",
                CastType::Instant,
                all.clone(),
                Self::ragnaraku_stats(),
                false,
            ),
            Self::make_ability(
                "hellfire_ray",
                "Hellfire Ray",
                CastType::Held,
                all.clone(),
                Self::kaen_daiko_stats(),
                false,
            ),
            Self::make_ability(
                "draconic_ascension",
                "Draconic Ascension",
                CastType::Instant,
                all.clone(),
                Self::ragnaraku_stats(),
                false,
            ),
            Self::make_ability(
                "draconic_thunderfall",
                "Draconic Thunderfall",
                CastType::Held,
                all.clone(),
                Self::kaen_daiko_stats(),
                false,
            ),
            Self::make_ability(
                "draconic_tail_slam",
                "Draconic Tail Slam",
                CastType::Instant,
                all.clone(),
                Self::kaifu_stats(),
                false,
            ),
            Self::make_ability(
                "tail_whip",
                "Tail Whip",
                CastType::Combo,
                all.clone(),
                Self::kaifu_stats(),
                false,
            ),
            Self::make_ability(
                "roar",
                "Dragon Roar",
                CastType::Instant,
                all.clone(),
                Self::kaifu_stats(),
                false,
            ),
            Self::make_ability(
                "dragon_dash",
                "Dragon Dash",
                CastType::Instant,
                all.clone(),
                Self::homuragumo_stats(),
                false,
            ),
            Self::make_ability(
                "transform_hybrid",
                "Transform: Hybrid",
                CastType::FormSwitch,
                all.clone(),
                Self::hybrid_form_stats(),
                false,
            ),
            Self::make_ability(
                "transform_dragon",
                "Transform: Dragon",
                CastType::FormSwitch,
                all,
                Self::dragon_form_stats(),
                true,
            ),
        ]
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::SeiryuDragon),
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

        match Self::canonical_name(raw_name) {
            "hybrid_form" => {
                self.exec_form_switch(ctx, "hybrid", &mut res);
            }
            "dragon_form" => {
                self.exec_form_switch(ctx, "dragon", &mut res);
            }
            "bolo_breath" => {
                self.exec_bolo_breath(ctx, event, &mut res);
            }
            "tatsumaki" => {
                if matches!(
                    event,
                    AbilityEvent::ChannelStarted { .. } | AbilityEvent::ChannelTick { .. }
                ) {
                    res.cast_result = Some(CastResult::Channelling);
                }
                self.exec_tatsumaki(ctx, event, &mut res);
            }
            "kaifu" => {
                self.exec_kaifu(ctx, &mut res);
            }
            "kosanze_ragnaraku" => {
                self.exec_kosanze_ragnaraku(ctx, raw_name, &mut res);
            }
            "homuragumo" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_homuragumo(ctx, event, &mut res);
            }
            "kaen_daiko" => {
                self.exec_kaen_daiko(ctx, raw_name, &mut res);
            }
            "thunderclad_veil" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_veil(ctx, event, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed || (ctx.submerged_in_water && ctx.config.revert_in_water) {
            ctx.form_runtime.set_flag(SEIRYU_FLAG_VEILED, false);
            ctx.form_runtime.set_flag(SEIRYU_FLAG_TWISTING, false);
            if ctx.player.active_form != "human" && ctx.player.active_form != "base" {
                ctx.player.active_form = "human".to_string();
                ctx.form_runtime.set_flag(form_flags::FLYING, false);
                res.form_switch_to = Some("human".to_string());
            }
            return res;
        }

        if ctx.player.active_form == "dragon" {
            let slithering = ctx.form_runtime.has_flag(SEIRYU_FLAG_SLITHER);
            ctx.form_runtime.set_flag(form_flags::FLYING, !slithering);
            if let Some(end_tick) = ctx.form_runtime.time_limit_end_tick {
                if ctx.ability_ctx.tick >= end_tick {
                    ctx.player.active_form = "human".to_string();
                    ctx.form_runtime.time_limit_end_tick = None;
                    ctx.form_runtime.set_flag(form_flags::FLYING, false);
                    ctx.form_runtime.safe_landing.arm();
                    res.form_switch_to = Some("human".to_string());
                }
            }
        }
        res
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

        match player.active_form.as_str() {
            "hybrid" => {
                mods.fire_immune = true;
                mods.mass_tier = EntityMassTier::Heavy;
                mods.mass_kg_override = Some(350.0);
                if form_runtime.has_flag(form_flags::FLYING) {
                    mods.reset_above_ground_ticks = true;
                }
            }
            "dragon" => {
                mods.passes_through_leaves = true;
                mods.fire_immune = true;
                mods.immovable = true;
                mods.mass_tier = EntityMassTier::Immovable;
                mods.mass_kg_override = Some(ENTITY_MASS_HEAVY_TARGET_KG);
                mods.reset_above_ground_ticks = true;
            }
            _ => {
                if form_runtime.has_flag(form_flags::FLYING) {
                    mods.reset_above_ground_ticks = true;
                }
            }
        }
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ability::{AbilityContext, AbilityModifiers, CastBudget, TechniqueHeat};
    use crate::core::combat::TargetCandidate;
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::rules::{CoreConfig, DFGameRules};
    use crate::core::world_fx::WorldFxManager;

    fn test_target(id_byte: u8, pos: DVec3) -> TargetCandidate {
        TargetCandidate {
            id: [id_byte; 16],
            center: pos,
            width: 0.6,
            height: 1.8,
            knockback_resistance: 0.0,
        }
    }

    #[test]
    fn seiryu_forms_abilities_and_aliases_match_java_stats() {
        let m = UoUoSeiryuModule::new();
        assert_eq!(m.fruit_id(), FruitId::UoUoSeiryu);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::SeiryuDragon);

        let forms = m.forms();
        assert_eq!(forms.len(), 3);
        let hybrid = forms.iter().find(|f| f.id.name == "hybrid").unwrap();
        assert_eq!(hybrid.stats.unlock_mastery, 3);
        assert_eq!(hybrid.stats.enter_cost, 20.0);
        assert!(hybrid.collision.fire_immune);

        let dragon = forms.iter().find(|f| f.id.name == "dragon").unwrap();
        assert_eq!(dragon.stats.unlock_mastery, 6);
        assert!(dragon.stats.flight);
        assert!(dragon.stats.ability_modifiers.free_abilities);
        assert!(dragon.collision.pass_through_leaves);
        assert!(dragon.collision.fire_immune);

        let boro = m.ability("boro_breath").unwrap();
        assert_eq!(boro.cast_type, CastType::Charged);
        assert_eq!(boro.stats.energy_cost, 40.0);
        assert_eq!(boro.stats.cooldown_ticks, 200);
        assert_eq!(boro.stats.unlock_mastery, 4);

        let tatsu = m.ability("tatsumaki").unwrap();
        assert_eq!(tatsu.cast_type, CastType::Held);
        assert_eq!(tatsu.stats.energy_cost, 50.0);
        assert_eq!(tatsu.stats.max_channel_ticks, 120);

        let ragnaraku = m.ability("kosanze_ragnaraku").unwrap();
        assert_eq!(ragnaraku.stats.energy_cost, 25.0);
        assert_eq!(ragnaraku.stats.damage, 10.0);

        let homura = m.ability("homuragumo").unwrap();
        assert_eq!(homura.stats.energy_cost, 20.0);
        assert_eq!(homura.stats.range, 8.0);

        let kaen = m.ability("kaen_daiko").unwrap();
        assert!(kaen.is_ultimate);
        assert_eq!(kaen.stats.energy_cost, 80.0);
        assert_eq!(kaen.stats.cooldown_ticks, 600);
    }

    #[test]
    fn seiryu_boro_breath_tatsumaki_ragnaraku_homuragumo_and_kaen_daiko() {
        let m = UoUoSeiryuModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::UoUoSeiryu),
            mastery: 10,
            energy: 300.0,
            active_form: "human".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut wfx = WorldFxManager::default();
        let targets = [test_target(2, DVec3::new(0.0, 0.0, 3.0))];

        let mut ctx = FruitContext {
            player_id: [1; 16],
            player: &mut player,
            form_runtime: &mut form_rt,
            gear_state: &mut gear,
            haki: &mut haki,
            logia: None,
            heat: &mut heat,
            world_fx: &mut wfx,
            budget: CastBudget::from_config(&cfg),
            ability_ctx: AbilityContext {
                caster: [1; 16],
                fruit: FruitId::UoUoSeiryu,
                ability_name: "hybrid_form".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 100,
                eye_pos: DVec3::ZERO,
                look_dir: DVec3::Z,
                aim_point: None,
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

        // Switch to Hybrid form and cast Kosanze Ragnaraku
        let hyb_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "hybrid_form".into(),
            },
        );
        assert_eq!(hyb_res.form_switch_to.as_deref(), Some("hybrid"));
        let hyb_mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(hyb_mods.fire_immune);
        assert!(!hyb_mods.passes_through_leaves);

        let rag_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kosanze_ragnaraku".into(),
                mastery: 10,
            },
        );
        assert_eq!(rag_res.hits.len(), 1);
        assert_eq!(rag_res.hits[0].stun_ticks, 30);
        assert!(rag_res.screen_shake.is_some());

        // Homuragumo flame cloud dash enables flight and burns target
        let hom_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "homuragumo".into(),
                mastery: 10,
            },
        );
        assert!(hom_res.caster_velocity_impulse.is_some());
        assert_eq!(hom_res.hits[0].set_fire_seconds, 4);
        assert!(ctx.form_runtime.has_flag(form_flags::FLYING));

        // Switch to Azure Dragon form: check leaves pass-through & immovable mass in body_modifiers
        let drg_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "dragon_form".into(),
            },
        );
        assert_eq!(drg_res.form_switch_to.as_deref(), Some("dragon"));
        let drg_mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(drg_mods.passes_through_leaves);
        assert!(drg_mods.fire_immune);
        assert!(drg_mods.is_immovable());
        assert_eq!(drg_mods.mass_tier, EntityMassTier::Immovable);
        assert_eq!(drg_mods.mass_kg_override, Some(ENTITY_MASS_HEAVY_TARGET_KG));

        // Boro Breath in Dragon form fires 14 meteors and deals SeiryuFlame damage
        let boro_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "boro_breath".into(),
                charge: 1.0,
                mastery: 10,
            },
        );
        assert_eq!(boro_res.damage_type, Some(GrandLineDamageType::SeiryuFlame));
        assert_eq!(boro_res.spawned_projectiles, 14);
        assert_eq!(boro_res.hits.len(), 1);
        assert_eq!(boro_res.hits[0].set_fire_seconds, 4);

        // Tatsumaki vortex pull & lift
        let tatsu_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChannelTick {
                ability: "tatsumaki".into(),
                elapsed_ticks: 10,
            },
        );
        assert_eq!(tatsu_res.hits.len(), 1);
        assert!(tatsu_res.hits[0].damage > 0.0);

        // Kaen Daiko ultimate
        let kaen_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kaen_daiko".into(),
                mastery: 10,
            },
        );
        assert_eq!(kaen_res.hits.len(), 1);
        assert_eq!(kaen_res.hits[0].set_fire_seconds, 6);
    }
}
