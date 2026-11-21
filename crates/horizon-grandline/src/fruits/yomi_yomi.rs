//! Yomi-Yomi no Mi (Revive-Revive Fruit) — 1:1 port from Grand Line 0.4.0 Java
//! (`com.devilfruits.fruits.yomi_yomi`).
//!
//! Includes:
//! - Second-life revival (`SoulPhase` + `SecondWind`): Upon first death, enters `"soul"` form
//!   (`soulSeconds = 180.0`, `soulHeight = 30.0`) to possess the corpse into `"skeleton"` form
//!   (`+20%` speed, `3.0` block jump height, `+2.0` safe fall distance). In `"skeleton"` form,
//!   `SecondWind` intercepts lethal damage every 20 minutes (`24000` ticks) with a 6-block
//!   `YomiHellfireCold` shockwave (`4.0` damage, 40-tick grace).
//! - Water-surface sprinting (`WaterRun`): In `"skeleton"` or `"soul_king"` form, sprinting on
//!   surface water (`waterRunSpeed = 0.65`, `waterRunEnergy = 4.0/s`) sets `on_water_override = true`
//!   in `EntityBodyModifiers` and drains `4.0 / 20.0` energy per tick.
//! - Underworld freezing sword techniques & Soul skills:
//!   - `aubade_coup_droit` (`aubade`): Piercing air-thrust line (`damage = 9.0`, `range = 8.0`, `width = 0.7`, `slowTicks = 50`).
//!   - `gavotte_bond_en_avant` (`gavotte`): Leaping forward slash (`damage = 8.0`, `range = 10.0`, `sweepRadius = 3.5`, `launch = 0.8`).
//!   - `kasuriuta_fubuki_giri` (`fubuki_giri` / `hanauta_sancho`): High-speed 5-block iai dash that
//!     freezes cut enemies solid for `60` ticks (`3s`) and freezes surface water into `minecraft:frosted_ice`.
//!   - `polka_remise` (`soul_solid`): 81-tick held 38-thrust flurry cone (`coneDegrees = 45.0`, `lockTicks = 6`).
//!   - `soul_projection`, `soul_dash`, `cold_soul` (`cold_slash` / `soul_parade`).
//!   - `soul_king` form (`unlock_mastery = 9`, `cooldown = 1800`, `timeLimit = 60s`, `freeAbilities = true`)
//!     with `king_strike`, `earth_crushing`, and `soul_king_roar` (`lullaby_parade` / `party_music`, `40%` armor-piercing `YomiSoulKingRoar` + 100-tick stun).

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::cold::ChillBurst;
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, FruitFormDef, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Grand Line 0.4.0 Java constants
// ---------------------------------------------------------------------------

/// `YomiYomi` synced form flags (`FormFlags.fruit(0..=2)`).
pub const YOMI_FLAG_POSSESSING: u32 = form_flags::fruit_bit(0);
pub const YOMI_FLAG_WATER_RUN: u32 = form_flags::fruit_bit(1);
pub const YOMI_FLAG_SECOND_WIND_GRACE: u32 = form_flags::fruit_bit(2);

/// `YomiYomi` fruit-wide settings (`YomiYomi.java`, `SecondWind.java`, `WaterRun.java`).
pub const YOMI_SKELETON_JUMP_BLOCKS: f64 = 3.0;
pub const YOMI_SOUL_SECONDS: f64 = 180.0;
pub const YOMI_SOUL_HEIGHT: f64 = 30.0;
pub const YOMI_CORPSE_HEALTH: f32 = 20.0;
pub const YOMI_POSSESS_REACH: f64 = 6.0;
pub const YOMI_WATER_RUN_SPEED: f64 = 0.65;
pub const YOMI_WATER_RUN_ENERGY_PER_SEC: f32 = 4.0;
pub const YOMI_SOUL_LEASH_BLOCKS: f64 = 64.0;
pub const YOMI_SHELL_RESISTANCE: f32 = 0.5;
pub const YOMI_SECOND_WIND_MINUTES: f64 = 20.0;
pub const YOMI_SECOND_WIND_COOLDOWN_TICKS: i32 = 24_000;
pub const YOMI_SECOND_WIND_GRACE_TICKS: i32 = 40;
pub const YOMI_SECOND_WIND_RADIUS: f64 = 6.0;
pub const YOMI_SECOND_WIND_DAMAGE: f32 = 4.0;
pub const YOMI_KING_AURA_RADIUS: f64 = 4.0;

/// `AubadeAbility` (`aubade` / `aubade_coup_droit`): energy 12, cooldown 120, mastery 1, damage 9.0, range 8.0, knockback 0.4.
pub const AUBADE_ENERGY: f32 = 12.0;
pub const AUBADE_COOLDOWN_TICKS: i32 = 120;
pub const AUBADE_UNLOCK_MASTERY: u32 = 1;
pub const AUBADE_DAMAGE: f32 = 9.0;
pub const AUBADE_RANGE: f32 = 8.0;
pub const AUBADE_KNOCKBACK: f32 = 0.4;
pub const AUBADE_SLOW_TICKS: i32 = 50;
pub const AUBADE_HALF_WIDTH: f64 = 0.7;

/// `GavotteAbility` (`gavotte` / `gavotte_bond_en_avant`): energy 16, cooldown 180, mastery 3, damage 8.0, range 10.0, knockback 1.4.
pub const GAVOTTE_ENERGY: f32 = 16.0;
pub const GAVOTTE_COOLDOWN_TICKS: i32 = 180;
pub const GAVOTTE_UNLOCK_MASTERY: u32 = 3;
pub const GAVOTTE_DAMAGE: f32 = 8.0;
pub const GAVOTTE_RANGE: f32 = 10.0;
pub const GAVOTTE_KNOCKBACK: f32 = 1.4;
pub const GAVOTTE_SWEEP_RADIUS: f64 = 3.5;
pub const GAVOTTE_LAUNCH_Y: f64 = 0.8;

/// `PolkaRemiseAbility` (`polka_remise` / `soul_solid`): energy 20, cooldown 220, mastery 4, damage 1.0 (x38 thrusts), range 5.0.
pub const POLKA_REMISE_ENERGY: f32 = 20.0;
pub const POLKA_REMISE_COOLDOWN_TICKS: i32 = 220;
pub const POLKA_REMISE_UNLOCK_MASTERY: u32 = 4;
pub const POLKA_REMISE_DAMAGE: f32 = 1.0;
pub const POLKA_REMISE_RANGE: f32 = 5.0;
pub const POLKA_REMISE_MAX_CHANNEL_TICKS: i32 = 81;
pub const POLKA_REMISE_MIN_CHANNEL_TICKS: i32 = 10;
pub const POLKA_REMISE_MAX_THRUSTS: u32 = 38;
pub const POLKA_REMISE_CONE_DEGREES: f64 = 45.0;
pub const POLKA_REMISE_SLIDE_SPEED: f64 = 1.1;
pub const POLKA_REMISE_LOCK_TICKS: i32 = 6;

/// `FubukiGiriAbility` (`fubuki_giri` / `kasuriuta_fubuki_giri` / `hanauta_sancho`): energy 24, cooldown 280, mastery 6, damage 12.0, range 5.0, freeze 60 ticks.
pub const FUBUKI_GIRI_ENERGY: f32 = 24.0;
pub const FUBUKI_GIRI_COOLDOWN_TICKS: i32 = 280;
pub const FUBUKI_GIRI_UNLOCK_MASTERY: u32 = 6;
pub const FUBUKI_GIRI_DAMAGE: f32 = 12.0;
pub const FUBUKI_GIRI_RANGE: f32 = 5.0;
pub const FUBUKI_GIRI_FREEZE_TICKS: i32 = 60;
pub const FUBUKI_GIRI_PATH_RADIUS: f64 = 1.6;
pub const FUBUKI_GIRI_WATER_FREEZE_TICKS: i32 = 160;

/// `SoulProjectionAbility`, `SoulDashAbility`, `ColdSoulAbility` (`cold_slash` / `soul_parade`).
pub const SOUL_PROJECTION_ENERGY: f32 = 20.0;
pub const SOUL_PROJECTION_COOLDOWN_TICKS: i32 = 600;
pub const SOUL_PROJECTION_UNLOCK_MASTERY: u32 = 1;

pub const SOUL_DASH_ENERGY: f32 = 10.0;
pub const SOUL_DASH_COOLDOWN_TICKS: i32 = 60;
pub const SOUL_DASH_UNLOCK_MASTERY: u32 = 1;
pub const SOUL_DASH_DAMAGE: f32 = 6.0;
pub const SOUL_DASH_RANGE: f32 = 15.0;
pub const SOUL_DASH_KNOCKBACK: f32 = 0.3;
pub const SOUL_DASH_FROSTBITE_TICKS: i32 = 40;

pub const COLD_SOUL_ENERGY: f32 = 16.0;
pub const COLD_SOUL_COOLDOWN_TICKS: i32 = 160;
pub const COLD_SOUL_UNLOCK_MASTERY: u32 = 1;
pub const COLD_SOUL_DAMAGE: f32 = 2.5;
pub const COLD_SOUL_RANGE: f32 = 8.0;
pub const COLD_SOUL_SLOW_TICKS: i32 = 100;
pub const COLD_SOUL_SLOW_LEVEL: i32 = 6;
pub const COLD_SOUL_PULSES: [i32; 3] = [11, 19, 27];

/// `SoulKing` form & abilities (`king_strike`, `earth_crushing`, `soul_king_roar` / `lullaby_parade` / `party_music`).
pub const SOUL_KING_UNLOCK_MASTERY: u32 = 9;
pub const SOUL_KING_COOLDOWN_TICKS: i32 = 1800;
pub const SOUL_KING_CAST_TICKS: i32 = 30;
pub const SOUL_KING_TIME_LIMIT_SECONDS: f32 = 60.0;

pub const KING_STRIKE_COOLDOWN_TICKS: i32 = 16;
pub const KING_STRIKE_DAMAGE: f32 = 9.0;
pub const KING_STRIKE_RANGE: f32 = 3.0;
pub const KING_STRIKE_KNOCKBACK: f32 = 1.1;
pub const KING_STRIKE_PIN_TICKS: i32 = 16;
pub const KING_STRIKE_PUSH_SPEED: f64 = 0.75;

pub const EARTH_CRUSHING_COOLDOWN_TICKS: i32 = 240;
pub const EARTH_CRUSHING_DAMAGE: f32 = 18.0;
pub const EARTH_CRUSHING_RANGE: f32 = 6.6;
pub const EARTH_CRUSHING_STUN_TICKS: i32 = 60;
pub const EARTH_CRUSHING_PUSH: f64 = 0.8;
pub const EARTH_CRUSHING_LAUNCH: f64 = 0.55;

pub const SOUL_KING_ROAR_COOLDOWN_TICKS: i32 = 500;
pub const SOUL_KING_ROAR_DAMAGE: f32 = 16.0;
pub const SOUL_KING_ROAR_RANGE: f32 = 9.3;
pub const SOUL_KING_ROAR_STUN_TICKS: i32 = 100;
pub const SOUL_KING_ROAR_PIERCE_SHARE: f32 = 0.4;

/// Canonical Yomi-Yomi VFX identifiers (`YomiFx.java`).
pub mod yomi_fx {
    pub const SOUL_DEPART: &str = "devilfruits:yomi_yomi/soul_depart";
    pub const SOUL_RISE: &str = "devilfruits:yomi_yomi/soul_rise";
    pub const POSSESS: &str = "devilfruits:yomi_yomi/possess";
    pub const AUBADE: &str = "devilfruits:yomi_yomi/aubade";
    pub const GAVOTTE_LEAP: &str = "devilfruits:yomi_yomi/gavotte_leap";
    pub const GAVOTTE_SWEEP: &str = "devilfruits:yomi_yomi/gavotte_sweep";
    pub const FUBUKI_DASH: &str = "devilfruits:yomi_yomi/fubuki_dash";
    pub const FUBUKI_BURST: &str = "devilfruits:yomi_yomi/fubuki_burst";
    pub const POLKA: &str = "devilfruits:yomi_yomi/polka";
    pub const POLKA_HIT: &str = "devilfruits:yomi_yomi/polka_hit";
    pub const POLKA_END: &str = "devilfruits:yomi_yomi/polka_end";
    pub const CHILL_BURST: &str = "devilfruits:yomi_yomi/chill_burst";
    pub const FREEZE: &str = "devilfruits:yomi_yomi/freeze";
    pub const PROJECT: &str = "devilfruits:yomi_yomi/project";
    pub const SOUL_DASH: &str = "devilfruits:yomi_yomi/soul_dash";
    pub const SOUL_DASH_SURGE: &str = "devilfruits:yomi_yomi/soul_dash_surge";
    pub const SOUL_CHILL: &str = "devilfruits:yomi_yomi/soul_chill";
    pub const COLD_SOUL: &str = "devilfruits:yomi_yomi/cold_soul";
    pub const COLD_SOUL_PULSE: &str = "devilfruits:yomi_yomi/cold_soul_pulse";
    pub const SECOND_WIND: &str = "devilfruits:yomi_yomi/second_wind";
    pub const KING_SUMMON: &str = "devilfruits:yomi_yomi/king_summon";
    pub const KING_VANISH: &str = "devilfruits:yomi_yomi/king_vanish";
    pub const KING_STRIKE: &str = "devilfruits:yomi_yomi/king_strike";
    pub const KING_HIT: &str = "devilfruits:yomi_yomi/king_hit";
    pub const KING_CRUSH: &str = "devilfruits:yomi_yomi/king_crush";
    pub const KING_ROAR: &str = "devilfruits:yomi_yomi/king_roar";
    pub const KING_ROAR_STUN: &str = "devilfruits:yomi_yomi/king_roar_stun";
}

/// 1:1 port of `GavotteAbility.leap(look, range)`.
pub fn gavotte_leap_velocity(look: DVec3, range: f64) -> DVec3 {
    let flat_raw = DVec3::new(look.x, 0.0, look.z);
    let flat = if flat_raw.length_squared() < 1e-4 {
        DVec3::Z
    } else {
        flat_raw.normalize()
    };
    let up = look.y.clamp(0.0, 0.8);
    let reach = (1.0 - 0.91_f64.powf(19.0)) / 0.09;
    let forward = (range / reach) * (1.0 - up * 0.45);
    flat * forward + DVec3::new(0.0, 0.66 + up * 0.45, 0.0)
}

/// 1:1 port of `SecondWind.rise`: checks whether the player can trigger Second Wind skeleton revive,
/// sets `"second_wind"` cooldown (`24000` ticks), arms `YOMI_FLAG_SECOND_WIND_GRACE`, and returns
/// the cold-flame shockwave hits (`4.0` `YomiHellfireCold` damage within 6.0 blocks).
pub fn trigger_second_wind(ctx: &mut FruitContext<'_>) -> Option<AbilityExecutionResult> {
    if ctx.suppressed {
        return None;
    }
    let is_skeleton_vessel = matches!(
        ctx.player.active_form.as_str(),
        "skeleton" | "soul_king" | "base"
    );
    if !is_skeleton_vessel
        || ctx
            .player
            .cooldown_remaining("second_wind", ctx.ability_ctx.tick)
            > 0
    {
        return None;
    }

    ctx.player.set_cooldown(
        "second_wind",
        ctx.ability_ctx.tick,
        YOMI_SECOND_WIND_COOLDOWN_TICKS,
    );
    ctx.player.active_form = "skeleton".to_string();
    ctx.form_runtime.set_flag(YOMI_FLAG_SECOND_WIND_GRACE, true);

    let mut res = AbilityExecutionResult {
        cast_result: Some(CastResult::Success),
        damage_type: Some(GrandLineDamageType::YomiHellfireCold),
        form_switch_to: Some("skeleton".to_string()),
        ..AbilityExecutionResult::default()
    };

    let center = ctx.ability_ctx.eye_pos;
    let burst = ChillBurst {
        damage: 8.0,
        damage_type: Some(GrandLineDamageType::YomiHellfireCold),
        slow_ticks: 70,
        frostbite_level: 5,
    };
    for t in CombatTargeting::in_radius(
        ctx.nearby_targets,
        ctx.player_id,
        center,
        YOMI_SECOND_WIND_RADIUS,
    ) {
        let away_raw = DVec3::new(t.center.x - center.x, 0.0, t.center.z - center.z);
        let away = if away_raw.length_squared() < 1e-4 {
            DVec3::Z
        } else {
            away_raw.normalize()
        };
        res.hits.push(AbilityHitEffect {
            target_id: t.id,
            damage: YOMI_SECOND_WIND_DAMAGE,
            damage_type: GrandLineDamageType::YomiHellfireCold,
            knockback_impulse: away * 1.6 + DVec3::new(0.0, 0.45, 0.0),
            stun_ticks: 0,
            chill_stacks_added: 1,
            chill_burst: Some(burst),
            freeze_ticks: 0,
            set_fire_seconds: 0,
        });
    }

    ctx.world_fx.emit_fx(
        yomi_fx::SECOND_WIND,
        0,
        center,
        DVec3::ZERO,
        YOMI_SECOND_WIND_RADIUS as f32,
        ctx.ability_ctx.tick,
        YOMI_SECOND_WIND_GRACE_TICKS,
        64.0,
    );
    res.vfx_events = ctx.world_fx.drain_vfx();
    Some(res)
}

/// Yomi-Yomi no Mi module (`YomiYomi.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct YomiYomiModule;

impl YomiYomiModule {
    pub const fn new() -> Self {
        Self
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
            id: AbilityId::new(FruitId::YomiYomi, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms,
            stats,
            is_ultimate,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        }
    }

    fn all_forms() -> Vec<String> {
        vec![
            "base".to_string(),
            "human".to_string(),
            "corpse".to_string(),
            "skeleton".to_string(),
            "soul".to_string(),
            "soul_king".to_string(),
        ]
    }

    fn aubade_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: AUBADE_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: AUBADE_COOLDOWN_TICKS,
            unlock_mastery: AUBADE_UNLOCK_MASTERY,
            damage: AUBADE_DAMAGE,
            range: AUBADE_RANGE,
            knockback: AUBADE_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn gavotte_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: GAVOTTE_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: GAVOTTE_COOLDOWN_TICKS,
            unlock_mastery: GAVOTTE_UNLOCK_MASTERY,
            damage: GAVOTTE_DAMAGE,
            range: GAVOTTE_RANGE,
            knockback: GAVOTTE_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn polka_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: POLKA_REMISE_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: POLKA_REMISE_COOLDOWN_TICKS,
            unlock_mastery: POLKA_REMISE_UNLOCK_MASTERY,
            damage: POLKA_REMISE_DAMAGE,
            range: POLKA_REMISE_RANGE,
            knockback: 0.0,
            max_channel_ticks: POLKA_REMISE_MAX_CHANNEL_TICKS,
            min_channel_ticks: POLKA_REMISE_MIN_CHANNEL_TICKS,
            ..AbilityStats::default()
        }
    }

    fn fubuki_giri_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: FUBUKI_GIRI_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: FUBUKI_GIRI_COOLDOWN_TICKS,
            unlock_mastery: FUBUKI_GIRI_UNLOCK_MASTERY,
            damage: FUBUKI_GIRI_DAMAGE,
            range: FUBUKI_GIRI_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn soul_projection_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SOUL_PROJECTION_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SOUL_PROJECTION_COOLDOWN_TICKS,
            unlock_mastery: SOUL_PROJECTION_UNLOCK_MASTERY,
            damage: 0.0,
            range: 1.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn soul_dash_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SOUL_DASH_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SOUL_DASH_COOLDOWN_TICKS,
            unlock_mastery: SOUL_DASH_UNLOCK_MASTERY,
            damage: SOUL_DASH_DAMAGE,
            range: SOUL_DASH_RANGE,
            knockback: SOUL_DASH_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn cold_soul_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: COLD_SOUL_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: COLD_SOUL_COOLDOWN_TICKS,
            unlock_mastery: COLD_SOUL_UNLOCK_MASTERY,
            damage: COLD_SOUL_DAMAGE,
            range: COLD_SOUL_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn soul_king_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 0.0,
            energy_per_second: 0.0,
            cooldown_ticks: SOUL_KING_COOLDOWN_TICKS,
            unlock_mastery: SOUL_KING_UNLOCK_MASTERY,
            damage: 0.0,
            range: 1.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn king_strike_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 0.0,
            energy_per_second: 0.0,
            cooldown_ticks: KING_STRIKE_COOLDOWN_TICKS,
            unlock_mastery: SOUL_KING_UNLOCK_MASTERY,
            damage: KING_STRIKE_DAMAGE,
            range: KING_STRIKE_RANGE,
            knockback: KING_STRIKE_KNOCKBACK,
            max_combo_steps: 3,
            combo_window_ticks: 34,
            ..AbilityStats::default()
        }
    }

    fn earth_crushing_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 0.0,
            energy_per_second: 0.0,
            cooldown_ticks: EARTH_CRUSHING_COOLDOWN_TICKS,
            unlock_mastery: SOUL_KING_UNLOCK_MASTERY,
            damage: EARTH_CRUSHING_DAMAGE,
            range: EARTH_CRUSHING_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn soul_king_roar_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: 0.0,
            energy_per_second: 0.0,
            cooldown_ticks: SOUL_KING_ROAR_COOLDOWN_TICKS,
            unlock_mastery: SOUL_KING_UNLOCK_MASTERY,
            damage: SOUL_KING_ROAR_DAMAGE,
            range: SOUL_KING_ROAR_RANGE,
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
            "aubade_coup_droit" => "aubade",
            "gavotte_bond_en_avant" => "gavotte",
            "hanauta_sancho" | "kasuriuta_fubuki_giri" => "fubuki_giri",
            "soul_solid" => "polka_remise",
            "cold_slash" | "soul_parade" => "cold_soul",
            "lullaby_parade" | "party_music" => "soul_king_roar",
            other => other,
        }
    }

    fn chill_burst() -> ChillBurst {
        ChillBurst {
            damage: 8.0,
            damage_type: Some(GrandLineDamageType::YomiHellfireCold),
            slow_ticks: 70,
            frostbite_level: 5,
        }
    }

    fn exec_aubade(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::aubade_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let from = eye + DVec3::new(0.0, -0.35, 0.0) + look * 0.6;
        let to = from + look * range;
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();

        res.animation_clip = Some("devilfruits:yomi_yomi/aubade".to_string());
        res.caster_velocity_impulse = Some(flat * 0.45);
        let push = look * (AUBADE_KNOCKBACK as f64) + DVec3::new(0.0, 0.12, 0.0);

        for t in CombatTargeting::in_capsule(
            ctx.nearby_targets,
            ctx.player_id,
            from,
            to,
            AUBADE_HALF_WIDTH,
        ) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiSoul,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 1,
                chill_burst: Some(Self::chill_burst()),
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            yomi_fx::AUBADE,
            0,
            from,
            to,
            AUBADE_HALF_WIDTH as f32,
            ctx.ability_ctx.tick,
            res.hits.len() as i32,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_gavotte(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::gavotte_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let leap_v = gavotte_leap_velocity(look, range);

        res.animation_clip = Some("devilfruits:yomi_yomi/gavotte".to_string());
        res.caster_velocity_impulse = Some(leap_v);

        let landing_center = eye + DVec3::new(look.x, 0.0, look.z).normalize_or_zero() * (range * 0.5);
        for t in CombatTargeting::in_radius(
            ctx.nearby_targets,
            ctx.player_id,
            landing_center,
            GAVOTTE_SWEEP_RADIUS + 2.5,
        ) {
            let away_raw = DVec3::new(t.center.x - landing_center.x, 0.0, t.center.z - landing_center.z);
            let away = if away_raw.length_squared() < 1e-4 {
                DVec3::Z
            } else {
                away_raw.normalize()
            };
            let push = away * ((GAVOTTE_KNOCKBACK as f64) * 0.6) + DVec3::new(0.0, GAVOTTE_LAUNCH_Y, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiSoul,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 1,
                chill_burst: Some(Self::chill_burst()),
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            yomi_fx::GAVOTTE_LEAP,
            0,
            eye,
            leap_v,
            1.0,
            ctx.ability_ctx.tick,
            0,
            64.0,
        );
        ctx.world_fx.emit_fx(
            yomi_fx::GAVOTTE_SWEEP,
            0,
            landing_center,
            leap_v,
            GAVOTTE_SWEEP_RADIUS as f32,
            ctx.ability_ctx.tick,
            res.hits.len() as i32,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_fubuki_giri(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::fubuki_giri_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let dir = if flat.length_squared() < 1e-4 {
            DVec3::Z
        } else {
            flat
        };
        let a = ctx.ability_ctx.eye_pos;
        let b = a + dir * (FUBUKI_GIRI_RANGE as f64);
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        res.animation_clip = Some("devilfruits:yomi_yomi/fubuki_giri".to_string());
        res.caster_velocity_impulse = Some(b - a);

        for t in CombatTargeting::in_capsule(
            ctx.nearby_targets,
            ctx.player_id,
            a,
            b,
            FUBUKI_GIRI_PATH_RADIUS,
        ) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiHellfireCold,
                knockback_impulse: DVec3::ZERO,
                stun_ticks: FUBUKI_GIRI_FREEZE_TICKS,
                chill_stacks_added: 1,
                chill_burst: Some(Self::chill_burst()),
                freeze_ticks: FUBUKI_GIRI_FREEZE_TICKS,
                set_fire_seconds: 0,
            });
            ctx.world_fx.emit_fx(
                yomi_fx::FREEZE,
                0,
                t.center,
                DVec3::ZERO,
                t.height as f32,
                ctx.ability_ctx.tick,
                FUBUKI_GIRI_FREEZE_TICKS,
                64.0,
            );
        }

        // Freeze surface water along the path into frosted_ice (`FubukiGiriAbility.freezeWater`)
        let restore_at = ctx.ability_ctx.tick + FUBUKI_GIRI_WATER_FREEZE_TICKS as i64;
        for step in 0..=5 {
            let p = a.lerp(b, (step as f64) / 5.0);
            let pos = [p.x.floor() as i32, p.y.floor() as i32 - 1, p.z.floor() as i32];
            ctx.world_fx.schedule_restore(
                pos,
                "minecraft:water",
                Some("minecraft:frosted_ice"),
                restore_at,
            );
            res.transformed_blocks
                .push((pos, "minecraft:frosted_ice".to_string()));
        }

        ctx.world_fx.emit_fx(
            yomi_fx::FUBUKI_DASH,
            0,
            a,
            b,
            1.0,
            ctx.ability_ctx.tick,
            4,
            64.0,
        );
        ctx.world_fx.emit_fx(
            yomi_fx::FUBUKI_BURST,
            0,
            a,
            b,
            FUBUKI_GIRI_PATH_RADIUS as f32,
            ctx.ability_ctx.tick,
            0,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_polka_remise(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::polka_remise_stats_ref();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let per_thrust_dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let half = (POLKA_REMISE_CONE_DEGREES * 0.5).to_radians();

        match event {
            AbilityEvent::ChannelStarted { .. } => {
                res.animation_clip = Some("devilfruits:yomi_yomi/polka_remise".to_string());
                res.caster_velocity_impulse = Some(flat * POLKA_REMISE_SLIDE_SPEED);
                ctx.world_fx.emit_fx(
                    yomi_fx::POLKA,
                    0,
                    eye,
                    flat,
                    POLKA_REMISE_CONE_DEGREES as f32,
                    ctx.ability_ctx.tick,
                    POLKA_REMISE_MAX_THRUSTS as i32,
                    64.0,
                );
            }
            AbilityEvent::ChannelTick { elapsed_ticks, .. } => {
                if *elapsed_ticks >= 4 && (*elapsed_ticks % 2) == 0 {
                    for t in CombatTargeting::in_cone(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        look,
                        range,
                        half,
                    ) {
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: per_thrust_dmg,
                            damage_type: GrandLineDamageType::YomiSoul,
                            knockback_impulse: DVec3::ZERO,
                            stun_ticks: POLKA_REMISE_LOCK_TICKS,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                        ctx.world_fx.emit_fx(
                            yomi_fx::POLKA_HIT,
                            0,
                            t.center,
                            look,
                            per_thrust_dmg,
                            ctx.ability_ctx.tick,
                            0,
                            64.0,
                        );
                    }
                }
            }
            AbilityEvent::CastInstant { .. } => {
                res.animation_clip = Some("devilfruits:yomi_yomi/polka_remise".to_string());
                res.caster_velocity_impulse = Some(flat * POLKA_REMISE_SLIDE_SPEED);
                for t in CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    range,
                    half,
                ) {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: per_thrust_dmg * 8.0,
                        damage_type: GrandLineDamageType::YomiSoul,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: POLKA_REMISE_LOCK_TICKS,
                        chill_stacks_added: 1,
                        chill_burst: Some(Self::chill_burst()),
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_fx(
                    yomi_fx::POLKA,
                    0,
                    eye,
                    flat,
                    POLKA_REMISE_CONE_DEGREES as f32,
                    ctx.ability_ctx.tick,
                    POLKA_REMISE_MAX_THRUSTS as i32,
                    64.0,
                );
            }
            AbilityEvent::ChannelEnded { .. } => {
                res.animation_clip = Some("devilfruits:yomi_yomi/polka_finish".to_string());
                ctx.world_fx.emit_fx(
                    yomi_fx::POLKA_END,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn polka_remise_stats_ref() -> AbilityStats {
        Self::polka_stats()
    }

    fn exec_soul_projection(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let entering = match event {
            AbilityEvent::Toggled { enabled, .. } => *enabled,
            _ => ctx.player.active_form != "soul",
        };
        if entering {
            ctx.player.active_form = "soul".to_string();
            ctx.form_runtime.set_flag(form_flags::FLYING, true);
            res.form_switch_to = Some("soul".to_string());
            ctx.world_fx.emit_fx(
                yomi_fx::PROJECT,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        } else {
            ctx.player.active_form = "skeleton".to_string();
            ctx.form_runtime.set_flag(form_flags::FLYING, false);
            ctx.form_runtime.safe_landing.arm();
            res.form_switch_to = Some("skeleton".to_string());
            ctx.world_fx.emit_fx(
                yomi_fx::POSSESS,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_soul_dash(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::soul_dash_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let end = eye + look * range;

        res.caster_velocity_impulse = Some(look * range);
        for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, eye, end, 1.1) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiHellfireCold,
                knockback_impulse: look * (SOUL_DASH_KNOCKBACK as f64),
                stun_ticks: 0,
                chill_stacks_added: 1,
                chill_burst: Some(Self::chill_burst()),
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
            ctx.world_fx.emit_fx(
                yomi_fx::SOUL_CHILL,
                0,
                t.center,
                DVec3::ZERO,
                t.height as f32,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }
        ctx.world_fx.emit_fx(
            yomi_fx::SOUL_DASH_SURGE,
            0,
            eye,
            end,
            0.0,
            ctx.ability_ctx.tick,
            3,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_cold_soul(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::cold_soul_stats();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg * 3.0,
                damage_type: GrandLineDamageType::YomiHellfireCold,
                knockback_impulse: DVec3::ZERO,
                stun_ticks: 0,
                chill_stacks_added: 3,
                chill_burst: Some(Self::chill_burst()),
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            yomi_fx::COLD_SOUL,
            0,
            eye,
            DVec3::ZERO,
            range as f32,
            ctx.ability_ctx.tick,
            COLD_SOUL_PULSES[0],
            64.0,
        );
        ctx.world_fx.emit_fx(
            yomi_fx::COLD_SOUL_PULSE,
            0,
            eye,
            DVec3::ZERO,
            range as f32,
            ctx.ability_ctx.tick,
            res.hits.len() as i32,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_soul_king(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let entering = ctx.player.active_form != "soul_king";
        if entering {
            ctx.player.active_form = "soul_king".to_string();
            ctx.form_runtime.time_limit_end_tick =
                Some(ctx.ability_ctx.tick + (SOUL_KING_TIME_LIMIT_SECONDS * 20.0) as i64);
            res.form_switch_to = Some("soul_king".to_string());
            res.animation_clip = Some("devilfruits:yomi_yomi/guitar_riff".to_string());
            ctx.world_fx.emit_fx(
                yomi_fx::KING_SUMMON,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                40,
                96.0,
            );
            ctx.world_fx.emit_shake(ctx.ability_ctx.eye_pos, 0.8, 28);
            res.screen_shake = Some((0.8, 28));
        } else {
            ctx.player.active_form = "skeleton".to_string();
            ctx.form_runtime.time_limit_end_tick = None;
            res.form_switch_to = Some("skeleton".to_string());
            ctx.world_fx.emit_fx(
                yomi_fx::KING_VANISH,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_king_strike(
        &self,
        ctx: &mut FruitContext<'_>,
        step: u32,
        is_finisher: bool,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::king_strike_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let dir = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let right = DVec3::new(-dir.z, 0.0, dir.x);
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let base_dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let dmg = if is_finisher || step >= 3 {
            base_dmg * 2.0
        } else {
            base_dmg
        };
        let half_deg: f64 = match step {
            1 => 85.0,
            2 => 55.0,
            _ => 42.0,
        };

        for t in CombatTargeting::in_cone(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            dir,
            range,
            half_deg.to_radians(),
        ) {
            let out = (t.center - eye).normalize_or_zero();
            let (fling, stun) = match step {
                1 => (
                    (right * 0.85 + out * 0.45).normalize_or_zero()
                        * (KING_STRIKE_KNOCKBACK as f64)
                        + DVec3::new(0.0, 0.35, 0.0),
                    0,
                ),
                2 => (DVec3::new(0.0, -1.2, 0.0), KING_STRIKE_PIN_TICKS),
                _ => (
                    out * KING_STRIKE_PUSH_SPEED + DVec3::new(0.0, 0.42, 0.0),
                    0,
                ),
            };
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiSoul,
                knockback_impulse: fling,
                stun_ticks: stun,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
            ctx.world_fx.emit_fx(
                yomi_fx::KING_HIT,
                0,
                t.center,
                fling,
                dmg,
                ctx.ability_ctx.tick,
                step as i32,
                64.0,
            );
        }
        ctx.world_fx.emit_fx(
            yomi_fx::KING_STRIKE,
            0,
            eye,
            dir,
            range as f32,
            ctx.ability_ctx.tick,
            step as i32,
            64.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_earth_crushing(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::earth_crushing_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let dir = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let apex = ctx.ability_ctx.eye_pos + dir * 1.5;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, apex, range) {
            let out_raw = DVec3::new(t.center.x - apex.x, 0.0, t.center.z - apex.z);
            let out = if out_raw.length_squared() < 1e-4 {
                dir
            } else {
                out_raw.normalize()
            };
            let push = out * EARTH_CRUSHING_PUSH + DVec3::new(0.0, EARTH_CRUSHING_LAUNCH, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiSoul,
                knockback_impulse: push,
                stun_ticks: EARTH_CRUSHING_STUN_TICKS,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            yomi_fx::KING_CRUSH,
            0,
            apex,
            dir,
            range as f32,
            ctx.ability_ctx.tick,
            0,
            96.0,
        );
        ctx.world_fx.emit_shake(apex, 0.85, 32);
        res.screen_shake = Some((0.85, 32));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_soul_king_roar(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::soul_king_roar_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let dir = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let center = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);

        res.damage_type = Some(GrandLineDamageType::YomiSoulKingRoar);
        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, center, range) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::YomiSoulKingRoar,
                knockback_impulse: DVec3::ZERO,
                stun_ticks: SOUL_KING_ROAR_STUN_TICKS,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
            ctx.world_fx.emit_fx(
                yomi_fx::KING_ROAR_STUN,
                0,
                t.center,
                DVec3::ZERO,
                SOUL_KING_ROAR_STUN_TICKS as f32,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }

        ctx.world_fx.emit_fx(
            yomi_fx::KING_ROAR,
            0,
            center,
            dir,
            range as f32,
            ctx.ability_ctx.tick,
            0,
            96.0,
        );
        ctx.world_fx.emit_shake(center, 0.9, 36);
        res.screen_shake = Some((0.9, 36));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }
}

impl DevilFruitModule for YomiYomiModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::YomiYomi
    }

    fn forms(&self) -> Vec<FruitFormDef> {
        let mut forms = FruitFormDef::builtin_forms(FruitId::YomiYomi);
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
                "soul_projection",
                "Soul Projection",
                CastType::Toggle,
                all.clone(),
                Self::soul_projection_stats(),
                false,
            ),
            Self::make_ability(
                "cold_slash",
                "Cold Soul (Soul Parade)",
                CastType::Instant,
                all.clone(),
                Self::cold_soul_stats(),
                false,
            ),
            Self::make_ability(
                "hanauta_sancho",
                "Hanauta Sancho: Yahazu Giri",
                CastType::Instant,
                all.clone(),
                Self::fubuki_giri_stats(),
                false,
            ),
            Self::make_ability(
                "party_music",
                "Party Music (Lullaby Parade / Soul King Roar)",
                CastType::Instant,
                all.clone(),
                Self::soul_king_roar_stats(),
                false,
            ),
            Self::make_ability(
                "soul_king",
                "Soul King",
                CastType::FormSwitch,
                all.clone(),
                Self::soul_king_stats(),
                true,
            ),
            Self::make_ability(
                "aubade",
                "Aubade Coup Droit",
                CastType::Instant,
                all.clone(),
                Self::aubade_stats(),
                false,
            ),
            Self::make_ability(
                "gavotte",
                "Gavotte Bond en Avant",
                CastType::Instant,
                all.clone(),
                Self::gavotte_stats(),
                false,
            ),
            Self::make_ability(
                "fubuki_giri",
                "Kasuriuta: Fubuki Giri",
                CastType::Instant,
                all.clone(),
                Self::fubuki_giri_stats(),
                false,
            ),
            Self::make_ability(
                "polka_remise",
                "Polka Remise (Soul Solid)",
                CastType::Held,
                all.clone(),
                Self::polka_stats(),
                false,
            ),
            Self::make_ability(
                "soul_dash",
                "Soul Dash",
                CastType::Instant,
                all.clone(),
                Self::soul_dash_stats(),
                false,
            ),
            Self::make_ability(
                "cold_soul",
                "Cold Soul",
                CastType::Instant,
                all.clone(),
                Self::cold_soul_stats(),
                false,
            ),
            Self::make_ability(
                "king_strike",
                "Ghost Strike",
                CastType::Combo,
                all.clone(),
                Self::king_strike_stats(),
                false,
            ),
            Self::make_ability(
                "earth_crushing",
                "Earth Crushing Strike",
                CastType::Instant,
                all.clone(),
                Self::earth_crushing_stats(),
                false,
            ),
            Self::make_ability(
                "soul_king_roar",
                "Soul King Roar",
                CastType::Instant,
                all.clone(),
                Self::soul_king_roar_stats(),
                false,
            ),
            // Spec aliases
            Self::make_ability(
                "aubade_coup_droit",
                "Aubade Coup Droit",
                CastType::Instant,
                all.clone(),
                Self::aubade_stats(),
                false,
            ),
            Self::make_ability(
                "gavotte_bond_en_avant",
                "Gavotte Bond en Avant",
                CastType::Instant,
                all.clone(),
                Self::gavotte_stats(),
                false,
            ),
            Self::make_ability(
                "lullaby_parade",
                "Lullaby Parade",
                CastType::Instant,
                all.clone(),
                Self::soul_king_roar_stats(),
                false,
            ),
            Self::make_ability(
                "soul_solid",
                "Soul Solid",
                CastType::Held,
                all.clone(),
                Self::polka_stats(),
                false,
            ),
            Self::make_ability(
                "kasuriuta_fubuki_giri",
                "Kasuriuta: Fubuki Giri",
                CastType::Instant,
                all.clone(),
                Self::fubuki_giri_stats(),
                false,
            ),
            Self::make_ability(
                "soul_parade",
                "Soul Parade",
                CastType::Instant,
                all,
                Self::cold_soul_stats(),
                false,
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
            damage_type: Some(GrandLineDamageType::YomiSoul),
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
            "aubade" => {
                self.exec_aubade(ctx, &mut res);
            }
            "gavotte" => {
                self.exec_gavotte(ctx, &mut res);
            }
            "fubuki_giri" => {
                res.damage_type = Some(GrandLineDamageType::YomiHellfireCold);
                self.exec_fubuki_giri(ctx, &mut res);
            }
            "polka_remise" => {
                if matches!(
                    event,
                    AbilityEvent::ChannelStarted { .. } | AbilityEvent::ChannelTick { .. }
                ) {
                    res.cast_result = Some(CastResult::Channelling);
                }
                self.exec_polka_remise(ctx, event, &mut res);
            }
            "soul_projection" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_soul_projection(ctx, event, &mut res);
            }
            "soul_dash" => {
                res.damage_type = Some(GrandLineDamageType::YomiHellfireCold);
                self.exec_soul_dash(ctx, &mut res);
            }
            "cold_soul" => {
                res.damage_type = Some(GrandLineDamageType::YomiHellfireCold);
                self.exec_cold_soul(ctx, &mut res);
            }
            "soul_king" => {
                self.exec_soul_king(ctx, &mut res);
            }
            "king_strike" => match event {
                AbilityEvent::ComboStep {
                    step, is_finisher, ..
                } => {
                    res.cast_result = Some(CastResult::ComboStep);
                    self.exec_king_strike(ctx, *step, *is_finisher, &mut res);
                }
                AbilityEvent::CastInstant { .. } => {
                    let step = ctx.ability_ctx.combo_step.max(1);
                    self.exec_king_strike(ctx, step, step >= 3, &mut res);
                }
                _ => {}
            },
            "earth_crushing" => {
                self.exec_earth_crushing(ctx, &mut res);
            }
            "soul_king_roar" => {
                self.exec_soul_king_roar(ctx, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed {
            ctx.form_runtime.set_flag(YOMI_FLAG_WATER_RUN, false);
            return res;
        }

        // Water-surface sprinting in Skeleton or Soul King form (`WaterRun.tick`)
        let can_water_run = matches!(ctx.player.active_form.as_str(), "skeleton" | "soul_king");
        if can_water_run && ctx.form_runtime.has_flag(YOMI_FLAG_WATER_RUN) && !ctx.submerged_in_water
        {
            let drain = YOMI_WATER_RUN_ENERGY_PER_SEC / 20.0;
            if !ctx.player.spend_energy(drain, ctx.creative, ctx.config) {
                ctx.form_runtime.set_flag(YOMI_FLAG_WATER_RUN, false);
            }
        } else if !can_water_run || ctx.submerged_in_water {
            ctx.form_runtime.set_flag(YOMI_FLAG_WATER_RUN, false);
        }

        // Soul King 60-second time limit expiration
        if ctx.player.active_form == "soul_king" {
            if let Some(end_tick) = ctx.form_runtime.time_limit_end_tick {
                if ctx.ability_ctx.tick >= end_tick {
                    ctx.player.active_form = "skeleton".to_string();
                    ctx.form_runtime.time_limit_end_tick = None;
                    res.form_switch_to = Some("skeleton".to_string());
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
        let skeleton_or_king = matches!(player.active_form.as_str(), "skeleton" | "soul_king");
        if skeleton_or_king
            && (form_runtime.has_flag(YOMI_FLAG_WATER_RUN)
                || (water_submersion > 0.0 && water_submersion <= 0.6 && player.energy > 0.0))
        {
            mods.on_water_override = true;
        }
        if player.active_form == "soul" {
            mods.passes_through_leaves = true;
            mods.reset_above_ground_ticks = true;
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
    fn yomi_abilities_and_aliases_match_java_stats() {
        let m = YomiYomiModule::new();
        assert_eq!(m.fruit_id(), FruitId::YomiYomi);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::YomiSoul);

        let aubade = m.ability("aubade_coup_droit").unwrap();
        assert_eq!(aubade.stats.energy_cost, 12.0);
        assert_eq!(aubade.stats.cooldown_ticks, 120);
        assert_eq!(aubade.stats.damage, 9.0);
        assert_eq!(aubade.stats.range, 8.0);

        let gavotte = m.ability("gavotte_bond_en_avant").unwrap();
        assert_eq!(gavotte.stats.energy_cost, 16.0);
        assert_eq!(gavotte.stats.cooldown_ticks, 180);
        assert_eq!(gavotte.stats.damage, 8.0);
        assert_eq!(gavotte.stats.range, 10.0);

        let fubuki = m.ability("kasuriuta_fubuki_giri").unwrap();
        assert_eq!(fubuki.stats.energy_cost, 24.0);
        assert_eq!(fubuki.stats.cooldown_ticks, 280);
        assert_eq!(fubuki.stats.unlock_mastery, 6);
        assert_eq!(fubuki.stats.damage, 12.0);

        let solid = m.ability("soul_solid").unwrap();
        assert_eq!(solid.cast_type, CastType::Held);
        assert_eq!(solid.stats.energy_cost, 20.0);
        assert_eq!(solid.stats.max_channel_ticks, 81);

        let lullaby = m.ability("lullaby_parade").unwrap();
        assert_eq!(lullaby.stats.cooldown_ticks, 500);
        assert_eq!(lullaby.stats.unlock_mastery, 9);
        assert_eq!(lullaby.stats.damage, 16.0);

        let king = m.ability("soul_king").unwrap();
        assert!(king.is_ultimate);
        assert_eq!(king.stats.unlock_mastery, 9);
        assert_eq!(king.stats.cooldown_ticks, 1800);
    }

    #[test]
    fn yomi_sword_techniques_freeze_and_soul_king_roar() {
        let m = YomiYomiModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::YomiYomi),
            mastery: 9,
            energy: 200.0,
            active_form: "skeleton".into(),
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
                fruit: FruitId::YomiYomi,
                ability_name: "kasuriuta_fubuki_giri".into(),
                mastery: 9,
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

        // Fubuki Giri freezes target for 60 ticks and transforms water into frosted_ice
        let fb_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kasuriuta_fubuki_giri".into(),
                mastery: 9,
            },
        );
        assert_eq!(fb_res.hits.len(), 1);
        assert_eq!(fb_res.hits[0].freeze_ticks, 60);
        assert_eq!(
            fb_res.hits[0].damage_type,
            GrandLineDamageType::YomiHellfireCold
        );
        assert!(!fb_res.transformed_blocks.is_empty());

        // Enter Soul King form and cast Lullaby Parade (Soul King Roar)
        let sk_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "soul_king".into(),
            },
        );
        assert_eq!(sk_res.form_switch_to.as_deref(), Some("soul_king"));

        let roar_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "lullaby_parade".into(),
                mastery: 9,
            },
        );
        assert_eq!(
            roar_res.damage_type,
            Some(GrandLineDamageType::YomiSoulKingRoar)
        );
        assert_eq!(roar_res.hits.len(), 1);
        assert_eq!(roar_res.hits[0].stun_ticks, 100);
    }

    #[test]
    fn yomi_second_wind_revive_and_water_surface_sprinting() {
        let m = YomiYomiModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::YomiYomi),
            mastery: 8,
            energy: 100.0,
            active_form: "skeleton".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut wfx = WorldFxManager::default();
        let targets = [test_target(3, DVec3::new(0.0, 0.0, 2.5))];

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
                fruit: FruitId::YomiYomi,
                ability_name: "second_wind".into(),
                mastery: 8,
                charge: 1.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 500,
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

        // Trigger Second Wind skeleton revive shockwave
        let revive = trigger_second_wind(&mut ctx).expect("second wind should trigger");
        assert_eq!(revive.hits.len(), 1);
        assert_eq!(revive.hits[0].damage, 4.0);
        assert!(ctx.player.cooldown_remaining("second_wind", 500) == 24_000);
        // Second call while on cooldown is blocked
        assert!(trigger_second_wind(&mut ctx).is_none());

        // Water-surface sprinting sets on_water_override and prevents water weakness sinking
        ctx.form_runtime.set_flag(YOMI_FLAG_WATER_RUN, true);
        let mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.4);
        assert!(mods.on_water_override);
        assert!(!mods.is_water_weakened());

        let e_before = ctx.player.energy;
        let _ = m.on_tick(&mut ctx);
        assert!(ctx.player.energy < e_before);
    }
}
