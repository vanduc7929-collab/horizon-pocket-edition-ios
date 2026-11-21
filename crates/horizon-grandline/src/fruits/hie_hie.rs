//! Hie Hie no Mi (Ice Logia) — Grand Line 0.4.0 1:1 port.
//!
//! Mirrors `fruits/hie_hie/{HieHie,HieAbilities,HieCold,HieEvents,HieFx,IceFreeze,IceWalk}.java`
//! and `fruits/hie_hie/ability/{IceSaberAbility,SaberSlashAbility,SaberGuardAbility,
//! TimeCapsuleAbility,IceBallAbility,IceBikeAbility,PartisanAbility,PheasantPeckAbility,IceAgeAbility}.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::{EntityBodyModifiers, FRICTION_GROUND};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastBudget, CastResult, CastType,
};
use crate::core::cold::{ChillBurst, FrozenKind, FrozenRules};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::FormRuntime;
use crate::core::fruit::FruitId;
use crate::core::logia::{LogiaManager, LogiaProfile, SolidBodySpec, limbs};
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Canonical Hie-Hie constants (`HieHie.java`, `HieCold.java`, `IceWalk.java`, abilities)
// ---------------------------------------------------------------------------

pub const HIE_COLD_STYLE: &str = "devilfruits:hie_hie/ice";

/// `HieCold.BURST = new Cold.Burst(4.0F, HieHie.FROST_DAMAGE, 70, 5)`
pub const HIE_CHILL_BURST: ChillBurst = ChillBurst {
    damage: 4.0,
    damage_type: Some(GrandLineDamageType::HieFrost),
    slow_ticks: 70,
    frostbite_level: 5,
};

/// `HieHie.java` settings for Ice Walk passive (`IceWalk.java`).
pub const ICE_WALK_EVERY_TICKS: i64 = 2;
pub const ICE_WALK_RADIUS: f64 = 2.5;
pub const ICE_WALK_THAW_SECONDS: f64 = 25.0;
pub const ICE_WALK_THAW_TICKS: i32 = 500;
pub const ICE_WALK_BLOCKS_PER_STEP: usize = 40;

/// `HieHie.java` settings for Ice Bike (`IceBikeAbility.java`).
pub const ICE_BIKE_CRUISE_SPEED: f64 = 1.0;
pub const ICE_BIKE_BOOST_SPEED: f64 = 1.8;
pub const ICE_BIKE_GRIP: f64 = 0.82;
pub const ICE_BIKE_THAW_SECONDS: f64 = 12.0;
pub const ICE_BIKE_RAM_DAMAGE: f32 = 3.0;

/// `IceSaberAbility.java` constants.
pub const ICE_SABER_ENERGY: f32 = 8.0;
pub const ICE_SABER_DRAIN_PER_SECOND: f32 = 1.0;
pub const ICE_SABER_COOLDOWN_TICKS: i32 = 30;
pub const ICE_SABER_UNLOCK_MASTERY: u32 = 1;
pub const ICE_SABER_SUMMON_TICKS: i32 = 10;

/// `SaberSlashAbility.java` constants.
pub const SABER_SLASH_STRIKES: [i32; 5] = [4, 4, 5, 4, 7];
pub const SABER_SLASH_POWER: [f32; 5] = [1.0, 1.0, 1.1, 1.15, 1.4];
pub const SABER_SLASH_ENERGY: f32 = 1.0;
pub const SABER_SLASH_FINISHER_ENERGY: f32 = 5.0;
pub const SABER_SLASH_COOLDOWN_TICKS: i32 = 16;
pub const SABER_SLASH_DAMAGE: f32 = 9.0;
pub const SABER_SLASH_RANGE: f32 = 3.6;
pub const SABER_SLASH_KNOCKBACK: f32 = 0.35;
pub const SABER_SLASH_ARC_DEGREES: f64 = 110.0;
pub const SABER_SLASH_FROSTBITE_TICKS: i32 = 60;
pub const SABER_SLASH_FROSTBITE_LEVEL: i32 = 2;
pub const SABER_SLASH_CRESCENT_RANGE: f64 = 20.0;
pub const SABER_SLASH_CRESCENT_DAMAGE: f32 = 11.0;
pub const SABER_SLASH_CRESCENT_SPAN: f64 = 3.0;

/// `SaberGuardAbility.java` constants.
pub const SABER_GUARD_ENERGY: f32 = 3.0;
pub const SABER_GUARD_COOLDOWN_TICKS: i32 = 20;
pub const SABER_GUARD_KNOCKBACK: f32 = 0.7;
pub const SABER_GUARD_WINDOW_TICKS: i32 = 8;

/// `TimeCapsuleAbility.java` (`ice_time_capsule` / `ice_time`) constants.
pub const TIME_CAPSULE_ENERGY: f32 = 25.0;
pub const TIME_CAPSULE_COOLDOWN_TICKS: i32 = 280;
pub const TIME_CAPSULE_UNLOCK_MASTERY: u32 = 2;
pub const TIME_CAPSULE_RANGE: f32 = 10.0;
pub const TIME_CAPSULE_WIDTH: f64 = 3.0;
pub const TIME_CAPSULE_RUN_TICKS: i32 = 3;
pub const TIME_CAPSULE_STATUE_TICKS: i32 = 100; // 5.0s * 20
pub const TIME_CAPSULE_THAW_TICKS: i32 = 400; // 20.0s * 20
pub const TIME_CAPSULE_MAX_BLOCKS: usize = 120;

/// `IceBallAbility.java` constants.
pub const ICE_BALL_ENERGY: f32 = 30.0;
pub const ICE_BALL_COOLDOWN_TICKS: i32 = 320;
pub const ICE_BALL_UNLOCK_MASTERY: u32 = 3;
pub const ICE_BALL_DAMAGE: f32 = 2.0;
pub const ICE_BALL_RANGE: f32 = 24.0;
pub const ICE_BALL_SPEED: f64 = 1.8;
pub const ICE_BALL_RADIUS: f64 = 1.2;
pub const ICE_BALL_MAX_TARGETS: usize = 3;
pub const ICE_BALL_FREEZE_TICKS: i32 = 160; // 8.0s * 20
pub const ICE_BALL_STRUGGLE_PRESSES: f32 = 20.0;
pub const ICE_BALL_THAW_TICKS: i32 = 400;

/// `IceBikeAbility.java` constants.
pub const ICE_BIKE_ENERGY: f32 = 8.0;
pub const ICE_BIKE_DRAIN_PER_SECOND: f32 = 2.0;
pub const ICE_BIKE_COOLDOWN_TICKS: i32 = 40;
pub const ICE_BIKE_UNLOCK_MASTERY: u32 = 4;

/// `PartisanAbility.java` (`ice_partisan` / `partisan` / `ice_block_partisan`) constants.
pub const PARTISAN_ENERGY: f32 = 28.0;
pub const PARTISAN_COOLDOWN_TICKS: i32 = 240;
pub const PARTISAN_UNLOCK_MASTERY: u32 = 5;
pub const PARTISAN_DAMAGE: f32 = 8.0;
pub const PARTISAN_RANGE: f32 = 40.0;
pub const PARTISAN_KNOCKBACK: f32 = 0.4;
pub const PARTISAN_SPEED: f64 = 3.0;
pub const PARTISAN_FAN_DEGREES: f64 = 4.0;
pub const PARTISAN_CAPSULE_RADIUS: f64 = 0.45;
pub const PARTISAN_STICK_TICKS: i32 = 80; // 4.0s * 20
pub const PARTISAN_THAW_TICKS: i32 = 400;

/// `PheasantPeckAbility.java` (`pheasant_peck` / `pheasant_beak`) constants.
pub const PHEASANT_ENERGY: f32 = 36.0;
pub const PHEASANT_COOLDOWN_TICKS: i32 = 320;
pub const PHEASANT_UNLOCK_MASTERY: u32 = 6;
pub const PHEASANT_DAMAGE: f32 = 12.0;
pub const PHEASANT_RANGE: f32 = 24.0;
pub const PHEASANT_KNOCKBACK: f32 = 0.6;
pub const PHEASANT_SPEED: f64 = 4.5;
pub const PHEASANT_HOMING_DEGREES: f64 = 10.0;
pub const PHEASANT_FREEZE_TICKS: i32 = 160; // 8.0s * 20
pub const PHEASANT_STRUGGLE_PRESSES: f32 = 20.0;
pub const PHEASANT_STRUGGLE_FACTOR: f32 = 0.25;
pub const PHEASANT_BURST_RADIUS: f64 = 2.5;
pub const PHEASANT_THAW_TICKS: i32 = 400;

/// `IceAgeAbility.java` (`ice_age`) constants.
pub const ICE_AGE_ENERGY: f32 = 90.0;
pub const ICE_AGE_COOLDOWN_TICKS: i32 = 2400;
pub const ICE_AGE_UNLOCK_MASTERY: u32 = 9;
pub const ICE_AGE_DAMAGE: f32 = 10.0;
pub const ICE_AGE_RANGE: f32 = 16.0;
pub const ICE_AGE_SPREAD_TICKS: i32 = 30;
pub const ICE_AGE_FREEZE_TICKS: i32 = 120; // 6.0s * 20
pub const ICE_AGE_STRUGGLE_PRESSES: f32 = 30.0;
pub const ICE_AGE_STRUGGLE_FACTOR: f32 = 0.5;
pub const ICE_AGE_SNOW_FRACTION: f64 = 0.35;
pub const ICE_AGE_THAW_TICKS: i32 = 900; // 45.0s * 20
pub const ICE_AGE_MAX_BLOCKS: usize = 2500;

/// Canonical Hie-Hie VFX resource IDs (`HieFx.java`).
pub mod hie_fx {
    pub const AWAKEN: &str = "devilfruits:hie_hie/awaken";
    pub const LEVEL_UP: &str = "devilfruits:hie_hie/level_up";
    pub const REINCARNATE: &str = "devilfruits:hie_hie/reincarnate";
    pub const ICE_WALK: &str = "devilfruits:hie_hie/ice_walk";
    pub const ICE_RUSH: &str = "devilfruits:hie_hie/ice_rush";
    pub const ICE_BREAK: &str = "devilfruits:hie_hie/ice_break";
    pub const TIME_CAPSULE: &str = "devilfruits:hie_hie/time_capsule";
    pub const ICE_BALL_CHARGE: &str = "devilfruits:hie_hie/ice_ball_charge";
    pub const ICE_BALL: &str = "devilfruits:hie_hie/ice_ball";
    pub const ICE_BALL_END: &str = "devilfruits:hie_hie/ice_ball_end";
    pub const SABER_SUMMON: &str = "devilfruits:hie_hie/saber_summon";
    pub const SABER_DISMISS: &str = "devilfruits:hie_hie/saber_dismiss";
    pub const SABER_SWING: &str = "devilfruits:hie_hie/saber_swing";
    pub const SABER_HIT: &str = "devilfruits:hie_hie/saber_hit";
    pub const SABER_CRESCENT: &str = "devilfruits:hie_hie/saber_crescent";
    pub const SABER_CRESCENT_END: &str = "devilfruits:hie_hie/saber_crescent_end";
    pub const SABER_GUARD: &str = "devilfruits:hie_hie/saber_guard";
    pub const PARTISAN_FORM: &str = "devilfruits:hie_hie/partisan_form";
    pub const PARTISAN_THROW: &str = "devilfruits:hie_hie/partisan_throw";
    pub const PARTISAN_END: &str = "devilfruits:hie_hie/partisan_end";
    pub const PHEASANT_FORM: &str = "devilfruits:hie_hie/pheasant_form";
    pub const PHEASANT_LAUNCH: &str = "devilfruits:hie_hie/pheasant_launch";
    pub const PHEASANT_END: &str = "devilfruits:hie_hie/pheasant_end";
    pub const ICE_AGE_CHARGE: &str = "devilfruits:hie_hie/ice_age_charge";
    pub const ICE_AGE: &str = "devilfruits:hie_hie/ice_age";
    pub const ICE_BIKE: &str = "devilfruits:hie_hie/ice_bike";
}

/// Canonical Hie-Hie animation clip IDs.
pub mod hie_anim {
    pub const ICE_BALL_THRUST: &str = "devilfruits:hie_hie/ice_ball_thrust";
    pub const ICE_SABER_SUMMON: &str = "devilfruits:hie_hie/ice_saber_summon";
    pub const SABER_GUARD: &str = "devilfruits:hie_hie/saber_guard";
    pub const TIME_CAPSULE_STOMP: &str = "devilfruits:hie_hie/time_capsule_stomp";
    pub const PARTISAN_WINDUP: &str = "devilfruits:hie_hie/partisan_windup";
    pub const PARTISAN_THROW: &str = "devilfruits:hie_hie/partisan_throw";
    pub const PHEASANT_FORM: &str = "devilfruits:hie_hie/pheasant_form";
    pub const PHEASANT_LAUNCH: &str = "devilfruits:hie_hie/pheasant_launch";
    pub const ICE_AGE_SPREAD: &str = "devilfruits:hie_hie/ice_age_spread";
    pub const BIKE_RIDE: &str = "devilfruits:hie_hie/bike_ride";
}

/// Hie Hie no Mi Devil Fruit module (`HieHie.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct HieHieModule;

impl HieHieModule {
    pub const fn new() -> Self {
        Self
    }

    /// Frozen rules for `TimeCapsuleAbility` (`FrozenKind::Statue`, 100 ticks, unbreakable, 0 struggle).
    pub const fn time_capsule_freeze_rules() -> FrozenRules {
        FrozenRules::of(FrozenKind::Statue, TIME_CAPSULE_STATUE_TICKS)
            .with_breakable(false)
            .with_struggle(0.0, 0.0)
    }

    /// Frozen rules for `IceBallAbility` (`FrozenKind::Block`, 160 ticks, breakable, 20 presses × 1.0).
    pub const fn ice_ball_freeze_rules() -> FrozenRules {
        FrozenRules::of(FrozenKind::Block, ICE_BALL_FREEZE_TICKS)
            .with_breakable(true)
            .with_struggle(ICE_BALL_STRUGGLE_PRESSES, 1.0)
    }

    /// Frozen rules for `PheasantPeckAbility` (`FrozenKind::Crystal`, 160 ticks, breakable, 20 presses × 0.25).
    pub const fn pheasant_freeze_rules() -> FrozenRules {
        FrozenRules::of(FrozenKind::Crystal, PHEASANT_FREEZE_TICKS)
            .with_breakable(true)
            .with_struggle(PHEASANT_STRUGGLE_PRESSES, PHEASANT_STRUGGLE_FACTOR)
    }

    /// Frozen rules for `IceAgeAbility` (`FrozenKind::Crystal`, 120 ticks, breakable, 30 presses × 0.5).
    pub const fn ice_age_freeze_rules() -> FrozenRules {
        FrozenRules::of(FrozenKind::Crystal, ICE_AGE_FREEZE_TICKS)
            .with_breakable(true)
            .with_struggle(ICE_AGE_STRUGGLE_PRESSES, ICE_AGE_STRUGGLE_FACTOR)
    }

    /// Computes the 3 trident headings for `PartisanAbility.heading(look, i)` (`0°`, `+4°`, `-4°`).
    pub fn partisan_headings(look: DVec3) -> [DVec3; 3] {
        let l = look.normalize_or_zero();
        if l.length_squared() < 1e-8 {
            return [DVec3::Z, DVec3::Z, DVec3::Z];
        }
        let rot = |deg: f64| {
            let a = deg.to_radians();
            let (sin, cos) = a.sin_cos();
            DVec3::new(l.x * cos - l.z * sin, l.y, l.x * sin + l.z * cos).normalize_or_zero()
        };
        [l, rot(PARTISAN_FAN_DEGREES), rot(-PARTISAN_FAN_DEGREES)]
    }

    /// Computes active counter weaken multiplier (e.g. `0.7` when `"heat"` counter is active).
    fn counter_damage_scale(ctx: &FruitContext<'_>) -> f32 {
        match (LogiaProfile::for_fruit(FruitId::HieHie), ctx.logia.as_deref()) {
            (Some(profile), Some(logia)) => {
                LogiaManager::active_weaken_multiplier(&profile, logia, ctx.ability_ctx.tick)
            }
            _ => 1.0,
        }
    }

    /// Dissolves limbs briefly when casting elemental techniques (`LogiaManager.partial`).
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

    /// Normalizes ability alias names so both `FruitMeta` names (`"partisan"`, `"pheasant_beak"`)
    /// and Java internal names (`"ice_partisan"`, `"ice_block_partisan"`, `"pheasant_peck"`, `"ice_time"`) resolve cleanly.
    fn canonical_ability_name(name: &str) -> &str {
        let stripped = name
            .split(':')
            .next_back()
            .unwrap_or(name)
            .split('/')
            .next_back()
            .unwrap_or(name);
        match stripped {
            "ice_partisan" | "ice_block_partisan" => "partisan",
            "pheasant_peck" => "pheasant_beak",
            "ice_time" => "ice_time_capsule",
            other => other,
        }
    }
}

impl DevilFruitModule for HieHieModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::HieHie
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let allowed = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "ice_saber"),
                display_name: "Ice Saber".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: ICE_SABER_ENERGY,
                    energy_per_second: ICE_SABER_DRAIN_PER_SECOND,
                    cooldown_ticks: ICE_SABER_COOLDOWN_TICKS,
                    unlock_mastery: ICE_SABER_UNLOCK_MASTERY,
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
                id: AbilityId::new(FruitId::HieHie, "saber_slash"),
                display_name: "Ice Saber Slash".into(),
                cast_type: CastType::Combo,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SABER_SLASH_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SABER_SLASH_COOLDOWN_TICKS,
                    unlock_mastery: 1,
                    damage: SABER_SLASH_DAMAGE,
                    range: SABER_SLASH_RANGE,
                    knockback: SABER_SLASH_KNOCKBACK,
                    max_combo_steps: 5,
                    combo_window_ticks: 16,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "saber_guard"),
                display_name: "Ice Saber Guard".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SABER_GUARD_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SABER_GUARD_COOLDOWN_TICKS,
                    unlock_mastery: 1,
                    damage: 0.0,
                    range: 0.0,
                    knockback: SABER_GUARD_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "ice_time_capsule"),
                display_name: "Ice Time Capsule".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: TIME_CAPSULE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: TIME_CAPSULE_COOLDOWN_TICKS,
                    unlock_mastery: TIME_CAPSULE_UNLOCK_MASTERY,
                    damage: 0.0,
                    range: TIME_CAPSULE_RANGE,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "ice_ball"),
                display_name: "Ice Ball".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: ICE_BALL_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: ICE_BALL_COOLDOWN_TICKS,
                    unlock_mastery: ICE_BALL_UNLOCK_MASTERY,
                    damage: ICE_BALL_DAMAGE,
                    range: ICE_BALL_RANGE,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "ice_bike"),
                display_name: "Ice Bike".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: ICE_BIKE_ENERGY,
                    energy_per_second: ICE_BIKE_DRAIN_PER_SECOND,
                    cooldown_ticks: ICE_BIKE_COOLDOWN_TICKS,
                    unlock_mastery: ICE_BIKE_UNLOCK_MASTERY,
                    damage: ICE_BIKE_RAM_DAMAGE,
                    range: 0.0,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "partisan"),
                display_name: "Ice Block: Partisan".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: PARTISAN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: PARTISAN_COOLDOWN_TICKS,
                    unlock_mastery: PARTISAN_UNLOCK_MASTERY,
                    damage: PARTISAN_DAMAGE,
                    range: PARTISAN_RANGE,
                    knockback: PARTISAN_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "pheasant_beak"),
                display_name: "Ice Block: Pheasant Beak".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: PHEASANT_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: PHEASANT_COOLDOWN_TICKS,
                    unlock_mastery: PHEASANT_UNLOCK_MASTERY,
                    damage: PHEASANT_DAMAGE,
                    range: PHEASANT_RANGE,
                    knockback: PHEASANT_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::HieHie, "ice_age"),
                display_name: "Ice Age".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: ICE_AGE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: ICE_AGE_COOLDOWN_TICKS,
                    unlock_mastery: ICE_AGE_UNLOCK_MASTERY,
                    damage: ICE_AGE_DAMAGE,
                    range: ICE_AGE_RANGE,
                    knockback: 0.0,
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
            damage_type: Some(GrandLineDamageType::HieIce),
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
            "ice_saber" => {
                if let AbilityEvent::Toggled { enabled, .. } = event {
                    res.cast_result = Some(CastResult::Toggled);
                    if *enabled {
                        Self::dissolve_arms(ctx, limbs::RIGHT_ARM, ICE_SABER_SUMMON_TICKS);
                        res.animation_clip = Some(hie_anim::ICE_SABER_SUMMON.into());
                        ctx.world_fx.emit_fx(
                            hie_fx::SABER_SUMMON,
                            0,
                            eye,
                            DVec3::ZERO,
                            0.0,
                            now,
                            ICE_SABER_SUMMON_TICKS,
                            64.0,
                        );
                    } else {
                        ctx.world_fx.emit_fx(
                            hie_fx::SABER_DISMISS,
                            0,
                            eye,
                            DVec3::ZERO,
                            0.0,
                            now,
                            0,
                            64.0,
                        );
                    }
                }
            }
            "saber_slash" => {
                let step = match event {
                    AbilityEvent::ComboStep { step, .. } => (*step).clamp(1, 5) - 1,
                    _ => ctx.ability_ctx.combo_step.clamp(1, 5) - 1,
                } as usize;
                res.cast_result = Some(CastResult::ComboStep);
                res.animation_clip = Some(format!("devilfruits:hie_hie/saber_m1_{}", step + 1));

                let slash_dmg = base_scaled_dmg * SABER_SLASH_POWER[step];
                let half_angle = (SABER_SLASH_ARC_DEGREES * 0.5).to_radians();
                let cone_hits = CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    half_angle,
                );
                let kb_scale = def.stats.knockback as f64 * if step == 4 { 1.6 } else { 1.0 };
                let push_vec = horiz_look * kb_scale + DVec3::new(0.0, 0.1, 0.0);
                let mut any_struck = false;
                for t in cone_hits {
                    any_struck = true;
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: slash_dmg,
                        damage_type: GrandLineDamageType::HieIce,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push_vec,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 1,
                        chill_burst: Some(HIE_CHILL_BURST),
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx.emit_fx(
                        hie_fx::SABER_HIT,
                        0,
                        t.center,
                        look,
                        step as f32,
                        now,
                        0,
                        64.0,
                    );
                }

                ctx.world_fx.emit_fx(
                    hie_fx::SABER_SWING,
                    0,
                    eye,
                    look,
                    scaled_range as f32,
                    now,
                    (step as i32) | if any_struck { 8 } else { 0 },
                    64.0,
                );

                // 5th combo step launches the flying ice crescent (`SaberSlashAbility.crescent`).
                if step == 4 {
                    res.spawned_projectiles = 1;
                    let crescent_dmg = (SABER_SLASH_CRESCENT_DAMAGE / SABER_SLASH_DAMAGE) * base_scaled_dmg;
                    let crescent_end = eye + look * SABER_SLASH_CRESCENT_RANGE;
                    let crescent_half = SABER_SLASH_CRESCENT_SPAN * 0.5;
                    ctx.world_fx.emit_fx(
                        hie_fx::SABER_CRESCENT,
                        0,
                        eye + look * 1.2 + DVec3::new(0.0, -0.4, 0.0),
                        look * 1.5,
                        crescent_half as f32,
                        now,
                        14,
                        96.0,
                    );
                    let crescent_hits = CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        crescent_end,
                        crescent_half,
                    );
                    for t in crescent_hits {
                        if !res.hits.iter().any(|h| h.target_id == t.id) {
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: crescent_dmg,
                                damage_type: GrandLineDamageType::HieIce,
                                knockback_impulse: CombatTargeting::launch_velocity(
                                    look * (def.stats.knockback as f64 * 1.2)
                                        + DVec3::new(0.0, 0.2, 0.0),
                                    t.knockback_resistance,
                                ),
                                stun_ticks: 0,
                                chill_stacks_added: 1,
                                chill_burst: Some(HIE_CHILL_BURST),
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }
                    ctx.world_fx.emit_fx(
                        hie_fx::SABER_CRESCENT_END,
                        0,
                        crescent_end,
                        look * 1.5,
                        crescent_half as f32,
                        now,
                        0,
                        96.0,
                    );
                }
            }
            "saber_guard" => {
                res.animation_clip = Some(hie_anim::SABER_GUARD.into());
                ctx.world_fx.emit_fx(
                    hie_fx::SABER_GUARD,
                    0,
                    eye,
                    look,
                    0.0,
                    now,
                    SABER_GUARD_WINDOW_TICKS,
                    64.0,
                );
            }
            "ice_time_capsule" => {
                res.animation_clip = Some(hie_anim::TIME_CAPSULE_STOMP.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_LEG, 6);
                let dir = if horiz_look.length_squared() > 1e-6 {
                    horiz_look
                } else {
                    DVec3::Z
                };
                let half_width = TIME_CAPSULE_WIDTH * 0.5;
                let end = eye + dir * scaled_range;
                ctx.world_fx.emit_fx(
                    hie_fx::TIME_CAPSULE,
                    0,
                    eye,
                    dir * scaled_range,
                    half_width as f32,
                    now,
                    TIME_CAPSULE_RUN_TICKS,
                    96.0,
                );

                // Lay ice sheets along the strip (`TimeCapsuleAbility.wave`).
                let mut strip_budget = CastBudget::new(TIME_CAPSULE_MAX_BLOCKS, 0);
                let base_y = (eye.y - 1.6).floor() as i32;
                let steps = (scaled_range * 2.0).ceil() as i32;
                for i in 0..=steps {
                    let p = eye + dir * (i as f64 * 0.5);
                    let pos = [p.x.floor() as i32, base_y, p.z.floor() as i32];
                    if ctx.world_fx.transform_block(
                        pos,
                        "minecraft:water",
                        "devilfruits:sea_ice",
                        1.0,
                        false,
                        &mut strip_budget,
                        TIME_CAPSULE_THAW_TICKS,
                        Some("minecraft:water"),
                        now,
                        ctx.config,
                    ) {
                        res.transformed_blocks
                            .push((pos, "devilfruits:sea_ice".into()));
                    }
                }

                // Freeze targets on the strip as unbreakable STATUE for 5.0s (`100` ticks).
                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    end,
                    half_width + 0.3,
                );
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: 0.0,
                        damage_type: GrandLineDamageType::HieIce,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: TIME_CAPSULE_STATUE_TICKS,
                        chill_stacks_added: 1,
                        chill_burst: Some(HIE_CHILL_BURST),
                        freeze_ticks: TIME_CAPSULE_STATUE_TICKS,
                        set_fire_seconds: 0,
                    });
                }
            }
            "ice_ball" => {
                res.animation_clip = Some(hie_anim::ICE_BALL_THRUST.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::ARMS, 10);
                let hands = eye + look * 0.8 + DVec3::new(0.0, -0.35, 0.0);
                let vel = look * ICE_BALL_SPEED;
                let life = ((scaled_range / ICE_BALL_SPEED).ceil() as i32).max(1);
                let end = hands + look * scaled_range;

                ctx.world_fx.emit_fx(
                    hie_fx::ICE_BALL_CHARGE,
                    0,
                    eye,
                    DVec3::ZERO,
                    1.0,
                    now,
                    6,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    hie_fx::ICE_BALL,
                    0,
                    hands,
                    vel,
                    ICE_BALL_RADIUS as f32,
                    now,
                    life,
                    128.0,
                );

                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    hands,
                    end,
                    ICE_BALL_RADIUS,
                );
                for t in hits.into_iter().take(ICE_BALL_MAX_TARGETS) {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::HieIce,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: ICE_BALL_FREEZE_TICKS,
                        chill_stacks_added: 1,
                        chill_burst: Some(HIE_CHILL_BURST),
                        freeze_ticks: ICE_BALL_FREEZE_TICKS,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    hie_fx::ICE_BALL_END,
                    0,
                    end,
                    vel,
                    ICE_BALL_RADIUS as f32,
                    now,
                    0,
                    128.0,
                );
            }
            "ice_bike" => {
                if let AbilityEvent::Toggled { enabled, .. } = event {
                    res.cast_result = Some(CastResult::Toggled);
                    if *enabled {
                        res.animation_clip = Some(hie_anim::BIKE_RIDE.into());
                        ctx.world_fx.emit_fx(
                            hie_fx::ICE_BIKE,
                            0,
                            eye,
                            DVec3::ZERO,
                            1.0,
                            now,
                            1,
                            64.0,
                        );
                    }
                }
            }
            "partisan" => {
                res.animation_clip = Some(hie_anim::PARTISAN_THROW.into());
                res.spawned_projectiles = 3;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 14);
                let vel_center = look * PARTISAN_SPEED;
                let life = ((scaled_range / PARTISAN_SPEED).ceil() as i32).max(1);

                ctx.world_fx.emit_fx(
                    hie_fx::PARTISAN_FORM,
                    0,
                    eye,
                    look,
                    0.0,
                    now,
                    11,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    hie_fx::PARTISAN_THROW,
                    0,
                    eye,
                    vel_center,
                    PARTISAN_SPEED as f32,
                    now,
                    life,
                    128.0,
                );

                // 3 fanned ice tridents at 0°, +4°, -4° (`PartisanAbility.heading`).
                let headings = Self::partisan_headings(look);
                for (i, dir_i) in headings.iter().copied().enumerate() {
                    let end_i = eye + dir_i * scaled_range;
                    let push = dir_i * (def.stats.knockback as f64) + DVec3::new(0.0, 0.1, 0.0);
                    let hits = CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        end_i,
                        PARTISAN_CAPSULE_RADIUS,
                    );
                    for t in hits {
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::HieIce,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 1,
                            chill_burst: Some(HIE_CHILL_BURST),
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                    ctx.world_fx.emit_fx(
                        hie_fx::PARTISAN_END,
                        0,
                        end_i,
                        dir_i * PARTISAN_SPEED,
                        i as f32,
                        now,
                        PARTISAN_STICK_TICKS,
                        128.0,
                    );
                }
            }
            "pheasant_beak" => {
                res.animation_clip = Some(hie_anim::PHEASANT_LAUNCH.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 12);
                let start = eye
                    + (if horiz_look.length_squared() > 1e-6 {
                        horiz_look
                    } else {
                        DVec3::Z
                    }) * 1.6
                    + DVec3::new(0.0, 0.9, 0.0);
                let life = ((scaled_range / PHEASANT_SPEED).ceil() as i32).max(1);

                // Pick homing quarry in 10° cone (`PheasantPeckAbility.quarry`).
                let quarry = CombatTargeting::aimed(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    PHEASANT_HOMING_DEGREES.to_radians(),
                );
                let end = quarry.map(|q| q.center).unwrap_or(start + look * scaled_range);
                let flight_dir = (end - start).normalize_or_zero();
                let vel = if flight_dir.length_squared() > 1e-6 {
                    flight_dir * PHEASANT_SPEED
                } else {
                    look * PHEASANT_SPEED
                };

                ctx.world_fx.emit_fx(
                    hie_fx::PHEASANT_FORM,
                    0,
                    eye,
                    look,
                    0.0,
                    now,
                    6,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    hie_fx::PHEASANT_LAUNCH,
                    0,
                    start,
                    vel,
                    if quarry.is_some() { 1.0 } else { -1.0 },
                    now,
                    life,
                    160.0,
                );

                // Direct hit along capsule or homing quarry seals target in CRYSTAL (`struggleFactor = 0.25`).
                let capsule_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    1.0,
                );
                let primary = quarry.or_else(|| capsule_hits.first().copied());
                if let Some(t) = primary {
                    let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.15, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::HieIce,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: PHEASANT_FREEZE_TICKS,
                        chill_stacks_added: 1,
                        chill_burst: Some(HIE_CHILL_BURST),
                        freeze_ticks: PHEASANT_FREEZE_TICKS,
                        set_fire_seconds: 0,
                    });

                    // Shatter burst chills everything within 2.5 blocks (`PheasantPeckAbility.shatter`).
                    let burst_hits = CombatTargeting::in_radius(
                        ctx.nearby_targets,
                        ctx.player_id,
                        t.center,
                        PHEASANT_BURST_RADIUS,
                    );
                    for extra in burst_hits {
                        if extra.id != t.id {
                            res.hits.push(AbilityHitEffect {
                                target_id: extra.id,
                                damage: 0.0,
                                damage_type: GrandLineDamageType::HieFrost,
                                knockback_impulse: DVec3::ZERO,
                                stun_ticks: 0,
                                chill_stacks_added: 1,
                                chill_burst: Some(HIE_CHILL_BURST),
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }
                }

                ctx.world_fx.emit_fx(
                    hie_fx::PHEASANT_END,
                    0,
                    end,
                    vel,
                    if primary.is_some() { 1.0 } else { 0.0 },
                    now,
                    0,
                    160.0,
                );
            }
            "ice_age" => {
                res.animation_clip = Some(hie_anim::ICE_AGE_SPREAD.into());
                Self::dissolve_arms(ctx, limbs::ALL, 26);

                ctx.world_fx.emit_fx(
                    hie_fx::ICE_AGE_CHARGE,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    now,
                    14,
                    96.0,
                );
                ctx.world_fx.emit_fx(
                    hie_fx::ICE_AGE,
                    0,
                    eye,
                    DVec3::new(ICE_AGE_THAW_TICKS as f64, 0.0, 0.0),
                    scaled_range as f32,
                    now,
                    ICE_AGE_SPREAD_TICKS,
                    160.0,
                );
                ctx.world_fx.emit_shake(eye, 0.85, 32);
                res.screen_shake = Some((0.85, 32));

                // Freeze surrounding terrain columns within budget (`IceAgeAbility.release`).
                let mut budget = CastBudget::new(ICE_AGE_MAX_BLOCKS, 0);
                let cx = eye.x.floor() as i32;
                let cy = (eye.y - 1.6).floor() as i32;
                let cz = eye.z.floor() as i32;
                let r_ceil = (scaled_range.ceil() as i32).min(16);
                let r2 = scaled_range * scaled_range;
                for dx in -r_ceil..=r_ceil {
                    for dz in -r_ceil..=r_ceil {
                        if (dx * dx + dz * dz) as f64 <= r2 {
                            let pos = [cx + dx, cy, cz + dz];
                            let dist = ((dx * dx + dz * dz) as f64).sqrt();
                            let thaw = ICE_AGE_THAW_TICKS + ((scaled_range - dist) * 8.0) as i32;
                            if ctx.world_fx.transform_block(
                                pos,
                                "minecraft:water",
                                "devilfruits:sea_ice",
                                1.0,
                                false,
                                &mut budget,
                                thaw,
                                Some("minecraft:water"),
                                now,
                                ctx.config,
                            ) {
                                res.transformed_blocks
                                    .push((pos, "devilfruits:sea_ice".into()));
                            }
                        }
                    }
                }

                // Encase all enemies within radius (and |dy| <= 6.0) in CRYSTAL for 6.0s (`120` ticks).
                let area_hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    scaled_range + 0.5,
                );
                for t in area_hits {
                    if (t.center.y - eye.y).abs() <= 6.0 {
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::HieIce,
                            knockback_impulse: DVec3::ZERO,
                            stun_ticks: ICE_AGE_FREEZE_TICKS,
                            chill_stacks_added: 1,
                            chill_burst: Some(HIE_CHILL_BURST),
                            freeze_ticks: ICE_AGE_FREEZE_TICKS,
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
        let mut res = AbilityExecutionResult::default();
        let now = ctx.ability_ctx.tick;

        // Tick SolidBody plating & crack regeneration (`SolidBodies.tick`).
        if let Some(logia) = ctx.logia.as_deref_mut() {
            logia
                .solid_state
                .tick(SolidBodySpec::HIE_HIE, ctx.player.mastery, now);
        }

        // Passive `IceWalk.tick`: every 2 ticks when not underwater, not suppressed,
        // and neither "heat" nor "seastone" counter is active, freeze water disc into `minecraft:frosted_ice`.
        let counter_blocked = ctx.logia.as_deref().is_some_and(|l| {
            l.is_counter_active("heat", now) || l.is_counter_active("seastone", now)
        });

        if (now % ICE_WALK_EVERY_TICKS) == 0
            && !ctx.submerged_in_water
            && !ctx.suppressed
            && !counter_blocked
        {
            let centre = ctx.ability_ctx.eye_pos + DVec3::new(0.0, -2.2, 0.0);
            let cx = centre.x.floor() as i32;
            let cy = centre.y.floor() as i32;
            let cz = centre.z.floor() as i32;
            let r = ICE_WALK_RADIUS.ceil() as i32;
            let r2 = ICE_WALK_RADIUS * ICE_WALK_RADIUS;
            let mut walk_budget = CastBudget::new(ICE_WALK_BLOCKS_PER_STEP, 0);

            for ring in 0..=r {
                for dx in -ring..=ring {
                    for dz in -ring..=ring {
                        if dx.abs().max(dz.abs()) == ring {
                            let ox = (cx + dx) as f64 + 0.5 - centre.x;
                            let oz = (cz + dz) as f64 + 0.5 - centre.z;
                            if ox * ox + oz * oz <= r2 {
                                let pos = [cx + dx, cy, cz + dz];
                                if ctx.world_fx.transform_block(
                                    pos,
                                    "minecraft:water",
                                    "minecraft:frosted_ice",
                                    1.0,
                                    false,
                                    &mut walk_budget,
                                    ICE_WALK_THAW_TICKS + ring * 2,
                                    Some("minecraft:water"),
                                    now,
                                    ctx.config,
                                ) {
                                    res.transformed_blocks
                                        .push((pos, "minecraft:frosted_ice".into()));
                                }
                            }
                        }
                    }
                }
            }

            if !res.transformed_blocks.is_empty() {
                ctx.world_fx.emit_fx(
                    hie_fx::ICE_WALK,
                    0,
                    centre + DVec3::new(0.0, 1.0, 0.0),
                    DVec3::ZERO,
                    ICE_WALK_RADIUS as f32,
                    now,
                    res.transformed_blocks.len() as i32,
                    48.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
        }

        res
    }

    fn body_modifiers(
        &self,
        player: &PlayerFruitData,
        _form_runtime: &FormRuntime,
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
        // `HieEvents.friction`: Hie-Hie users walk on ice as on stone (`FRICTION_GROUND = 0.6`, `hie_ice_slip_exempt`).
        mods.friction_override = Some(FRICTION_GROUND);
        // While walking on top of water (submersion <= 1.0), Ice Walk keeps the user supported on water.
        if water_submersion <= 1.0 {
            mods.on_water_override = true;
        }
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ability::{AbilityContext, AbilityModifiers, TechniqueHeat};
    use crate::core::cold::{ColdManager, ColdState, MAX_CHILL};
    use crate::core::combat::TargetCandidate;
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::logia::{DamageCategory, LogiaData, LogiaState};
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
    fn hie_hie_abilities_and_logia_profile_match_java() {
        let module = HieHieModule::new();
        assert_eq!(module.fruit_id(), FruitId::HieHie);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::HieIce);

        let profile = module.logia_profile().expect("Hie-Hie is a Logia");
        assert_eq!(profile.element, "ice");
        assert_eq!(profile.elemental_drain_per_second, 4.0);
        assert_eq!(profile.dash_range, 24.0);
        assert_eq!(profile.dash_speed, 3.0);
        assert_eq!(profile.solid_body, Some(SolidBodySpec::HIE_HIE));
        assert!(profile.counters.iter().any(|c| c.id == "heat"));
        assert!(profile.counters.iter().any(|c| c.id == "seastone"));

        // Verify all base abilities + aliases resolve with exact Java constants.
        let ice_ball = module.ability("ice_ball").unwrap();
        assert_eq!(ice_ball.stats.energy_cost, 30.0);
        assert_eq!(ice_ball.stats.cooldown_ticks, 320);
        assert_eq!(ice_ball.stats.unlock_mastery, 3);
        assert_eq!(ice_ball.stats.damage, 2.0);
        assert_eq!(ice_ball.stats.range, 24.0);

        let partisan = module.ability("ice_block_partisan").unwrap();
        assert_eq!(partisan.id.name, "partisan");
        assert_eq!(partisan.stats.energy_cost, 28.0);
        assert_eq!(partisan.stats.cooldown_ticks, 240);
        assert_eq!(partisan.stats.unlock_mastery, 5);
        assert_eq!(partisan.stats.damage, 8.0);
        assert_eq!(partisan.stats.range, 40.0);

        let time_cap = module.ability("ice_time").unwrap();
        assert_eq!(time_cap.id.name, "ice_time_capsule");
        assert_eq!(time_cap.stats.energy_cost, 25.0);
        assert_eq!(time_cap.stats.cooldown_ticks, 280);

        let pheasant = module.ability("pheasant_beak").unwrap();
        assert_eq!(pheasant.stats.energy_cost, 36.0);
        assert_eq!(pheasant.stats.cooldown_ticks, 320);
        assert_eq!(pheasant.stats.unlock_mastery, 6);
        assert_eq!(pheasant.stats.damage, 12.0);

        let ice_age = module.ability("ice_age").unwrap();
        assert!(ice_age.is_ultimate);
        assert_eq!(ice_age.stats.energy_cost, 90.0);
        assert_eq!(ice_age.stats.cooldown_ticks, 2400);
        assert_eq!(ice_age.stats.unlock_mastery, 9);
        assert_eq!(ice_age.stats.damage, 10.0);
        assert_eq!(ice_age.stats.range, 16.0);
    }

    #[test]
    fn hie_hie_execute_all_abilities_chill_and_freeze_kinds() {
        let module = HieHieModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::HieHie),
            mastery: 10,
            energy: 280.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = vec![make_target(2, DVec3::new(0.0, 64.0, 6.0))];

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
                fruit: FruitId::HieHie,
                ability_name: "ice_ball".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 100,
                eye_pos: DVec3::new(0.0, 64.0, 0.0),
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

        // 1. Ice Ball freezes target in BLOCK for 160 ticks + adds 1 chill stack with HIE_CHILL_BURST.
        let r_ball = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "ice_ball".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_ball.hits.len(), 1);
        assert_eq!(r_ball.hits[0].freeze_ticks, ICE_BALL_FREEZE_TICKS);
        assert_eq!(r_ball.hits[0].chill_stacks_added, 1);
        assert_eq!(r_ball.hits[0].chill_burst, Some(HIE_CHILL_BURST));

        // 2. Ice Time Capsule freezes target as STATUE for 100 ticks with 0 damage.
        let r_tc = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "ice_time_capsule".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_tc.hits.len(), 1);
        assert_eq!(r_tc.hits[0].damage, 0.0);
        assert_eq!(r_tc.hits[0].freeze_ticks, TIME_CAPSULE_STATUE_TICKS);
        assert!(!r_tc.transformed_blocks.is_empty());

        // 3. Partisan throws 3 tridents.
        let r_part = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "partisan".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_part.spawned_projectiles, 3);
        assert!(!r_part.hits.is_empty());

        // 4. Pheasant Beak homes and freezes in CRYSTAL for 160 ticks.
        let r_pheasant = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "pheasant_beak".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_pheasant.hits.len(), 1);
        assert_eq!(r_pheasant.hits[0].freeze_ticks, PHEASANT_FREEZE_TICKS);

        // 5. Saber Slash step 5 fires crescent projectile.
        let r_saber = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ComboStep {
                ability: "saber_slash".into(),
                step: 5,
                is_finisher: true,
            },
        );
        assert_eq!(r_saber.spawned_projectiles, 1);
        assert!(!r_saber.hits.is_empty());

        // 6. Ice Age encases area in CRYSTAL (120 ticks) and transforms blocks.
        let r_age = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "ice_age".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_age.hits.len(), 1);
        assert_eq!(r_age.hits[0].freeze_ticks, ICE_AGE_FREEZE_TICKS);
        assert!(r_age.screen_shake.is_some());
        assert!(!r_age.transformed_blocks.is_empty());

        // Verify 4 chill stacks trigger HieCold.BURST (4.0 frost damage, 70 slow ticks, level 5).
        let mut cold_state = ColdState::NONE;
        for i in 1..MAX_CHILL {
            let out = ColdManager::add_chill(&mut cold_state, 100 + i as i64, Some(HIE_COLD_STYLE), HIE_CHILL_BURST);
            assert_eq!(out, crate::core::cold::ChillOutcome::Stacked(i));
        }
        let burst = ColdManager::add_chill(&mut cold_state, 110, Some(HIE_COLD_STYLE), HIE_CHILL_BURST);
        assert_eq!(
            burst,
            crate::core::cold::ChillOutcome::BurstTriggered {
                damage: 4.0,
                damage_type: Some(GrandLineDamageType::HieFrost),
                slow_ticks: 70,
                frostbite_amplifier: 4,
            }
        );
    }

    #[test]
    fn hie_hie_frost_walk_and_body_modifiers() {
        let module = HieHieModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::HieHie),
            mastery: 5,
            energy: 180.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData {
            state: LogiaState::Solid,
            ..LogiaData::default()
        };
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();

        // Check body modifiers: ground friction override (0.6) on ice & water walk support.
        let mods = module.body_modifiers(&player, &form_rt, false, 0.4);
        assert_eq!(mods.friction_override, Some(FRICTION_GROUND));
        assert!(mods.on_water_override);

        let suppressed_mods = module.body_modifiers(&player, &form_rt, true, 1.5);
        assert_eq!(suppressed_mods.friction_override, None);

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
                fruit: FruitId::HieHie,
                ability_name: String::new(),
                mastery: 5,
                charge: 0.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 20, // even tick triggers IceWalk
                eye_pos: DVec3::new(10.0, 65.6, 10.0),
                look_dir: DVec3::Z,
                aim_point: None,
                modifiers: AbilityModifiers::IDENTITY,
            },
            nearby_targets: &[],
            config: &cfg,
            game_rules: DFGameRules::vanilla_default(),
            creative: false,
            submerged_in_water: false,
            suppressed: false,
            on_ground: true,
            in_lava: false,
        };

        let tick_res = module.on_tick(&mut ctx);
        assert!(!tick_res.transformed_blocks.is_empty());
        assert!(
            tick_res
                .transformed_blocks
                .iter()
                .all(|(_, block)| block == "minecraft:frosted_ice")
        );

        // Verify SolidBody absorbs chip/shatter hits.
        let profile = module.logia_profile().unwrap();
        let out = LogiaManager::evaluate_damage(
            &profile,
            ctx.player,
            ctx.logia.as_deref_mut().unwrap(),
            DamageCategory::Projectile,
            4.0,
            0,
            glam::Vec3::ZERO,
            glam::Vec3::Z,
            None,
            None,
            25,
            false,
            &cfg,
            &crate::core::logia::LogiaConfig::default(),
        );
        assert!(out.passed_through);
        assert_eq!(out.final_damage, 0.0);
    }
}
