//! Doru-Doru no Mi (Wax-Wax Fruit) — 1:1 port from Grand Line 0.4.0 Java
//! (`com.devilfruits.fruits.doru_doru`).
//!
//! Includes:
//! - `tools` (`wax_tools` / `candle_lock`): Forms/dissolves wax tools, wax key (`keyEnergy = 15.0`),
//!   or applies a wax restraint lock hold (`RestraintRules`).
//! - `atelier` (`wax_atelier`): Toggle wax sculpting mode (`SCULPTING` form flag, geometric mastery
//!   scaling from mastery 2 to mastery 10).
//! - `wall` (`candle_wall` / `candle_house`): Raises a temporary wax barrier wall (`7x5x1.5`,
//!   `lifetimeSeconds = 45.0`) or crouching dome / Candle House (`domeRadius = 3.0`), scheduling
//!   timed restoration in `WorldFxManager`.
//! - `lance` (`wax_lance` / `doru_doru_arts_mori`): Charged harpoon thrust (`< 8` ticks, pierces
//!   `2..=5` targets) or high-speed thrown wax lance (`>= 8` ticks, `throwSpeed = 2.6`), adding
//!   `WaxCoating` stacks (`MAX = 10` -> full wax encasement).
//! - `service` (`candle_service`): Giant rotating candle stand (`coatRadius = 16.0`, `candleSeconds = 30.0`,
//!   `candles = 8`) that rains wax flakes to coat and encase nearby enemies.
//! - `champion` (`candle_champion`): Giant wax mech suit form (`scale +1.78`, `attack +7.0`, `armor +8.0`,
//!   `absorption = 60.0` / `championIntegrity = 40.0`, `mass = 250.0 kg`, `3.0x` heat vulnerability,
//!   AoE punch splash & landing stomp).

use glam::DVec3;
use horizon_world::grandline_hooks::{
    ENTITY_MASS_CHAMPION_KG, EntityBodyModifiers, EntityMassTier,
};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType, RestraintRules};
use crate::core::form::{FormRuntime, FruitFormDef, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Grand Line 0.4.0 Java constants
// ---------------------------------------------------------------------------

/// `DoruDoru.SCULPTING` synced form flag (`FormFlags.fruit(0)`).
pub const DORU_FLAG_SCULPTING: u32 = form_flags::fruit_bit(0);

/// `DoruDoru` fruit-wide settings & `WaxCoating` constants.
pub const WAX_BLOCK_ID: &str = "devilfruits:wax_block";
pub const WAX_COATING_MAX_STACKS: i32 = 10;
pub const WAX_COATING_SLOW_PER_STACK: f64 = 0.06;
pub const WAX_COATING_SECONDS_MOVING: f64 = 2.0;
pub const WAX_COATING_SECONDS_STILL: f64 = 0.4;
pub const WAX_ENCASE_SECONDS: f64 = 3.0;
pub const WAX_BOSS_ENCASE_SECONDS: f64 = 1.0;
pub const WAX_ENCASE_IMMUNITY_SECONDS: f64 = 6.0;
pub const WAX_ENCASE_SHAKES: f32 = 12.0;
pub const WAX_COATING_SHAKES_PER_STACK: f32 = 2.0;
pub const DORU_BLOCK_COST_AT_MASTERY_2: f64 = 1.5;
pub const DORU_BLOCK_COST_AT_MASTERY_10: f64 = 0.06;
pub const DORU_HONEYCOMB_ENERGY: f32 = 8.0;
pub const DORU_CANDLE_ENERGY: f32 = 5.0;
pub const DORU_LUMP_REFUND_SHARE: f64 = 0.8;

/// `WaxToolsAbility` (`tools` / `candle_lock`): energy 4, cooldown 6, mastery 1, keyEnergy 15.0.
pub const WAX_TOOLS_ENERGY: f32 = 4.0;
pub const WAX_TOOLS_COOLDOWN_TICKS: i32 = 6;
pub const WAX_TOOLS_UNLOCK_MASTERY: u32 = 1;
pub const WAX_KEY_ENERGY: f32 = 15.0;

/// `WaxAtelierAbility` (`atelier`): energy 0, cooldown 10, mastery 2, range 24.0.
pub const WAX_ATELIER_ENERGY: f32 = 0.0;
pub const WAX_ATELIER_COOLDOWN_TICKS: i32 = 10;
pub const WAX_ATELIER_UNLOCK_MASTERY: u32 = 2;
pub const WAX_ATELIER_RANGE: f32 = 24.0;
pub const WAX_ATELIER_REACH_M2: f64 = 24.0;
pub const WAX_ATELIER_REACH_M10: f64 = 40.0;
pub const WAX_ATELIER_JOB_SIZE_M2: f64 = 64.0;
pub const WAX_ATELIER_JOB_SIZE_M10: f64 = 4096.0;
pub const WAX_ATELIER_VOXELS_PER_TICK_M2: f64 = 64.0;
pub const WAX_ATELIER_VOXELS_PER_TICK_M10: f64 = 512.0;

/// `CandleWallAbility` (`wall` / `candle_wall` / `candle_house`): energy 18, cooldown 60, mastery 3, range 12.0.
pub const CANDLE_WALL_ENERGY: f32 = 18.0;
pub const CANDLE_WALL_COOLDOWN_TICKS: i32 = 60;
pub const CANDLE_WALL_UNLOCK_MASTERY: u32 = 3;
pub const CANDLE_WALL_RANGE: f32 = 12.0;
pub const CANDLE_WALL_WIDTH: i32 = 7;
pub const CANDLE_WALL_HEIGHT: i32 = 5;
pub const CANDLE_WALL_THICKNESS: f64 = 1.5;
pub const CANDLE_WALL_BOW: f64 = 1.0;
pub const CANDLE_WALL_DOME_RADIUS: i32 = 3;
pub const CANDLE_WALL_LIFETIME_SECONDS: f64 = 45.0;
pub const CANDLE_WALL_MAX_WALLS: usize = 3;
pub const CANDLE_WALL_RISE_TICKS: i32 = 6;

/// `WaxLanceAbility` (`lance` / `doru_doru_arts_mori`): energy 12, cooldown 30, mastery 5, damage 9.0, range 6.0, knockback 1.4.
pub const WAX_LANCE_ENERGY: f32 = 12.0;
pub const WAX_LANCE_COOLDOWN_TICKS: i32 = 30;
pub const WAX_LANCE_UNLOCK_MASTERY: u32 = 5;
pub const WAX_LANCE_DAMAGE: f32 = 9.0;
pub const WAX_LANCE_RANGE: f32 = 6.0;
pub const WAX_LANCE_KNOCKBACK: f32 = 1.4;
pub const WAX_LANCE_THROW_CHARGE_TICKS: i32 = 8;
pub const WAX_LANCE_CHARGE_TICKS: i32 = 20;
pub const WAX_LANCE_PIERCE_M5: f64 = 2.0;
pub const WAX_LANCE_PIERCE_M10: f64 = 5.0;
pub const WAX_LANCE_THROW_SPEED: f64 = 2.6;
pub const WAX_LANCE_PEG_LIFETIME_SECONDS: f64 = 60.0;

/// `CandleServiceAbility` (`service`): energy 40, cooldown 120, mastery 8, range 10.0.
pub const CANDLE_SERVICE_ENERGY: f32 = 40.0;
pub const CANDLE_SERVICE_COOLDOWN_TICKS: i32 = 120;
pub const CANDLE_SERVICE_UNLOCK_MASTERY: u32 = 8;
pub const CANDLE_SERVICE_RANGE: f32 = 10.0;
pub const CANDLE_SERVICE_COAT_RADIUS: f64 = 16.0;
pub const CANDLE_SERVICE_CANDLE_SECONDS: f64 = 30.0;
pub const CANDLE_SERVICE_CANDLES: i32 = 8;
pub const CANDLE_SERVICE_STAND_INTEGRITY: f64 = 0.5;
pub const CANDLE_SERVICE_RISE_TICKS: i32 = 30;

/// `ChampionForm` (`champion` / `candle_champion`): mastery 10, enter_cost 60.0, drain 1.0/s, cooldown 600, cast_ticks 30.
pub const CHAMPION_UNLOCK_MASTERY: u32 = 10;
pub const CHAMPION_ENTER_COST: f32 = 60.0;
pub const CHAMPION_DRAIN_PER_SEC: f32 = 1.0;
pub const CHAMPION_COOLDOWN_TICKS: i32 = 600;
pub const CHAMPION_CAST_TICKS: i32 = 30;
pub const CHAMPION_INTEGRITY_ABSORPTION: f32 = 40.0;
pub const CHAMPION_HEAT_DAMAGE_MULTIPLIER: f32 = 3.0;
pub const CHAMPION_PUNCH_SPLASH_FACTOR: f32 = 0.6;
pub const CHAMPION_PUNCH_SPLASH_RADIUS: f64 = 3.0;

/// Canonical Doru-Doru VFX identifiers (`DoruFx.java`).
pub mod doru_fx {
    pub const ABSORB: &str = "devilfruits:doru_doru/absorb";
    pub const HEAT_WARNING: &str = "devilfruits:doru_doru/heat_warning";
    pub const TOOL_FORM: &str = "devilfruits:doru_doru/tool_form";
    pub const WALL_RISE: &str = "devilfruits:doru_doru/wall_rise";
    pub const LANCE_FORM: &str = "devilfruits:doru_doru/lance_form";
    pub const LANCE_THRUST: &str = "devilfruits:doru_doru/lance_thrust";
    pub const LANCE_THROW: &str = "devilfruits:doru_doru/lance_throw";
    pub const LANCE_HIT: &str = "devilfruits:doru_doru/lance_hit";
    pub const ENCASE: &str = "devilfruits:doru_doru/encase";
    pub const BREAK_FREE: &str = "devilfruits:doru_doru/break_free";
    pub const MELT_OFF: &str = "devilfruits:doru_doru/melt_off";
    pub const STRUGGLE: &str = "devilfruits:doru_doru/struggle";
    pub const SERVICE: &str = "devilfruits:doru_doru/service";
    pub const SERVICE_END: &str = "devilfruits:doru_doru/service_end";
    pub const CHAMPION_RISE: &str = "devilfruits:doru_doru/champion_rise";
    pub const CHAMPION_MELT: &str = "devilfruits:doru_doru/champion_melt";
    pub const CHAMPION_PUNCH: &str = "devilfruits:doru_doru/champion_punch";
    pub const CHAMPION_STOMP: &str = "devilfruits:doru_doru/champion_stomp";
}

/// Geometric mastery scaling matching `DoruDoru.blockCost(mastery)` & `WaxAtelierAbility.scaled(lo, hi, mastery)`.
pub fn doru_geometric_scale(lo: f64, hi: f64, mastery: u32) -> f64 {
    let t = (((mastery as f64) - 2.0) / 8.0).clamp(0.0, 1.0);
    if lo > 0.0 && hi > 0.0 {
        lo * (hi / lo).powf(t)
    } else {
        lo + (hi - lo) * t
    }
}

/// Energy cost per poured wax block at `mastery` (`DoruDoru.blockCost`).
pub fn wax_block_cost(mastery: u32) -> f64 {
    doru_geometric_scale(
        DORU_BLOCK_COST_AT_MASTERY_2,
        DORU_BLOCK_COST_AT_MASTERY_10,
        mastery,
    )
}

/// Wax restraint hold rules for Candle Lock / Wax Encasement (`RestraintRules`).
pub fn wax_restraint_rules(is_boss: bool) -> RestraintRules {
    let secs = if is_boss {
        WAX_BOSS_ENCASE_SECONDS
    } else {
        WAX_ENCASE_SECONDS
    };
    let mut rules = RestraintRules::of("doru_doru:wax_lock", (secs * 20.0).round() as i32);
    rules.struggle_presses = WAX_ENCASE_SHAKES;
    rules.max_struggle_cut = 1.0;
    rules
}

/// Doru-Doru no Mi module (`DoruDoru.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct DoruDoruModule;

impl DoruDoruModule {
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
            id: AbilityId::new(FruitId::DoruDoru, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms,
            stats,
            is_ultimate,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        }
    }

    fn tools_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: WAX_TOOLS_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: WAX_TOOLS_COOLDOWN_TICKS,
            unlock_mastery: WAX_TOOLS_UNLOCK_MASTERY,
            damage: 4.0,
            range: 6.0,
            knockback: 0.2,
            ..AbilityStats::default()
        }
    }

    fn atelier_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: WAX_ATELIER_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: WAX_ATELIER_COOLDOWN_TICKS,
            unlock_mastery: WAX_ATELIER_UNLOCK_MASTERY,
            damage: 0.0,
            range: WAX_ATELIER_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn wall_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: CANDLE_WALL_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: CANDLE_WALL_COOLDOWN_TICKS,
            unlock_mastery: CANDLE_WALL_UNLOCK_MASTERY,
            damage: 0.0,
            range: CANDLE_WALL_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn lance_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: WAX_LANCE_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: WAX_LANCE_COOLDOWN_TICKS,
            unlock_mastery: WAX_LANCE_UNLOCK_MASTERY,
            damage: WAX_LANCE_DAMAGE,
            range: WAX_LANCE_RANGE,
            knockback: WAX_LANCE_KNOCKBACK,
            max_charge_ticks: WAX_LANCE_CHARGE_TICKS,
            ..AbilityStats::default()
        }
    }

    fn service_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: CANDLE_SERVICE_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: CANDLE_SERVICE_COOLDOWN_TICKS,
            unlock_mastery: CANDLE_SERVICE_UNLOCK_MASTERY,
            damage: 0.0,
            range: CANDLE_SERVICE_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn champion_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: CHAMPION_ENTER_COST,
            energy_per_second: CHAMPION_DRAIN_PER_SEC,
            cooldown_ticks: CHAMPION_COOLDOWN_TICKS,
            unlock_mastery: CHAMPION_UNLOCK_MASTERY,
            damage: 14.0,
            range: 4.5,
            knockback: 1.0,
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
            "wax_tools" | "candle_lock" => "tools",
            "wax_atelier" => "atelier",
            "candle_wall" => "wall",
            "wax_lance" | "doru_doru_arts_mori" => "lance",
            "candle_service" => "service",
            "candle_champion" => "champion",
            other => other,
        }
    }

    fn exec_tools(
        &self,
        ctx: &mut FruitContext<'_>,
        raw_name: &str,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::tools_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let encase_ticks = (WAX_ENCASE_SECONDS * 20.0).round() as i32;

        res.animation_clip = Some("devilfruits:doru_doru/tool_form".to_string());
        ctx.world_fx.emit_fx(
            doru_fx::TOOL_FORM,
            0,
            eye,
            look,
            1.0,
            ctx.ability_ctx.tick,
            0,
            32.0,
        );

        // When invoked as `candle_lock` or with a target in range, binds the target in a wax restraint lock
        if raw_name.contains("candle_lock") || !ctx.nearby_targets.is_empty() {
            if let Some(t) = CombatTargeting::aimed(
                ctx.nearby_targets,
                ctx.player_id,
                eye,
                look,
                range,
                0.55,
            ) {
                ctx.world_fx.emit_fx(
                    doru_fx::ENCASE,
                    0,
                    t.center,
                    DVec3::ZERO,
                    encase_ticks as f32,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: dmg,
                    damage_type: GrandLineDamageType::DoruWax,
                    knockback_impulse: DVec3::ZERO,
                    stun_ticks: encase_ticks,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
            }
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_atelier(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let active = match event {
            AbilityEvent::Toggled { enabled, .. } => *enabled,
            AbilityEvent::ToggleTick { .. } | AbilityEvent::CastInstant { .. } => {
                !ctx.form_runtime.has_flag(DORU_FLAG_SCULPTING)
            }
            _ => false,
        };
        ctx.form_runtime.set_flag(DORU_FLAG_SCULPTING, active);
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_wall(
        &self,
        ctx: &mut FruitContext<'_>,
        is_dome: bool,
        res: &mut AbilityExecutionResult,
    ) {
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let facing = if flat.length_squared() < 1e-4 {
            DVec3::Z
        } else {
            flat
        };
        let right = DVec3::new(-facing.z, 0.0, facing.x);
        let base = if is_dome {
            ctx.ability_ctx.eye_pos
        } else {
            ctx.ability_ctx
                .aim_point
                .unwrap_or(ctx.ability_ctx.eye_pos + facing * 4.0)
        };
        let restore_ticks = (CANDLE_WALL_LIFETIME_SECONDS * 20.0).round() as i32;

        let bx = base.x.floor() as i32;
        let by = base.y.floor() as i32;
        let bz = base.z.floor() as i32;

        if is_dome {
            // Crouching dome / Candle House shell
            let r = CANDLE_WALL_DOME_RADIUS;
            for dy in 0..=r {
                for dx in -r..=r {
                    for dz in -r..=r {
                        let d2 = dx * dx + dy * dy + dz * dz;
                        if d2 >= (r - 1) * (r - 1) && d2 <= r * r + 1 {
                            let pos = [bx + dx, by + dy, bz + dz];
                            let restore_at = ctx.ability_ctx.tick + restore_ticks as i64;
                            ctx.world_fx.schedule_restore(
                                pos,
                                "minecraft:air",
                                Some(WAX_BLOCK_ID),
                                restore_at,
                            );
                            res.transformed_blocks.push((pos, WAX_BLOCK_ID.to_string()));
                        }
                    }
                }
            }
            res.animation_clip = Some("devilfruits:doru_doru/dome_raise".to_string());
            ctx.world_fx.emit_fx(
                doru_fx::WALL_RISE,
                0,
                base,
                facing,
                CANDLE_WALL_DOME_RADIUS as f32,
                ctx.ability_ctx.tick,
                1,
                64.0,
            );
        } else {
            // Bowed 7x5x1.5 Candle Wall
            let half_w = CANDLE_WALL_WIDTH / 2;
            for w in -half_w..=half_w {
                let norm = (w as f64) / (half_w.max(1) as f64);
                let bow_off = (1.0 - norm * norm) * CANDLE_WALL_BOW;
                for h in 0..CANDLE_WALL_HEIGHT {
                    let pt = base
                        + right * (w as f64)
                        + facing * bow_off
                        + DVec3::new(0.0, h as f64, 0.0);
                    let pos = [pt.x.floor() as i32, pt.y.floor() as i32, pt.z.floor() as i32];
                    let restore_at = ctx.ability_ctx.tick + restore_ticks as i64;
                    ctx.world_fx.schedule_restore(
                        pos,
                        "minecraft:air",
                        Some(WAX_BLOCK_ID),
                        restore_at,
                    );
                    res.transformed_blocks.push((pos, WAX_BLOCK_ID.to_string()));
                }
            }
            res.animation_clip = Some("devilfruits:doru_doru/wall_slam".to_string());
            ctx.world_fx.emit_fx(
                doru_fx::WALL_RISE,
                0,
                base,
                facing,
                CANDLE_WALL_WIDTH as f32,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_lance(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::lance_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let hand = eye + look * 0.6 + DVec3::new(0.0, -0.3, 0.0);

        match event {
            AbilityEvent::ChargeStarted { .. } => {
                res.animation_clip = Some("devilfruits:doru_doru/lance_charge".to_string());
                ctx.world_fx.emit_fx(
                    doru_fx::LANCE_FORM,
                    0,
                    eye,
                    look,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
            }
            AbilityEvent::ChargeReleased { charge, .. } => {
                self.fire_lance(ctx, &stats, hand, look, *charge, res);
            }
            AbilityEvent::CastInstant { .. } => {
                self.fire_lance(ctx, &stats, hand, look, ctx.ability_ctx.charge, res);
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn fire_lance(
        &self,
        ctx: &mut FruitContext<'_>,
        stats: &AbilityStats,
        hand: DVec3,
        look: DVec3,
        charge: f32,
        res: &mut AbilityExecutionResult,
    ) {
        let c = charge.clamp(0.0, 1.0);
        let held_ticks = (c * (WAX_LANCE_CHARGE_TICKS as f32)).round() as i32;
        let base_dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let push = flat * (WAX_LANCE_KNOCKBACK as f64) + DVec3::new(0.0, 0.35, 0.0);

        if held_ticks >= WAX_LANCE_THROW_CHARGE_TICKS {
            // Thrown Wax Lance projectile (`throwSpeed = 2.6`)
            let power = 0.75 + 0.25 * c;
            let vel = look * (WAX_LANCE_THROW_SPEED * (power as f64));
            res.spawned_projectiles = 1;
            res.animation_clip = Some("devilfruits:doru_doru/lance_throw".to_string());
            ctx.world_fx.emit_fx(
                doru_fx::LANCE_THROW,
                0,
                hand,
                vel,
                power,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );

            let end = hand + look * 24.0;
            for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, hand, end, 0.85)
                .into_iter()
                .take(1)
            {
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: base_dmg * power,
                    damage_type: GrandLineDamageType::DoruWax,
                    knockback_impulse: push,
                    stun_ticks: 0,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
            }
        } else {
            // Piercing melee thrust (`pierceAtMastery5 = 2` -> `pierceAtMastery10 = 5`)
            res.animation_clip = Some("devilfruits:doru_doru/lance_thrust".to_string());
            let reach =
                stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
                    as f64;
            let end = hand + look * reach;
            let t_norm = (((ctx.player.mastery as f64) - 5.0) / 5.0).clamp(0.0, 1.0);
            let pierce = (WAX_LANCE_PIERCE_M5
                + (WAX_LANCE_PIERCE_M10 - WAX_LANCE_PIERCE_M5) * t_norm)
                .round() as usize;

            let mut hits =
                CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, hand, end, 0.75);
            hits.sort_by(|a, b| {
                a.center
                    .distance_squared(hand)
                    .partial_cmp(&b.center.distance_squared(hand))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for (idx, t) in hits.into_iter().take(pierce.max(1)).enumerate() {
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: base_dmg,
                    damage_type: GrandLineDamageType::DoruWax,
                    knockback_impulse: push,
                    stun_ticks: 0,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
                ctx.world_fx.emit_fx(
                    doru_fx::LANCE_HIT,
                    0,
                    t.center,
                    look,
                    1.0,
                    ctx.ability_ctx.tick + (idx as i64),
                    0,
                    64.0,
                );
            }
            ctx.world_fx.emit_fx(
                doru_fx::LANCE_THRUST,
                0,
                hand,
                end - hand,
                res.hits.len() as f32,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        }
    }

    fn exec_service(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let base = ctx.ability_ctx.aim_point.unwrap_or(eye + look * 4.0);
        let plate = base + DVec3::new(0.0, 3.0, 0.0);
        let burn_ticks = (CANDLE_SERVICE_CANDLE_SECONDS * 20.0).round() as i32;
        let restore_at = ctx.ability_ctx.tick + (CANDLE_SERVICE_RISE_TICKS + burn_ticks) as i64;

        res.animation_clip = Some("devilfruits:doru_doru/service_raise".to_string());

        // Build candle stand pillar & plate blocks
        let bx = base.x.floor() as i32;
        let by = base.y.floor() as i32;
        let bz = base.z.floor() as i32;
        for dy in 0..=3 {
            let pos = [bx, by + dy, bz];
            ctx.world_fx
                .schedule_restore(pos, "minecraft:air", Some(WAX_BLOCK_ID), restore_at);
            res.transformed_blocks.push((pos, WAX_BLOCK_ID.to_string()));
        }

        // Coat & encase nearby targets within `CANDLE_SERVICE_COAT_RADIUS` (16.0)
        let encase_ticks = (WAX_ENCASE_SECONDS * 20.0).round() as i32;
        for t in CombatTargeting::in_radius(
            ctx.nearby_targets,
            ctx.player_id,
            plate,
            CANDLE_SERVICE_COAT_RADIUS,
        ) {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: 0.0,
                damage_type: GrandLineDamageType::DoruWax,
                knockback_impulse: DVec3::ZERO,
                stun_ticks: encase_ticks,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        ctx.world_fx.emit_fx(
            doru_fx::SERVICE,
            -1,
            plate,
            DVec3::new(2.35, 1.0, CANDLE_SERVICE_COAT_RADIUS),
            CANDLE_SERVICE_CANDLE_SECONDS as f32,
            ctx.ability_ctx.tick,
            CANDLE_SERVICE_CANDLES,
            128.0,
        );
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_champion(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let entering = ctx.player.active_form != "champion";
        if entering {
            ctx.player.active_form = "champion".to_string();
            res.form_switch_to = Some("champion".to_string());
            res.animation_clip = Some("devilfruits:doru_doru/champion/transform".to_string());
            ctx.world_fx.emit_fx(
                doru_fx::CHAMPION_RISE,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                CHAMPION_INTEGRITY_ABSORPTION,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
            ctx.world_fx.emit_shake(ctx.ability_ctx.eye_pos, 0.75, 28);
            res.screen_shake = Some((0.75, 28));
        } else {
            ctx.player.active_form = "base".to_string();
            res.form_switch_to = Some("base".to_string());
            res.animation_clip = Some("devilfruits:doru_doru/champion/revert".to_string());
            ctx.world_fx.emit_fx(
                doru_fx::CHAMPION_MELT,
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
}

impl DevilFruitModule for DoruDoruModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::DoruDoru
    }

    fn forms(&self) -> Vec<FruitFormDef> {
        let mut forms = FruitFormDef::builtin_forms(FruitId::DoruDoru);
        for f in &mut forms {
            if f.id.name == "base" {
                for alias in [
                    "candle_lock",
                    "doru_doru_arts_mori",
                    "candle_wall",
                    "candle_house",
                    "candle_champion",
                    "wax_tools",
                    "wax_atelier",
                    "wax_lance",
                    "candle_service",
                ] {
                    if !f.abilities.iter().any(|a| a == alias) {
                        f.abilities.push(alias.to_string());
                    }
                }
            } else if f.id.name == "champion" {
                for alias in [
                    "candle_champion",
                    "candle_wall",
                    "candle_house",
                    "doru_doru_arts_mori",
                    "wax_lance",
                ] {
                    if !f.abilities.iter().any(|a| a == alias) {
                        f.abilities.push(alias.to_string());
                    }
                }
            }
        }
        forms
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_only = vec!["base".to_string()];
        let both_forms = vec!["base".to_string(), "champion".to_string()];
        vec![
            Self::make_ability(
                "tools",
                "Wax Tools (Candle Lock)",
                CastType::Instant,
                base_only.clone(),
                Self::tools_stats(),
                false,
            ),
            Self::make_ability(
                "atelier",
                "Wax Atelier",
                CastType::Toggle,
                base_only.clone(),
                Self::atelier_stats(),
                false,
            ),
            Self::make_ability(
                "wall",
                "Candle Wall",
                CastType::Instant,
                both_forms.clone(),
                Self::wall_stats(),
                false,
            ),
            Self::make_ability(
                "lance",
                "Doru Doru Arts: Mori (Wax Lance)",
                CastType::Charged,
                both_forms.clone(),
                Self::lance_stats(),
                false,
            ),
            Self::make_ability(
                "service",
                "Candle Service Set",
                CastType::Instant,
                base_only.clone(),
                Self::service_stats(),
                false,
            ),
            Self::make_ability(
                "champion",
                "Candle Champion",
                CastType::FormSwitch,
                both_forms.clone(),
                Self::champion_stats(),
                true,
            ),
            // Spec aliases
            Self::make_ability(
                "candle_lock",
                "Candle Lock",
                CastType::Instant,
                base_only.clone(),
                Self::tools_stats(),
                false,
            ),
            Self::make_ability(
                "doru_doru_arts_mori",
                "Doru Doru Arts: Mori",
                CastType::Charged,
                both_forms.clone(),
                Self::lance_stats(),
                false,
            ),
            Self::make_ability(
                "candle_wall",
                "Candle Wall",
                CastType::Instant,
                both_forms.clone(),
                Self::wall_stats(),
                false,
            ),
            Self::make_ability(
                "candle_house",
                "Candle House",
                CastType::Instant,
                both_forms.clone(),
                Self::wall_stats(),
                false,
            ),
            Self::make_ability(
                "candle_champion",
                "Candle Champion",
                CastType::FormSwitch,
                both_forms.clone(),
                Self::champion_stats(),
                true,
            ),
            Self::make_ability(
                "wax_tools",
                "Wax Tools",
                CastType::Instant,
                base_only.clone(),
                Self::tools_stats(),
                false,
            ),
            Self::make_ability(
                "wax_atelier",
                "Wax Atelier",
                CastType::Toggle,
                base_only.clone(),
                Self::atelier_stats(),
                false,
            ),
            Self::make_ability(
                "wax_lance",
                "Wax Lance",
                CastType::Charged,
                both_forms,
                Self::lance_stats(),
                false,
            ),
            Self::make_ability(
                "candle_service",
                "Candle Service",
                CastType::Instant,
                base_only,
                Self::service_stats(),
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
            damage_type: Some(GrandLineDamageType::DoruWax),
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

        if raw_name.ends_with("candle_house") {
            self.exec_wall(ctx, true, &mut res);
            return res;
        }

        match Self::canonical_name(raw_name) {
            "tools" => {
                self.exec_tools(ctx, raw_name, &mut res);
            }
            "atelier" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_atelier(ctx, event, &mut res);
            }
            "wall" => {
                self.exec_wall(ctx, false, &mut res);
            }
            "lance" => {
                if matches!(event, AbilityEvent::ChargeStarted { .. }) {
                    res.cast_result = Some(CastResult::Charging);
                }
                self.exec_lance(ctx, event, &mut res);
            }
            "service" => {
                self.exec_service(ctx, &mut res);
            }
            "champion" => {
                self.exec_champion(ctx, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed {
            ctx.form_runtime.set_flag(DORU_FLAG_SCULPTING, false);
        }
        if ctx.player.active_form == "champion" {
            // Lava / intense heat rapidly melts the Candle Champion suit (`ChampionForm.tick`)
            if ctx.in_lava || ctx.suppressed || ctx.submerged_in_water {
                ctx.player.active_form = "base".to_string();
                res.form_switch_to = Some("base".to_string());
                ctx.world_fx.emit_fx(
                    doru_fx::HEAT_WARNING,
                    0,
                    ctx.ability_ctx.eye_pos,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    32.0,
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
        if player.active_form == "champion" {
            mods.mass_tier = EntityMassTier::Heavy;
            mods.mass_kg_override = Some(ENTITY_MASS_CHAMPION_KG);
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
    fn doru_abilities_and_geometric_scaling_match_java() {
        let m = DoruDoruModule::new();
        assert_eq!(m.fruit_id(), FruitId::DoruDoru);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::DoruWax);

        let lock = m.ability("candle_lock").unwrap();
        assert_eq!(lock.stats.energy_cost, 4.0);
        assert_eq!(lock.stats.cooldown_ticks, 6);
        assert_eq!(lock.stats.unlock_mastery, 1);

        let wall = m.ability("candle_wall").unwrap();
        assert_eq!(wall.stats.energy_cost, 18.0);
        assert_eq!(wall.stats.cooldown_ticks, 60);
        assert_eq!(wall.stats.unlock_mastery, 3);

        let mori = m.ability("doru_doru_arts_mori").unwrap();
        assert_eq!(mori.cast_type, CastType::Charged);
        assert_eq!(mori.stats.energy_cost, 12.0);
        assert_eq!(mori.stats.damage, 9.0);
        assert_eq!(mori.stats.unlock_mastery, 5);

        let champ = m.ability("candle_champion").unwrap();
        assert!(champ.is_ultimate);
        assert_eq!(champ.stats.unlock_mastery, 10);
        assert_eq!(champ.stats.energy_cost, 60.0);
        assert_eq!(champ.stats.cooldown_ticks, 600);

        assert!((wax_block_cost(2) - 1.5).abs() < 1e-6);
        assert!((wax_block_cost(10) - 0.06).abs() < 1e-6);
        let rules = wax_restraint_rules(false);
        assert_eq!(rules.ticks, 60);
        assert_eq!(rules.struggle_presses, 12.0);
    }

    #[test]
    fn doru_candle_wall_house_lance_and_champion_mech() {
        let m = DoruDoruModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::DoruDoru),
            mastery: 10,
            energy: 250.0,
            active_form: "base".into(),
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
                fruit: FruitId::DoruDoru,
                ability_name: "candle_wall".into(),
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

        // Candle Wall places temporary wax blocks scheduled for restoration
        let wall_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "candle_wall".into(),
                mastery: 10,
            },
        );
        assert_eq!(wall_res.transformed_blocks.len(), 35);
        assert!(!ctx.world_fx.restore_entries.is_empty());

        // Candle House dome
        let house_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "candle_house".into(),
                mastery: 10,
            },
        );
        assert!(house_res.transformed_blocks.len() > 20);

        // Quick tap (< 8 ticks) does melee thrust; full charge throws projectile
        let thrust_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "doru_doru_arts_mori".into(),
                charge: 0.2,
                mastery: 10,
            },
        );
        assert_eq!(thrust_res.spawned_projectiles, 0);
        assert_eq!(thrust_res.hits.len(), 1);

        let throw_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "doru_doru_arts_mori".into(),
                charge: 1.0,
                mastery: 10,
            },
        );
        assert_eq!(throw_res.spawned_projectiles, 1);
        assert_eq!(throw_res.hits.len(), 1);

        // Switch to Candle Champion form (250 kg mass) and melt on lava tick
        let champ_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "candle_champion".into(),
            },
        );
        assert_eq!(champ_res.form_switch_to.as_deref(), Some("champion"));
        let mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert_eq!(mods.mass_kg_override, Some(250.0));
        assert_eq!(mods.mass_tier, EntityMassTier::Heavy);

        ctx.in_lava = true;
        let melt_res = m.on_tick(&mut ctx);
        assert_eq!(melt_res.form_switch_to.as_deref(), Some("base"));
    }
}
