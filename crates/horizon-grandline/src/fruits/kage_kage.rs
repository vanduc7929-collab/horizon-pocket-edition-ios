//! Kage-Kage no Mi (Shadow-Shadow Fruit) — 1:1 port from Grand Line 0.4.0 Java
//! (`com.devilfruits.fruits.kage_kage`).
//!
//! Includes:
//! - `brick_bat`: Shadow Aegis projectile-intercept shield (`CATCH_RADIUS = 2.4`, 4 intercepts/s,
//!   `interceptEnergy = 3.0`) that releases a 10-bat homing swarm (`swarmEnergy = 12.0`) on toggle off.
//! - `spear` (`tsuno_tokage` / `shadow_spear`): Ground shadow spear that impales & stuns (`2.5s = 50 ticks`).
//! - `collection` (`shadow_extract` / `shadow_collection`): Held 40-tick scissors snip that steals a
//!   victim's shadow (`maxShadows = 100`, `+0.3%` damage, `+0.2` HP, `+0.05%` speed per shadow) and marks
//!   the victim shadowless (`Sunburn` in daylight).
//! - `trap` (`black_box` / `shadow_trap`): Encases a target inside a shadow casket for `2.5s` (3 damage pulses).
//! - `hand` (`kage_kakumei` / `shadow_hand`): Giant shadow palm slap AoE (`radius = 4.0`, Slowness II for 2s).
//! - `doppelman` & `shadows_asgard`: Summons/commands the 80-HP Doppelman shadow clone, supports place-swapping
//!   (`swapEnergy = 6.0`, `swapCooldownTicks = 30`), and Asgard giant shadow-absorption form.

use glam::DVec3;
use horizon_world::grandline_hooks::{EntityBodyModifiers, EntityMassTier};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{
    FormAttributeBonuses, FormCollision, FormId, FormRuntime, FormStats, FruitFormDef, form_flags,
};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Grand Line 0.4.0 Java constants
// ---------------------------------------------------------------------------

/// `KageKage.AEGIS` & Asgard / Doppelman synced form flags.
pub const KAGE_FLAG_AEGIS: u32 = form_flags::fruit_bit(0);
pub const KAGE_FLAG_DOPPELMAN_OUT: u32 = form_flags::fruit_bit(1);
pub const KAGE_FLAG_ASGARD: u32 = form_flags::fruit_bit(2);

/// `Shadows` scaling settings (`KageKage.java` + `Shadows.java`).
pub const KAGE_MAX_SHADOWS: u32 = 100;
pub const KAGE_SHADOW_DAMAGE_BONUS: f32 = 0.003;
pub const KAGE_SHADOW_HEALTH_BONUS: f64 = 0.2;
pub const KAGE_SHADOW_SPEED_BONUS: f64 = 5.0e-4;
pub const KAGE_SHADOW_SPEED_CAP: f64 = 0.03;
pub const KAGE_SUNBURN_SECONDS: i32 = 8;
pub const KAGE_MAX_ZOMBIES: usize = 6;

/// `BrickBatAbility`: energy 6, drain 2/s, cooldown 240, mastery 1, damage 20.0 (10 bats), range 30.0, knockback 0.15.
pub const BRICK_BAT_ENERGY: f32 = 6.0;
pub const BRICK_BAT_DRAIN_PER_SEC: f32 = 2.0;
pub const BRICK_BAT_COOLDOWN_TICKS: i32 = 240;
pub const BRICK_BAT_UNLOCK_MASTERY: u32 = 1;
pub const BRICK_BAT_DAMAGE: f32 = 20.0;
pub const BRICK_BAT_RANGE: f32 = 30.0;
pub const BRICK_BAT_KNOCKBACK: f32 = 0.15;
pub const BRICK_BAT_AEGIS_MAX_SECONDS: f64 = 30.0;
pub const BRICK_BAT_INTERCEPTS_PER_SEC: u32 = 4;
pub const BRICK_BAT_INTERCEPT_ENERGY: f32 = 3.0;
pub const BRICK_BAT_CATCH_RADIUS: f64 = 2.4;
pub const BRICK_BAT_COUNT: usize = 10;
pub const BRICK_BAT_SWARM_ENERGY: f32 = 12.0;
pub const BRICK_BAT_AIM_CONE_DEGREES: f64 = 8.0;
pub const BRICK_BAT_AUTO_TARGET_RANGE: f64 = 20.0;

/// `ShadowSpearAbility` (`spear` / `tsuno_tokage`): energy 18, cooldown 200, mastery 2, damage 9.0, range 30.0, stun 2.5s (50 ticks).
pub const SHADOW_SPEAR_ENERGY: f32 = 18.0;
pub const SHADOW_SPEAR_COOLDOWN_TICKS: i32 = 200;
pub const SHADOW_SPEAR_UNLOCK_MASTERY: u32 = 2;
pub const SHADOW_SPEAR_DAMAGE: f32 = 9.0;
pub const SHADOW_SPEAR_RANGE: f32 = 30.0;
pub const SHADOW_SPEAR_STUN_SECONDS: f64 = 2.5;
pub const SHADOW_SPEAR_BATS: f32 = 4.0;
pub const SHADOW_SPEAR_AIM_CONE_DEGREES: f64 = 6.0;

/// `ShadowCollectionAbility` (`collection` / `shadow_extract`): energy 25, cooldown 100, mastery 3, range 5.0, castTicks 40.
pub const SHADOW_COLLECTION_ENERGY: f32 = 25.0;
pub const SHADOW_COLLECTION_COOLDOWN_TICKS: i32 = 100;
pub const SHADOW_COLLECTION_UNLOCK_MASTERY: u32 = 3;
pub const SHADOW_COLLECTION_RANGE: f32 = 5.0;
pub const SHADOW_COLLECTION_CAST_TICKS: i32 = 40;
pub const SHADOW_COLLECTION_AIM_CONE_DEGREES: f64 = 12.0;

/// `ShadowTrapAbility` (`trap` / `black_box`): energy 30, cooldown 320, mastery 4, damage 18.0, range 24.0, knockback 0.9.
pub const SHADOW_TRAP_ENERGY: f32 = 30.0;
pub const SHADOW_TRAP_COOLDOWN_TICKS: i32 = 320;
pub const SHADOW_TRAP_UNLOCK_MASTERY: u32 = 4;
pub const SHADOW_TRAP_DAMAGE: f32 = 18.0;
pub const SHADOW_TRAP_RANGE: f32 = 24.0;
pub const SHADOW_TRAP_KNOCKBACK: f32 = 0.9;
pub const SHADOW_TRAP_RADIUS: f64 = 2.5;
pub const SHADOW_TRAP_HOLD_SECONDS: f64 = 2.5;
pub const SHADOW_TRAP_PULSES: usize = 3;
pub const SHADOW_TRAP_BOSS_HOLD_FACTOR: f64 = 0.4;

/// `ShadowHandAbility` (`hand` / `kage_kakumei`): energy 35, cooldown 400, mastery 6, damage 12.0, range 30.0, knockback 1.2, radius 4.0.
pub const SHADOW_HAND_ENERGY: f32 = 35.0;
pub const SHADOW_HAND_COOLDOWN_TICKS: i32 = 400;
pub const SHADOW_HAND_UNLOCK_MASTERY: u32 = 6;
pub const SHADOW_HAND_DAMAGE: f32 = 12.0;
pub const SHADOW_HAND_RANGE: f32 = 30.0;
pub const SHADOW_HAND_KNOCKBACK: f32 = 1.2;
pub const SHADOW_HAND_RADIUS: f64 = 4.0;
pub const SHADOW_HAND_SLOWNESS_SECONDS: f64 = 2.0;
pub const SHADOW_HAND_BATS: f32 = 7.0;
pub const SHADOW_HAND_HEIGHT: f64 = 5.5;

/// `DoppelmanAbility` & `ShadowsAsgard`: energy 50, cooldown 1200, mastery 8, damage 3.5, range 30.0, knockback 0.4.
pub const DOPPELMAN_ENERGY: f32 = 50.0;
pub const DOPPELMAN_COOLDOWN_TICKS: i32 = 1200;
pub const DOPPELMAN_UNLOCK_MASTERY: u32 = 8;
pub const DOPPELMAN_DAMAGE: f32 = 3.5;
pub const DOPPELMAN_RANGE: f32 = 30.0;
pub const DOPPELMAN_KNOCKBACK: f32 = 0.4;
pub const DOPPELMAN_HEALTH: f64 = 80.0;
pub const DOPPELMAN_LEASH_BLOCKS: f64 = 24.0;
pub const DOPPELMAN_COMMAND_ENERGY: f32 = 2.0;
pub const DOPPELMAN_SWAP_ENERGY: f32 = 6.0;
pub const DOPPELMAN_SWAP_COOLDOWN_TICKS: i32 = 30;

/// Canonical Kage-Kage VFX identifiers (`KageFx.java`).
pub mod kage_fx {
    pub const AEGIS_ON: &str = "devilfruits:kage_kage/aegis_on";
    pub const AEGIS_OFF: &str = "devilfruits:kage_kage/aegis_off";
    pub const AEGIS_BLOCK: &str = "devilfruits:kage_kage/aegis_block";
    pub const SWARM: &str = "devilfruits:kage_kage/swarm";
    pub const SPEAR: &str = "devilfruits:kage_kage/spear";
    pub const TRAP: &str = "devilfruits:kage_kage/trap";
    pub const TRAP_SWALLOW: &str = "devilfruits:kage_kage/trap_swallow";
    pub const HAND: &str = "devilfruits:kage_kage/hand";
    pub const DOPPEL_POOL: &str = "devilfruits:kage_kage/doppel_pool";
    pub const DOPPEL_ORDER: &str = "devilfruits:kage_kage/doppel_order";
    pub const COLLECT: &str = "devilfruits:kage_kage/collect";
    pub const COLLECT_SNIP: &str = "devilfruits:kage_kage/collect_snip";
    pub const COLLECT_END: &str = "devilfruits:kage_kage/collect_end";
}

/// 1:1 port of `com.devilfruits.fruits.kage_kage.KageTiming`.
pub struct KageTiming;

impl KageTiming {
    pub fn bat_run(distance: f64) -> i32 {
        (distance.ceil() as i32).clamp(6, 30)
    }

    pub fn bat_flight(distance: f64) -> i32 {
        ((distance * 1.3 / 1.05).ceil() as i32).clamp(6, 40)
    }

    pub fn bat_arrival(index: usize, distance: f64) -> i32 {
        10 + ((index as f32) * 1.2).round() as i32 + Self::bat_flight(distance)
    }

    pub fn spear_run(distance: f64) -> i32 {
        ((distance / 0.9).ceil() as i32).clamp(6, 34)
    }

    pub fn spear_pierce(run: i32) -> i32 {
        run + 6 + 3
    }

    pub fn trap_pull(run: i32) -> i32 {
        run + 31
    }

    pub fn trap_shut(run: i32) -> i32 {
        run + 40
    }

    pub fn hand_impact(run: i32) -> i32 {
        run + 40
    }
}

/// Computes `Shadows.damageMultiplier(shadow_count)`: `1.0 + count * 0.003`.
pub fn shadow_damage_multiplier(held_shadows: u32) -> f32 {
    let clamped = held_shadows.min(KAGE_MAX_SHADOWS);
    1.0 + (clamped as f32) * KAGE_SHADOW_DAMAGE_BONUS
}

/// Computes `(max_health_bonus, movement_speed_bonus)` from held shadow count (`Shadows.applyBonuses`).
pub fn shadow_attribute_bonuses(held_shadows: u32) -> (f64, f64) {
    let n = held_shadows.min(KAGE_MAX_SHADOWS) as f64;
    let health = n * KAGE_SHADOW_HEALTH_BONUS;
    let speed = (n * KAGE_SHADOW_SPEED_BONUS).min(KAGE_SHADOW_SPEED_CAP);
    (health, speed)
}

/// Checks whether a shadowless entity is exposed to direct daylight (`Sunburn.exposed`).
pub const fn sunburn_exposed(day: bool, sees_sky: bool, wet: bool, brightness: f32) -> bool {
    day && sees_sky && !wet && brightness > 0.5
}

/// Kage-Kage no Mi module (`KageKage.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct KageKageModule;

impl KageKageModule {
    pub const fn new() -> Self {
        Self
    }

    fn make_ability(
        name: &str,
        display_name: &str,
        cast_type: CastType,
        stats: AbilityStats,
        is_ultimate: bool,
    ) -> AbilityDef {
        AbilityDef {
            id: AbilityId::new(FruitId::KageKage, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms: vec!["base".to_string(), "asgard".to_string()],
            stats,
            is_ultimate,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        }
    }

    fn brick_bat_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: BRICK_BAT_ENERGY,
            energy_per_second: BRICK_BAT_DRAIN_PER_SEC,
            cooldown_ticks: BRICK_BAT_COOLDOWN_TICKS,
            unlock_mastery: BRICK_BAT_UNLOCK_MASTERY,
            damage: BRICK_BAT_DAMAGE,
            range: BRICK_BAT_RANGE,
            knockback: BRICK_BAT_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn spear_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SHADOW_SPEAR_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SHADOW_SPEAR_COOLDOWN_TICKS,
            unlock_mastery: SHADOW_SPEAR_UNLOCK_MASTERY,
            damage: SHADOW_SPEAR_DAMAGE,
            range: SHADOW_SPEAR_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn collection_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SHADOW_COLLECTION_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SHADOW_COLLECTION_COOLDOWN_TICKS,
            unlock_mastery: SHADOW_COLLECTION_UNLOCK_MASTERY,
            damage: 0.0,
            range: SHADOW_COLLECTION_RANGE,
            knockback: 0.0,
            max_channel_ticks: SHADOW_COLLECTION_CAST_TICKS + 2,
            min_channel_ticks: 0,
            ..AbilityStats::default()
        }
    }

    fn trap_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SHADOW_TRAP_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SHADOW_TRAP_COOLDOWN_TICKS,
            unlock_mastery: SHADOW_TRAP_UNLOCK_MASTERY,
            damage: SHADOW_TRAP_DAMAGE,
            range: SHADOW_TRAP_RANGE,
            knockback: SHADOW_TRAP_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn hand_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SHADOW_HAND_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SHADOW_HAND_COOLDOWN_TICKS,
            unlock_mastery: SHADOW_HAND_UNLOCK_MASTERY,
            damage: SHADOW_HAND_DAMAGE,
            range: SHADOW_HAND_RANGE,
            knockback: SHADOW_HAND_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn doppelman_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: DOPPELMAN_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: DOPPELMAN_COOLDOWN_TICKS,
            unlock_mastery: DOPPELMAN_UNLOCK_MASTERY,
            damage: DOPPELMAN_DAMAGE,
            range: DOPPELMAN_RANGE,
            knockback: DOPPELMAN_KNOCKBACK,
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
            "tsuno_tokage" | "shadow_spear" => "spear",
            "shadow_extract" | "shadow_collection" => "collection",
            "black_box" | "shadow_trap" => "trap",
            "kage_kakumei" | "shadow_hand" => "hand",
            other => other,
        }
    }

    fn held_shadow_count(form_runtime: &FormRuntime) -> u32 {
        ((form_runtime.flags >> 16) & 0xFF).min(KAGE_MAX_SHADOWS)
    }

    fn set_held_shadow_count(form_runtime: &mut FormRuntime, count: u32) {
        let c = count.min(KAGE_MAX_SHADOWS);
        form_runtime.flags = (form_runtime.flags & 0x0000_FFFF) | (c << 16);
    }

    fn exec_brick_bat(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::brick_bat_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let shadow_mult = shadow_damage_multiplier(Self::held_shadow_count(ctx.form_runtime));

        match event {
            AbilityEvent::Toggled { enabled: true, .. } => {
                ctx.form_runtime.set_flag(KAGE_FLAG_AEGIS, true);
                res.animation_clip = Some("devilfruits:kage_kage/aegis_raise".to_string());
                ctx.world_fx.emit_fx(
                    kage_fx::AEGIS_ON,
                    0,
                    eye,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
            }
            AbilityEvent::ToggleTick { .. } => {
                ctx.form_runtime.set_flag(KAGE_FLAG_AEGIS, true);
            }
            AbilityEvent::Toggled { enabled: false, .. } | AbilityEvent::CastInstant { .. } => {
                ctx.form_runtime.set_flag(KAGE_FLAG_AEGIS, false);
                let range = stats.scaled_range(
                    ctx.player.mastery,
                    ctx.config,
                    &ctx.ability_ctx.modifiers,
                ) as f64;
                let total_dmg = stats.scaled_damage(
                    ctx.player.mastery,
                    ctx.config,
                    &ctx.ability_ctx.modifiers,
                ) * shadow_mult;

                let target = CombatTargeting::aimed(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    range,
                    BRICK_BAT_AIM_CONE_DEGREES.to_radians().max(0.45),
                )
                .or_else(|| {
                    CombatTargeting::in_radius(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        BRICK_BAT_AUTO_TARGET_RANGE,
                    )
                    .into_iter()
                    .next()
                });

                if let Some(t) = target {
                    res.animation_clip = Some("devilfruits:kage_kage/summon".to_string());
                    res.spawned_projectiles = BRICK_BAT_COUNT;
                    let away = DVec3::new(t.center.x - eye.x, 0.0, t.center.z - eye.z)
                        .normalize_or_zero();
                    let push = away * (BRICK_BAT_KNOCKBACK as f64) + DVec3::new(0.0, 0.02, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: total_dmg,
                        damage_type: GrandLineDamageType::KageShadow,
                        knockback_impulse: push,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx.emit_fx(
                        kage_fx::SWARM,
                        0,
                        t.center,
                        DVec3::ZERO,
                        BRICK_BAT_COUNT as f32,
                        ctx.ability_ctx.tick,
                        1,
                        64.0,
                    );
                }
                ctx.world_fx.emit_fx(
                    kage_fx::AEGIS_OFF,
                    0,
                    eye,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    if res.hits.is_empty() { 0 } else { 1 },
                    64.0,
                );
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_spear(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::spear_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * shadow_damage_multiplier(Self::held_shadow_count(ctx.form_runtime));
        let stun_ticks = (SHADOW_SPEAR_STUN_SECONDS * 20.0).round() as i32;

        res.animation_clip = Some("devilfruits:kage_kage/point_thrust".to_string());

        let target = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            SHADOW_SPEAR_AIM_CONE_DEGREES.to_radians().max(0.45),
        )
        .or_else(|| {
            CombatTargeting::in_cone(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.5)
                .into_iter()
                .next()
        });

        if let Some(t) = target {
            let run = KageTiming::spear_run(t.center.distance(eye));
            ctx.world_fx.emit_fx(
                kage_fx::SPEAR,
                0,
                t.center,
                DVec3::new(run as f64, stun_ticks as f64, 0.0),
                SHADOW_SPEAR_BATS,
                ctx.ability_ctx.tick,
                KageTiming::spear_pierce(run),
                64.0,
            );
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::KageShadow,
                knockback_impulse: DVec3::ZERO,
                stun_ticks,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_collection(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::collection_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;

        let target = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            SHADOW_COLLECTION_AIM_CONE_DEGREES.to_radians().max(0.5),
        )
        .or_else(|| {
            CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, range)
                .into_iter()
                .next()
        });

        match event {
            AbilityEvent::ChannelStarted { .. } => {
                res.animation_clip = Some("devilfruits:kage_kage/snip_open".to_string());
                if let Some(t) = target {
                    ctx.world_fx.emit_fx(
                        kage_fx::COLLECT,
                        0,
                        t.center,
                        DVec3::ZERO,
                        1.0,
                        ctx.ability_ctx.tick,
                        0,
                        64.0,
                    );
                }
            }
            AbilityEvent::ChannelTick { elapsed_ticks, .. } => {
                if *elapsed_ticks >= SHADOW_COLLECTION_CAST_TICKS {
                    let cur = Self::held_shadow_count(ctx.form_runtime);
                    if cur < KAGE_MAX_SHADOWS {
                        Self::set_held_shadow_count(ctx.form_runtime, cur + 1);
                    }
                    res.animation_clip = Some("devilfruits:kage_kage/snip_cut".to_string());
                    if let Some(t) = target {
                        ctx.world_fx.emit_fx(
                            kage_fx::COLLECT_SNIP,
                            0,
                            t.center,
                            DVec3::ZERO,
                            1.0,
                            ctx.ability_ctx.tick,
                            Self::held_shadow_count(ctx.form_runtime) as i32,
                            64.0,
                        );
                    }
                }
            }
            AbilityEvent::CastInstant { .. } | AbilityEvent::ChannelEnded { .. } => {
                let cur = Self::held_shadow_count(ctx.form_runtime);
                if cur < KAGE_MAX_SHADOWS && target.is_some() {
                    Self::set_held_shadow_count(ctx.form_runtime, cur + 1);
                }
                res.animation_clip = Some("devilfruits:kage_kage/snip_cut".to_string());
                if let Some(t) = target {
                    ctx.world_fx.emit_fx(
                        kage_fx::COLLECT_SNIP,
                        0,
                        t.center,
                        DVec3::ZERO,
                        1.0,
                        ctx.ability_ctx.tick,
                        Self::held_shadow_count(ctx.form_runtime) as i32,
                        64.0,
                    );
                }
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_trap(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::trap_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * shadow_damage_multiplier(Self::held_shadow_count(ctx.form_runtime));
        let hold_ticks = (SHADOW_TRAP_HOLD_SECONDS * 20.0).round() as i32;

        res.animation_clip = Some("devilfruits:kage_kage/palm_press".to_string());

        let victim = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            0.5,
        )
        .or_else(|| {
            CombatTargeting::in_cone(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.55)
                .into_iter()
                .next()
        });

        if let Some(t) = victim {
            let run = KageTiming::bat_run(t.center.distance(eye));
            let spit_stun = 9 + hold_ticks + 7 + 2;
            let front = (eye - t.center).normalize_or_zero();
            let push = DVec3::new(front.x, 0.0, front.z).normalize_or_zero()
                * (SHADOW_TRAP_KNOCKBACK as f64)
                + DVec3::new(0.0, 0.35, 0.0);

            ctx.world_fx.emit_fx(
                kage_fx::TRAP,
                0,
                t.center,
                DVec3::new(run as f64, hold_ticks as f64, 0.0),
                1.0,
                ctx.ability_ctx.tick,
                hold_ticks,
                64.0,
            );
            ctx.world_fx.emit_fx(
                kage_fx::TRAP_SWALLOW,
                0,
                t.center,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                hold_ticks,
                64.0,
            );

            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::KageShadow,
                knockback_impulse: push,
                stun_ticks: spit_stun,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_hand(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::hand_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * shadow_damage_multiplier(Self::held_shadow_count(ctx.form_runtime));

        res.animation_clip = Some("devilfruits:kage_kage/crouch_ground".to_string());

        let spot = ctx
            .ability_ctx
            .aim_point
            .or_else(|| {
                CombatTargeting::aimed(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.5)
                    .map(|t| t.center)
            })
            .unwrap_or(eye + look * 8.0);

        let fall = DVec3::new(spot.x - eye.x, 0.0, spot.z - eye.z).normalize_or_zero();
        let run = KageTiming::bat_run(spot.distance(eye));
        ctx.world_fx.emit_fx(
            kage_fx::HAND,
            0,
            spot,
            fall * SHADOW_HAND_HEIGHT,
            SHADOW_HAND_BATS,
            ctx.ability_ctx.tick,
            KageTiming::hand_impact(run),
            64.0,
        );

        for t in CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, spot, SHADOW_HAND_RADIUS)
        {
            let mut away = DVec3::new(t.center.x - spot.x, 0.0, t.center.z - spot.z);
            away = if away.length_squared() < 0.04 {
                fall
            } else {
                away.normalize()
            };
            let push = away * ((SHADOW_HAND_KNOCKBACK as f64) * 0.7) + DVec3::new(0.0, 0.35, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::KageShadow,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_doppelman(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::doppelman_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers)
            * shadow_damage_multiplier(Self::held_shadow_count(ctx.form_runtime));

        let already_out = ctx.form_runtime.has_flag(KAGE_FLAG_DOPPELMAN_OUT);
        if !already_out {
            ctx.form_runtime.set_flag(KAGE_FLAG_DOPPELMAN_OUT, true);
            res.animation_clip = Some("devilfruits:kage_kage/summon".to_string());
            res.spawned_projectiles = 1;
            let right = DVec3::new(-look.z, 0.0, look.x).normalize_or_zero();
            let spot = eye + right * 1.6;
            ctx.world_fx.emit_fx(
                kage_fx::DOPPEL_POOL,
                0,
                spot,
                DVec3::ZERO,
                1.3,
                ctx.ability_ctx.tick,
                48,
                64.0,
            );
        } else {
            // Command active Doppelman to attack aimed target or swap/recall
            if let Some(t) = CombatTargeting::aimed(
                ctx.nearby_targets,
                ctx.player_id,
                eye,
                look,
                range,
                0.5,
            ) {
                let push = look * (DOPPELMAN_KNOCKBACK as f64) + DVec3::new(0.0, 0.15, 0.0);
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: dmg,
                    damage_type: GrandLineDamageType::KageShadow,
                    knockback_impulse: push,
                    stun_ticks: 0,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
                ctx.world_fx.emit_fx(
                    kage_fx::DOPPEL_ORDER,
                    0,
                    t.center,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    1,
                    64.0,
                );
            } else {
                // Swap places with Doppelman through shadow pools
                let swap_offset = look * 6.0;
                res.caster_velocity_impulse = Some(swap_offset);
                ctx.world_fx.emit_fx(
                    kage_fx::DOPPEL_POOL,
                    0,
                    eye,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    14,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    kage_fx::DOPPEL_POOL,
                    0,
                    eye + swap_offset,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick ^ 25_214_903_917,
                    14,
                    64.0,
                );
            }
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_shadows_asgard(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let entering = ctx.player.active_form != "asgard";
        if entering {
            ctx.player.active_form = "asgard".to_string();
            ctx.form_runtime.set_flag(KAGE_FLAG_ASGARD, true);
            if Self::held_shadow_count(ctx.form_runtime) == 0 {
                Self::set_held_shadow_count(ctx.form_runtime, KAGE_MAX_SHADOWS);
            }
            res.form_switch_to = Some("asgard".to_string());
            res.animation_clip = Some("devilfruits:kage_kage/summon".to_string());
            ctx.world_fx.emit_fx(
                kage_fx::DOPPEL_POOL,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                2.5,
                ctx.ability_ctx.tick,
                60,
                96.0,
            );
            ctx.world_fx.emit_shake(ctx.ability_ctx.eye_pos, 0.85, 32);
            res.screen_shake = Some((0.85, 32));
        } else {
            ctx.player.active_form = "base".to_string();
            ctx.form_runtime.set_flag(KAGE_FLAG_ASGARD, false);
            res.form_switch_to = Some("base".to_string());
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }
}

impl DevilFruitModule for KageKageModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::KageKage
    }

    fn forms(&self) -> Vec<FruitFormDef> {
        let mut forms = FruitFormDef::builtin_forms(FruitId::KageKage);
        let all_abilities = vec![
            "brick_bat".to_string(),
            "spear".to_string(),
            "collection".to_string(),
            "trap".to_string(),
            "hand".to_string(),
            "doppelman".to_string(),
            "tsuno_tokage".to_string(),
            "shadow_extract".to_string(),
            "black_box".to_string(),
            "kage_kakumei".to_string(),
            "shadows_asgard".to_string(),
        ];
        if let Some(base) = forms.first_mut() {
            base.abilities = all_abilities.clone();
        }
        let (max_hp_bonus, speed_bonus) = shadow_attribute_bonuses(KAGE_MAX_SHADOWS);
        forms.push(FruitFormDef {
            id: FormId::new(FruitId::KageKage, "asgard"),
            is_base: false,
            stats: FormStats {
                unlock_mastery: 8,
                enter_cost: 50.0,
                drain_per_second: 2.0,
                cooldown_ticks: 1200,
                cast_ticks: 30,
                energy_regen_multiplier: 0.5,
                time_limit_seconds: Some(60.0),
                ..FormStats::default()
            },
            collision: FormCollision::default(),
            attributes: FormAttributeBonuses {
                scale_add: 1.5,
                max_health_add: max_hp_bonus,
                attack_damage_add: 8.0,
                armor_add: 8.0,
                knockback_resistance_add: 0.8,
                movement_speed_mult: speed_bonus,
                ..FormAttributeBonuses::default()
            },
            abilities: all_abilities,
            ultimate_ability: Some("shadows_asgard".to_string()),
        });
        forms
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        vec![
            Self::make_ability(
                "brick_bat",
                "Brick Bat",
                CastType::Toggle,
                Self::brick_bat_stats(),
                false,
            ),
            Self::make_ability(
                "spear",
                "Tsuno-Tokage (Shadow Spear)",
                CastType::Instant,
                Self::spear_stats(),
                false,
            ),
            Self::make_ability(
                "collection",
                "Shadow Extract (Shadow Collection)",
                CastType::Held,
                Self::collection_stats(),
                false,
            ),
            Self::make_ability(
                "trap",
                "Black Box (Shadow Trap)",
                CastType::Instant,
                Self::trap_stats(),
                false,
            ),
            Self::make_ability(
                "hand",
                "Kage Kakumei (Shadow Hand)",
                CastType::Instant,
                Self::hand_stats(),
                false,
            ),
            Self::make_ability(
                "doppelman",
                "Doppelman",
                CastType::Instant,
                Self::doppelman_stats(),
                true,
            ),
            // Spec aliases
            Self::make_ability(
                "tsuno_tokage",
                "Tsuno-Tokage",
                CastType::Instant,
                Self::spear_stats(),
                false,
            ),
            Self::make_ability(
                "shadow_extract",
                "Shadow Extract",
                CastType::Held,
                Self::collection_stats(),
                false,
            ),
            Self::make_ability(
                "black_box",
                "Black Box",
                CastType::Instant,
                Self::trap_stats(),
                false,
            ),
            Self::make_ability(
                "kage_kakumei",
                "Kage Kakumei",
                CastType::Instant,
                Self::hand_stats(),
                false,
            ),
            Self::make_ability(
                "shadows_asgard",
                "Shadows Asgard",
                CastType::FormSwitch,
                Self::doppelman_stats(),
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
            damage_type: Some(GrandLineDamageType::KageShadow),
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
            "brick_bat" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_brick_bat(ctx, event, &mut res);
            }
            "spear" => {
                self.exec_spear(ctx, &mut res);
            }
            "collection" => {
                if matches!(
                    event,
                    AbilityEvent::ChannelStarted { .. } | AbilityEvent::ChannelTick { .. }
                ) {
                    res.cast_result = Some(CastResult::Channelling);
                }
                self.exec_collection(ctx, event, &mut res);
            }
            "trap" => {
                self.exec_trap(ctx, &mut res);
            }
            "hand" => {
                self.exec_hand(ctx, &mut res);
            }
            "doppelman" => {
                self.exec_doppelman(ctx, &mut res);
            }
            "shadows_asgard" => {
                self.exec_shadows_asgard(ctx, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed {
            ctx.form_runtime.set_flag(KAGE_FLAG_AEGIS, false);
            if ctx.player.active_form == "asgard" {
                ctx.player.active_form = "base".to_string();
                ctx.form_runtime.set_flag(KAGE_FLAG_ASGARD, false);
                res.form_switch_to = Some("base".to_string());
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
        if player.active_form == "asgard" || form_runtime.has_flag(KAGE_FLAG_ASGARD) {
            mods.mass_tier = EntityMassTier::Heavy;
            mods.mass_kg_override = Some(300.0);
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
    fn kage_abilities_shadow_scaling_and_sunburn_match_java() {
        let m = KageKageModule::new();
        assert_eq!(m.fruit_id(), FruitId::KageKage);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::KageShadow);

        let bb = m.ability("brick_bat").unwrap();
        assert_eq!(bb.cast_type, CastType::Toggle);
        assert_eq!(bb.stats.energy_cost, 6.0);
        assert_eq!(bb.stats.cooldown_ticks, 240);
        assert_eq!(bb.stats.damage, 20.0);

        let spear = m.ability("tsuno_tokage").unwrap();
        assert_eq!(spear.stats.energy_cost, 18.0);
        assert_eq!(spear.stats.cooldown_ticks, 200);
        assert_eq!(spear.stats.damage, 9.0);

        let extract = m.ability("shadow_extract").unwrap();
        assert_eq!(extract.cast_type, CastType::Held);
        assert_eq!(extract.stats.energy_cost, 25.0);
        assert_eq!(extract.stats.unlock_mastery, 3);

        let box_ab = m.ability("black_box").unwrap();
        assert_eq!(box_ab.stats.energy_cost, 30.0);
        assert_eq!(box_ab.stats.damage, 18.0);
        assert_eq!(box_ab.stats.unlock_mastery, 4);

        let hand = m.ability("kage_kakumei").unwrap();
        assert_eq!(hand.stats.energy_cost, 35.0);
        assert_eq!(hand.stats.damage, 12.0);
        assert_eq!(hand.stats.unlock_mastery, 6);

        let doppel = m.ability("doppelman").unwrap();
        assert!(doppel.is_ultimate);
        assert_eq!(doppel.stats.energy_cost, 50.0);
        assert_eq!(doppel.stats.cooldown_ticks, 1200);
        assert_eq!(doppel.stats.unlock_mastery, 8);

        assert!((shadow_damage_multiplier(100) - 1.3).abs() < 1e-4);
        let (hp, spd) = shadow_attribute_bonuses(100);
        assert!((hp - 20.0).abs() < 1e-6);
        assert!((spd - 0.03).abs() < 1e-6);
        assert!(sunburn_exposed(true, true, false, 0.8));
        assert!(!sunburn_exposed(true, true, true, 0.8));
    }

    #[test]
    fn kage_collection_scales_damage_and_doppelman_asgard_work() {
        let m = KageKageModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::KageKage),
            mastery: 9,
            energy: 250.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut wfx = WorldFxManager::default();
        let targets = [test_target(2, DVec3::new(0.0, 0.0, 4.0))];

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
                fruit: FruitId::KageKage,
                ability_name: "tsuno_tokage".into(),
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

        let spear_base = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "tsuno_tokage".into(),
                mastery: 9,
            },
        );
        assert_eq!(spear_base.hits.len(), 1);
        assert_eq!(spear_base.hits[0].stun_ticks, 50);
        let base_dmg = spear_base.hits[0].damage;

        // Enter Shadows Asgard (grants 100 shadows)
        let asgard_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "shadows_asgard".into(),
            },
        );
        assert_eq!(asgard_res.form_switch_to.as_deref(), Some("asgard"));
        let mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert_eq!(mods.mass_tier, EntityMassTier::Heavy);

        let spear_boosted = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "tsuno_tokage".into(),
                mastery: 9,
            },
        );
        assert!(spear_boosted.hits[0].damage > base_dmg * 1.25);

        // Summon Doppelman then command attack
        let d_summon = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "doppelman".into(),
                mastery: 9,
            },
        );
        assert_eq!(d_summon.spawned_projectiles, 1);

        let d_cmd = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "doppelman".into(),
                mastery: 9,
            },
        );
        assert_eq!(d_cmd.hits.len(), 1);
    }
}
