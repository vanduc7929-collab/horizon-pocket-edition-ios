//! Bomu Bomu no Mi (Bomb-Bomb Fruit) — 1:1 port of `com.devilfruits.fruits.bomu_bomu`.
//!
//! Includes `nose_fancy_cannon` (`cannon`), `kick_bomb` (`punch` / `explosive_punch`),
//! `breeze_breath_bomb` (`shot` / `explosive_shot`), `zenshin_kibaku` (`self_destruct`),
//! 3-tier self-detonation charge scaling, vanish & reform state, explosion immunity,
//! and self-detonation spherical craters.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::core::world_fx::core_fx;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Java constants (`BomuBomu.java`, `ability/*.java`, `explosion/*.java`)
// ---------------------------------------------------------------------------

/// `BomuBomu.java` global explosion settings.
pub const BOMU_MAX_BLOCKS_PER_EXPLOSION: usize = 220;
pub const BOMU_MAX_DEBRIS_PER_EXPLOSION: usize = 14;
pub const BOMU_MAX_EXPLOSIONS_PER_TICK_PER_PLAYER: usize = 4;
pub const BOMU_ABSORB_FX_COOLDOWN_TICKS: i64 = 30;
pub const BOMU_PISTOL_ROUNDS: i32 = 6;
pub const BOMU_PISTOL_RELOAD_SECONDS: f32 = 2.0;
pub const BOMU_GUST_KNOCKBACK: f64 = 0.6;
pub const BOMU_IMMUNE_TO_KNOCKBACK: bool = true;

/// `BomuBomu.ARMED` & `BomuBomu.VANISHED` form flag bits.
pub const BOMU_FLAG_ARMED: u32 = form_flags::fruit_bit(0);
pub const BOMU_FLAG_VANISHED: u32 = form_flags::fruit_bit(1);

/// `ExplosivePunchAbility.java` (`punch` / `explosive_punch` / `kick_bomb`) constants.
pub const PUNCH_ENERGY: f32 = 14.0;
pub const PUNCH_COOLDOWN_TICKS: i32 = 30;
pub const PUNCH_UNLOCK_MASTERY: u32 = 1;
pub const PUNCH_DAMAGE: f32 = 10.0;
pub const PUNCH_RANGE: f32 = 2.8;
pub const PUNCH_KNOCKBACK: f32 = 1.1;
pub const PUNCH_ARM_SECONDS: f32 = 8.0;
pub const PUNCH_MAX_RADIUS: f32 = 4.2;
pub const PUNCH_TARGET_KNOCKBACK: f64 = 0.9;
pub const PUNCH_MAX_PLAYER_DAMAGE: f32 = 14.0;

/// `NoseFancyCannonAbility.java` (`cannon` / `nose_fancy_cannon`) constants.
pub const CANNON_ENERGY: f32 = 5.0;
pub const CANNON_COOLDOWN_TICKS: i32 = 300;
pub const CANNON_UNLOCK_MASTERY: u32 = 2;
pub const CANNON_DAMAGE: f32 = 5.0;
pub const CANNON_RANGE: f32 = 1.8;
pub const CANNON_KNOCKBACK: f32 = 0.5;
pub const CANNON_SHOT_SPEED: f64 = 1.5;
pub const CANNON_FUSE_TICKS: i32 = 3;
pub const CANNON_MAX_PLAYER_DAMAGE: f32 = 6.0;
pub const BOOGER_BOMB_GRAVITY: f64 = 0.03;
pub const BOOGER_BOMB_MAX_LIFE_TICKS: i32 = 80;

/// `ExplosiveShotAbility.java` (`shot` / `explosive_shot` / `breeze_breath_bomb`) constants.
pub const SHOT_ENERGY: f32 = 14.0;
pub const SHOT_COOLDOWN_TICKS: i32 = 16;
pub const SHOT_UNLOCK_MASTERY: u32 = 4;
pub const SHOT_DAMAGE: f32 = 16.0;
pub const SHOT_RANGE: f32 = 4.0;
pub const SHOT_KNOCKBACK: f32 = 1.3;
pub const SHOT_BULLET_SPEED: f64 = 5.0;
pub const SHOT_MAX_DISTANCE: f64 = 96.0;
pub const SHOT_MAX_RADIUS: f32 = 6.5;
pub const SHOT_MAX_PLAYER_DAMAGE: f32 = 18.0;
pub const BOMB_BULLET_GRAVITY: f64 = 0.004;

/// `SelfDestructAbility.java` (`self_destruct` / `zenshin_kibaku`) constants.
pub const SELF_DESTRUCT_ENERGY: f32 = 130.0;
pub const SELF_DESTRUCT_COOLDOWN_TICKS: i32 = 1200;
pub const SELF_DESTRUCT_UNLOCK_MASTERY: u32 = 6;
pub const SELF_DESTRUCT_DAMAGE: f32 = 30.0;
pub const SELF_DESTRUCT_RANGE: f32 = 8.0;
pub const SELF_DESTRUCT_KNOCKBACK: f32 = 2.6;
pub const SELF_DESTRUCT_CHARGE_SECONDS: f32 = 2.5;
pub const SELF_DESTRUCT_CHARGE_TICKS: i32 = 50;
pub const SELF_DESTRUCT_TIER1_THRESHOLD: f32 = 0.1;
pub const SELF_DESTRUCT_TIER2_THRESHOLD: f32 = 0.5;
pub const SELF_DESTRUCT_MAX_RADIUS: f32 = 12.0;
pub const SELF_DESTRUCT_VANISH_SECONDS: f32 = 1.5;
pub const SELF_DESTRUCT_INVULNERABLE_SECONDS: f32 = 0.6;
pub const SELF_DESTRUCT_FIRE_CHANCE: f32 = 0.06;
pub const SELF_DESTRUCT_MAX_PLAYER_DAMAGE: f32 = 40.0;
pub const SELF_DESTRUCT_TIER_COSTS: [f32; 3] = [0.25, 0.55, 1.0];
pub const SELF_DESTRUCT_TIER_COOLDOWNS: [f32; 3] = [0.15, 0.45, 1.0];

/// Canonical Bomu-Bomu VFX identifiers (`BomuFx.java`).
pub mod bomu_fx {
    pub const AWAKEN: &str = "devilfruits:bomu_bomu/awaken";
    pub const ABSORB: &str = "devilfruits:bomu_bomu/absorb";
    pub const PUNCH_ARM: &str = "devilfruits:bomu_bomu/punch_arm";
    pub const PUNCH_FIZZLE: &str = "devilfruits:bomu_bomu/punch_fizzle";
    pub const CANNON_FIRE: &str = "devilfruits:bomu_bomu/cannon_fire";
    pub const SPLAT: &str = "devilfruits:bomu_bomu/splat";
    pub const SHOT_FIRE: &str = "devilfruits:bomu_bomu/shot_fire";
    pub const BREATH: &str = "devilfruits:bomu_bomu/breath";
    pub const GUST: &str = "devilfruits:bomu_bomu/gust";
    pub const SELF_DESTRUCT_TIER: &str = "devilfruits:bomu_bomu/self_destruct_tier";
    pub const SELF_DESTRUCT_VANISH: &str = "devilfruits:bomu_bomu/self_destruct_vanish";
    pub const SELF_DESTRUCT_REFORM: &str = "devilfruits:bomu_bomu/self_destruct_reform";
}

/// Bomu Bomu no Mi module (`com.devilfruits.fruits.bomu_bomu.BomuBomu`).
#[derive(Clone, Copy, Debug, Default)]
pub struct BomuBomuModule;

impl BomuBomuModule {
    pub const fn new() -> Self {
        Self
    }

    /// True if the Bomu-Bomu user is immune to explosion damage and knockback (`BomuBomu.immune`).
    pub const fn is_explosion_immune(has_bomu_fruit: bool, suppressed: bool) -> bool {
        has_bomu_fruit && !suppressed
    }

    /// Evaluates self-destruct tier (`0..=3`) for a given charge fraction (`SelfDestructAbility.tier`).
    pub fn self_destruct_tier(charge: f32) -> u8 {
        if charge >= 0.999 {
            3
        } else if charge >= SELF_DESTRUCT_TIER2_THRESHOLD {
            2
        } else if charge >= SELF_DESTRUCT_TIER1_THRESHOLD {
            1
        } else {
            0
        }
    }

    /// Exact tier-scaled energy cost for `self_destruct` / `zenshin_kibaku` (`SelfDestructAbility.energyCost`).
    pub fn self_destruct_energy_cost(charge: f32) -> f32 {
        let t = Self::self_destruct_tier(charge);
        if t == 0 {
            0.0
        } else {
            SELF_DESTRUCT_ENERGY * SELF_DESTRUCT_TIER_COSTS[(t - 1) as usize]
        }
    }

    /// Exact tier-scaled cooldown ticks for `self_destruct` / `zenshin_kibaku` (`SelfDestructAbility.cooldownTicks`).
    pub fn self_destruct_cooldown_ticks(charge: f32) -> i32 {
        let t = Self::self_destruct_tier(charge);
        if t == 0 {
            0
        } else {
            ((SELF_DESTRUCT_COOLDOWN_TICKS as f32) * SELF_DESTRUCT_TIER_COOLDOWNS[(t - 1) as usize])
                .round() as i32
        }
    }

    /// Exact blast damage falloff curve (`BombExplosionService.hurtEntities`):
    /// `f = max(0, 1 - d / radius)`, `damage = min(max_player, entity_damage * (f * f + f) * 0.5)`.
    pub fn explosion_damage_at_distance(
        base_damage: f32,
        distance: f64,
        radius: f64,
        max_player_damage: f32,
    ) -> f32 {
        if radius <= 0.0 || distance >= radius {
            return 0.0;
        }
        let f = (1.0 - distance / radius).max(0.0) as f32;
        let raw = base_damage * (f * f + f) * 0.5;
        raw.min(max_player_damage)
    }
}

impl DevilFruitModule for BomuBomuModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::BomuBomu
    }

    fn primary_damage_type(&self) -> GrandLineDamageType {
        GrandLineDamageType::BomuExplosion
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_form = vec!["base".to_string()];

        let make_punch = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::BomuBomu, name),
            display_name: display.into(),
            cast_type: CastType::Instant,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: PUNCH_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: PUNCH_COOLDOWN_TICKS,
                unlock_mastery: PUNCH_UNLOCK_MASTERY,
                damage: PUNCH_DAMAGE,
                range: PUNCH_RANGE,
                knockback: PUNCH_KNOCKBACK,
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_cannon = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::BomuBomu, name),
            display_name: display.into(),
            cast_type: CastType::Instant,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: CANNON_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: CANNON_COOLDOWN_TICKS,
                unlock_mastery: CANNON_UNLOCK_MASTERY,
                damage: CANNON_DAMAGE,
                range: CANNON_RANGE,
                knockback: CANNON_KNOCKBACK,
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_shot = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::BomuBomu, name),
            display_name: display.into(),
            cast_type: CastType::Instant,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: SHOT_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: SHOT_COOLDOWN_TICKS,
                unlock_mastery: SHOT_UNLOCK_MASTERY,
                damage: SHOT_DAMAGE,
                range: SHOT_RANGE,
                knockback: SHOT_KNOCKBACK,
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_self_destruct = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::BomuBomu, name),
            display_name: display.into(),
            cast_type: CastType::Charged,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: SELF_DESTRUCT_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: SELF_DESTRUCT_COOLDOWN_TICKS,
                unlock_mastery: SELF_DESTRUCT_UNLOCK_MASTERY,
                damage: SELF_DESTRUCT_DAMAGE,
                range: SELF_DESTRUCT_RANGE,
                knockback: SELF_DESTRUCT_KNOCKBACK,
                max_charge_ticks: SELF_DESTRUCT_CHARGE_TICKS,
                ..AbilityStats::default()
            },
            is_ultimate: true,
            stops_movement_while_charging: true,
            allows_cancel_on_hurt: false,
        };

        vec![
            make_punch("punch", "Explosive Punch"),
            make_punch("explosive_punch", "Explosive Punch"),
            make_punch("kick_bomb", "Kick Bomb"),
            make_cannon("cannon", "Nose Fancy Cannon"),
            make_cannon("nose_fancy_cannon", "Nose Fancy Cannon"),
            make_shot("shot", "Explosive Shot"),
            make_shot("explosive_shot", "Explosive Shot"),
            make_shot("breeze_breath_bomb", "Breeze Breath Bomb"),
            make_self_destruct("self_destruct", "Zenshin Kibaku"),
            make_self_destruct("zenshin_kibaku", "Zenshin Kibaku"),
        ]
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::BomuExplosion),
            ..AbilityExecutionResult::default()
        };

        let mastery = ctx.player.mastery;
        let mods = ctx.ability_ctx.modifiers;
        let eye = ctx.ability_ctx.eye_pos;
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let look = if look.length_squared() < 1e-8 {
            DVec3::Z
        } else {
            look
        };
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let flat = if flat.length_squared() < 1e-8 {
            DVec3::Z
        } else {
            flat
        };
        let now = ctx.ability_ctx.tick;

        match event {
            AbilityEvent::CastInstant { ability, .. }
                if matches!(
                    ability.as_str(),
                    "punch" | "explosive_punch" | "kick_bomb"
                ) =>
            {
                let Some(def) = self.ability("punch") else {
                    return res;
                };
                // ExplosivePunchAbility: arms the fist (BOMU_FLAG_ARMED) and if a target is within melee/blast range, detonates immediately!
                ctx.form_runtime.set_flag(BOMU_FLAG_ARMED, true);
                ctx.world_fx
                    .emit_fx(bomu_fx::PUNCH_ARM, 0, eye, look, 1.0, now, 0, 64.0);
                res.animation_clip = Some("devilfruits:bomu_bomu/punch_arm".into());

                let radius = def
                    .stats
                    .scaled_range(mastery, ctx.config, &mods)
                    .min(PUNCH_MAX_RADIUS) as f64;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = (def.stats.knockback * 0.7) as f64;
                let blast_center = eye + look * 1.4;

                let nearby = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    blast_center,
                    radius * 1.25,
                );
                if !nearby.is_empty() {
                    ctx.form_runtime.set_flag(BOMU_FLAG_ARMED, false);
                    res.animation_clip = Some("devilfruits:bomu_bomu/punch_hit".into());
                    ctx.world_fx.emit_fx(
                        core_fx::EXPLOSION,
                        0,
                        blast_center,
                        DVec3::Y,
                        radius as f32,
                        now,
                        0,
                        64.0,
                    );
                    for target in nearby {
                        let d = target.center.distance(blast_center);
                        let dmg = Self::explosion_damage_at_distance(
                            base_dmg,
                            d,
                            radius,
                            PUNCH_MAX_PLAYER_DAMAGE,
                        );
                        let push_scale = (1.0 - d / (radius * 1.25)).max(0.0);
                        let away = DVec3::new(
                            target.center.x - eye.x,
                            0.0,
                            target.center.z - eye.z,
                        )
                        .normalize_or_zero();
                        let dir = if away.length_squared() < 1e-4 {
                            flat
                        } else {
                            away
                        };
                        let push = dir * (kb * push_scale + PUNCH_TARGET_KNOCKBACK)
                            + DVec3::new(0.0, 0.35, 0.0);
                        if dmg > 0.0 {
                            res.hits.push(AbilityHitEffect {
                                target_id: target.id,
                                damage: dmg,
                                damage_type: GrandLineDamageType::BomuExplosion,
                                knockback_impulse: CombatTargeting::launch_velocity(
                                    push,
                                    target.knockback_resistance,
                                ),
                                stun_ticks: 0,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }

                    ctx.budget.blocks_remaining =
                        ctx.budget.blocks_remaining.min(BOMU_MAX_BLOCKS_PER_EXPLOSION);
                    ctx.budget.debris_remaining =
                        ctx.budget.debris_remaining.min(BOMU_MAX_DEBRIS_PER_EXPLOSION);
                    res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                        blast_center,
                        radius * 0.8,
                        &mut ctx.budget,
                        false,
                        now,
                        ctx.config,
                        ctx.game_rules,
                        |_pos| ("minecraft:stone".to_string(), 1.5, false),
                    );
                }

                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. }
                if matches!(ability.as_str(), "cannon" | "nose_fancy_cannon") =>
            {
                let Some(def) = self.ability("cannon") else {
                    return res;
                };
                // NoseFancyCannonAbility: shoots BoogerBomb from nose at speed 1.5
                let nose = eye + DVec3::new(0.0, -0.12, 0.0) + flat * 0.25;
                let velocity = look * CANNON_SHOT_SPEED;
                res.spawned_projectiles = 1;
                res.animation_clip = Some("devilfruits:bomu_bomu/nose_flick".into());
                ctx.world_fx
                    .emit_fx(bomu_fx::CANNON_FIRE, 0, nose, velocity, 1.0, now, 0, 64.0);

                let radius = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let impact_pos = ctx.ability_ctx.aim_point.unwrap_or(nose + look * 8.0);

                for target in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    nose,
                    impact_pos,
                    radius,
                ) {
                    let d = target.center.distance(impact_pos).min(radius * 0.5);
                    let dmg = Self::explosion_damage_at_distance(
                        base_dmg,
                        d,
                        radius,
                        CANNON_MAX_PLAYER_DAMAGE,
                    )
                    .max(base_dmg * 0.5);
                    let dir = (target.center - nose).normalize_or_zero();
                    let push = dir * kb + DVec3::new(0.0, kb * 0.25, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::BomuExplosion,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            target.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx
                        .emit_fx(bomu_fx::SPLAT, 0, target.center, -look, 1.0, now, 0, 48.0);
                }

                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. }
                if matches!(
                    ability.as_str(),
                    "shot" | "explosive_shot" | "breeze_breath_bomb"
                ) =>
            {
                let Some(def) = self.ability("shot") else {
                    return res;
                };
                // ExplosiveShotAbility: BombBullet breath shot at 5.0 blocks/tick up to 96 blocks
                let side = DVec3::new(-look.z, 0.0, look.x).normalize_or_zero() * 0.24;
                let muzzle = eye + look * 0.95 + side + DVec3::new(0.0, -0.14, 0.0);
                res.spawned_projectiles = 1;
                res.animation_clip = Some("devilfruits:bomu_bomu/pistol_fire".into());
                ctx.world_fx
                    .emit_fx(bomu_fx::SHOT_FIRE, 0, muzzle, look, 1.0, now, 1, 64.0);

                let radius = def
                    .stats
                    .scaled_range(mastery, ctx.config, &mods)
                    .min(SHOT_MAX_RADIUS) as f64;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let end = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(muzzle + look * 16.0);

                for target in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    muzzle,
                    end,
                    radius,
                ) {
                    let dmg = base_dmg.min(SHOT_MAX_PLAYER_DAMAGE);
                    let push = look * kb + DVec3::new(0.0, kb * 0.25, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::BomuExplosion,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            target.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 2,
                    });
                }

                ctx.budget.blocks_remaining =
                    ctx.budget.blocks_remaining.min(BOMU_MAX_BLOCKS_PER_EXPLOSION);
                ctx.budget.debris_remaining =
                    ctx.budget.debris_remaining.min(BOMU_MAX_DEBRIS_PER_EXPLOSION);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    end,
                    radius * 0.75,
                    &mut ctx.budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_pos| ("minecraft:stone".to_string(), 1.5, false),
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::ChargeStarted { ability, .. }
                if ability == "self_destruct" || ability == "zenshin_kibaku" =>
            {
                res.cast_result = Some(CastResult::Charging);
                res.animation_clip = Some("devilfruits:bomu_bomu/self_destruct_charge".into());
            }
            AbilityEvent::ChargeReleased { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "self_destruct" || ability == "zenshin_kibaku" =>
            {
                let c = match event {
                    AbilityEvent::ChargeReleased { charge, .. } => charge.clamp(0.0, 1.0),
                    _ => ctx.ability_ctx.charge.clamp(0.0, 1.0),
                };
                let tier = Self::self_destruct_tier(c);
                if tier == 0 {
                    return res;
                }
                let Some(def) = self.ability("self_destruct") else {
                    return res;
                };
                let full_r = def
                    .stats
                    .scaled_range(mastery, ctx.config, &mods)
                    .min(SELF_DESTRUCT_MAX_RADIUS);
                let radius = match tier {
                    1 => full_r.min(2.5),
                    2 => full_r.min(5.0),
                    _ => full_r,
                } as f64;
                let share = match tier {
                    1 => 0.3,
                    2 => 0.6,
                    _ => 1.0,
                };
                let block_scale = if tier == 1 { 0.5 } else { 1.0 };
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods) * share;
                let kb = (def.stats.knockback * share) as f64;
                let vanish_ticks = (SELF_DESTRUCT_VANISH_SECONDS * 20.0).round() as i32;

                ctx.form_runtime.set_flag(BOMU_FLAG_VANISHED, true);
                res.animation_clip = Some("devilfruits:bomu_bomu/self_destruct_blast".into());
                ctx.world_fx.emit_fx(
                    bomu_fx::SELF_DESTRUCT_VANISH,
                    0,
                    eye,
                    DVec3::ZERO,
                    tier as f32,
                    now,
                    vanish_ticks,
                    96.0,
                );
                ctx.world_fx.emit_shake(eye, 0.5 * (tier as f32), (radius as i32) * 3);
                res.screen_shake = Some((0.5 * (tier as f32), (radius as i32) * 3));

                for target in CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    radius * 1.25,
                ) {
                    let d = target.center.distance(eye);
                    let dmg = Self::explosion_damage_at_distance(
                        base_dmg,
                        d,
                        radius,
                        SELF_DESTRUCT_MAX_PLAYER_DAMAGE,
                    );
                    let push_scale = (1.0 - d / (radius * 1.25)).max(0.0);
                    let dir = (target.center - eye).normalize_or_zero();
                    let dir = if dir.length_squared() < 1e-6 {
                        DVec3::Y
                    } else {
                        dir
                    };
                    let k = kb * push_scale;
                    let push = dir * k + DVec3::new(0.0, k * 0.25, 0.0);
                    if dmg > 0.0 || push_scale > 0.0 {
                        res.hits.push(AbilityHitEffect {
                            target_id: target.id,
                            damage: dmg,
                            damage_type: GrandLineDamageType::BomuExplosion,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                target.knockback_resistance,
                            ),
                            stun_ticks: if tier == 3 { 15 } else { 0 },
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: if tier == 3 { 4 } else { 0 },
                        });
                    }
                }

                ctx.budget.blocks_remaining =
                    ctx.budget.blocks_remaining.min(BOMU_MAX_BLOCKS_PER_EXPLOSION);
                ctx.budget.debris_remaining =
                    ctx.budget.debris_remaining.min(BOMU_MAX_DEBRIS_PER_EXPLOSION);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    eye,
                    radius * block_scale,
                    &mut ctx.budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_pos| ("minecraft:stone".to_string(), 1.5, false),
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed {
            if ctx.form_runtime.has_flag(BOMU_FLAG_ARMED) {
                ctx.form_runtime.set_flag(BOMU_FLAG_ARMED, false);
                ctx.world_fx.emit_fx(
                    bomu_fx::PUNCH_FIZZLE,
                    0,
                    ctx.ability_ctx.eye_pos,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    32.0,
                );
            }
            if ctx.form_runtime.has_flag(BOMU_FLAG_VANISHED) {
                ctx.form_runtime.set_flag(BOMU_FLAG_VANISHED, false);
            }
            res.vfx_events = ctx.world_fx.drain_vfx();
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
        EntityBodyModifiers {
            devil_fruit_water_weakness: player.has_fruit(),
            water_submersion,
            ..EntityBodyModifiers::VANILLA
        }
        .with_bomu_guard(suppressed)
    }
}

trait BomuGuard {
    fn with_bomu_guard(self, suppressed: bool) -> Self;
}

impl BomuGuard for EntityBodyModifiers {
    fn with_bomu_guard(self, _suppressed: bool) -> Self {
        self
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

    fn make_target(id: u8, pos: DVec3) -> TargetCandidate {
        TargetCandidate {
            id: [id; 16],
            center: pos,
            width: 0.6,
            height: 1.8,
            knockback_resistance: 0.0,
        }
    }

    #[test]
    fn bomu_bomu_abilities_and_tiers_match_java() {
        let module = BomuBomuModule::new();
        assert_eq!(module.fruit_id(), FruitId::BomuBomu);
        assert_eq!(
            module.primary_damage_type(),
            GrandLineDamageType::BomuExplosion
        );

        let cannon = module.ability("nose_fancy_cannon").expect("nose_fancy_cannon");
        assert_eq!(cannon.stats.energy_cost, 5.0);
        assert_eq!(cannon.stats.cooldown_ticks, 300);
        assert_eq!(cannon.stats.damage, 5.0);
        assert_eq!(cannon.stats.range, 1.8);

        let kick = module.ability("kick_bomb").expect("kick_bomb");
        assert_eq!(kick.stats.energy_cost, 14.0);
        assert_eq!(kick.stats.cooldown_ticks, 30);
        assert_eq!(kick.stats.damage, 10.0);

        let breath = module
            .ability("breeze_breath_bomb")
            .expect("breeze_breath_bomb");
        assert_eq!(breath.stats.energy_cost, 14.0);
        assert_eq!(breath.stats.cooldown_ticks, 16);
        assert_eq!(breath.stats.damage, 16.0);

        let zenshin = module.ability("zenshin_kibaku").expect("zenshin_kibaku");
        assert_eq!(zenshin.cast_type, CastType::Charged);
        assert!(zenshin.is_ultimate);
        assert_eq!(zenshin.stats.energy_cost, 130.0);
        assert_eq!(zenshin.stats.cooldown_ticks, 1200);
        assert_eq!(zenshin.stats.max_charge_ticks, 50);

        assert_eq!(BomuBomuModule::self_destruct_tier(0.05), 0);
        assert_eq!(BomuBomuModule::self_destruct_tier(0.15), 1);
        assert_eq!(BomuBomuModule::self_destruct_tier(0.60), 2);
        assert_eq!(BomuBomuModule::self_destruct_tier(1.0), 3);
        assert!((BomuBomuModule::self_destruct_energy_cost(1.0) - 130.0).abs() < 1e-4);
        assert_eq!(BomuBomuModule::self_destruct_cooldown_ticks(0.2), 180);
        assert!(BomuBomuModule::is_explosion_immune(true, false));
        assert!(!BomuBomuModule::is_explosion_immune(true, true));
    }

    #[test]
    fn bomu_bomu_execution_and_self_detonation_crater() {
        let module = BomuBomuModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::BomuBomu),
            mastery: 6,
            energy: 190.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = [make_target(2, DVec3::new(0.0, 64.0, 2.0))];

        let mut ctx = FruitContext {
            player_id: [1; 16],
            player: &mut player,
            form_runtime: &mut form_rt,
            gear_state: &mut gear,
            haki: &mut haki,
            logia: None,
            heat: &mut heat,
            world_fx: &mut world_fx,
            budget: CastBudget::from_config(&cfg),
            ability_ctx: AbilityContext {
                caster: [1; 16],
                fruit: FruitId::BomuBomu,
                ability_name: "nose_fancy_cannon".into(),
                mastery: 6,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 80,
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

        let cannon_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "nose_fancy_cannon".into(),
                mastery: 6,
            },
        );
        assert_eq!(cannon_res.spawned_projectiles, 1);
        assert_eq!(cannon_res.hits.len(), 1);

        let kick_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kick_bomb".into(),
                mastery: 6,
            },
        );
        assert_eq!(kick_res.hits.len(), 1);
        assert!(!kick_res.destroyed_blocks.is_empty());

        let breath_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "breeze_breath_bomb".into(),
                mastery: 6,
            },
        );
        assert_eq!(breath_res.spawned_projectiles, 1);
        assert_eq!(breath_res.hits.len(), 1);

        ctx.budget = CastBudget::from_config(&cfg);
        let zenshin_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "zenshin_kibaku".into(),
                charge: 1.0,
                mastery: 6,
            },
        );
        assert_eq!(zenshin_res.hits.len(), 1);
        assert!(!zenshin_res.destroyed_blocks.is_empty());
        assert!(ctx.form_runtime.has_flag(BOMU_FLAG_VANISHED));

        // Suppression on tick clears vanished/armed flags
        ctx.suppressed = true;
        let _ = module.on_tick(&mut ctx);
        assert!(!ctx.form_runtime.has_flag(BOMU_FLAG_VANISHED));
    }
}
