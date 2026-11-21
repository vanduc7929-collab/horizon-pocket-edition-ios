//! Horo-Horo no Mi (Hollow-Hollow Fruit) — 1:1 port from Grand Line 0.4.0 Java
//! (`com.devilfruits.fruits.horo_horo`).
//!
//! Includes:
//! - `negative_hollow`: Homing negative ghost (`HollowFlight`) that passes through targets
//!   and inflicts the `horo_horo/negative` depression kneel debuff (`-90%` speed, `-100%` jump,
//!   Weakness II, Mining Fatigue III, blocks ability casts).
//! - `mini_hollow`: Toggle 4-ghost homing cluster that clings to the target and detonates
//!   on second press (clap) or after `attachSeconds` (8s) at `timeoutFactor` (0.5).
//! - `astral_projection` (`ghost_form`): Astral ghost body projection (`"ghost"` form) with
//!   tethered body anchor (`ghostLeash = 64.0`), flight, recall speed (`0.55`), and damage forwarding.
//! - `special_hollow` (`toku_hollow`): Giant Kamikaze Hollow that swallows & stuns its target
//!   for `swellSeconds` (2.5s = 50 ticks) and detonates a massive `HollowBlast` (`HoroBlast` damage).

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

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

/// `GhostForm` synced form flags (`FormFlags.fruit(0..=2)`).
pub const GHOST_FLAG_OUT: u32 = form_flags::fruit_bit(0);
pub const GHOST_FLAG_POSSESSING: u32 = form_flags::fruit_bit(1);
pub const GHOST_FLAG_RECALLING: u32 = form_flags::fruit_bit(2);

/// `HoroHoro` fruit-wide settings.
pub const HORO_BLOCKS_ABILITIES_IN_NEGATIVE: bool = true;
pub const HORO_BOSS_NEGATIVE_FACTOR: f64 = 0.4;
pub const HORO_GHOST_LEASH_BLOCKS: f64 = 64.0;
pub const HORO_RECALL_HOLD_SECONDS: f64 = 3.0;
pub const HORO_POSSESS_REACH_BLOCKS: f64 = 4.0;
pub const HORO_POSSESS_TICKS: i32 = 12;
pub const HORO_RECALL_SPEED: f64 = 0.55;

/// `Negative` depression debuff attribute modifiers (`Negative.java`).
pub const NEGATIVE_SPEED_MODIFIER: f64 = -0.9;
pub const NEGATIVE_JUMP_MODIFIER: f64 = -1.0;
pub const NEGATIVE_FLY_MODIFIER: f64 = -0.9;
pub const NEGATIVE_WEAKNESS_AMPLIFIER: u8 = 1;
pub const NEGATIVE_MINING_FATIGUE_AMPLIFIER: u8 = 2;

/// `NegativeHollowAbility`: energy 18, cooldown 160, mastery 1, damage 2.0, range 24.0.
pub const NEGATIVE_HOLLOW_ENERGY: f32 = 18.0;
pub const NEGATIVE_HOLLOW_COOLDOWN_TICKS: i32 = 160;
pub const NEGATIVE_HOLLOW_UNLOCK_MASTERY: u32 = 1;
pub const NEGATIVE_HOLLOW_DAMAGE: f32 = 2.0;
pub const NEGATIVE_HOLLOW_RANGE: f32 = 24.0;
pub const NEGATIVE_HOLLOW_SECONDS: f64 = 4.0;
pub const NEGATIVE_HOLLOW_SPEED: f64 = 1.3;
pub const NEGATIVE_HOLLOW_TURN: f64 = 0.22;
pub const NEGATIVE_HOLLOW_AIM_CONE_DEGREES: f64 = 8.0;
pub const NEGATIVE_HOLLOW_EMERGE_TICKS: i32 = 6;
pub const NEGATIVE_HOLLOW_MAX_FLIGHT_TICKS: i32 = 80;

/// `MiniHollowAbility`: energy 22, cooldown 200, mastery 2, damage 14.0 (shared across 4 ghosts), range 24.0, knockback 0.45.
pub const MINI_HOLLOW_ENERGY: f32 = 22.0;
pub const MINI_HOLLOW_COOLDOWN_TICKS: i32 = 200;
pub const MINI_HOLLOW_UNLOCK_MASTERY: u32 = 2;
pub const MINI_HOLLOW_DAMAGE: f32 = 14.0;
pub const MINI_HOLLOW_RANGE: f32 = 24.0;
pub const MINI_HOLLOW_KNOCKBACK: f32 = 0.45;
pub const MINI_HOLLOW_GHOST_COUNT: usize = 4;
pub const MINI_HOLLOW_ATTACH_SECONDS: f64 = 8.0;
pub const MINI_HOLLOW_TIMEOUT_FACTOR: f32 = 0.5;
pub const MINI_HOLLOW_SPEED: f64 = 1.0;
pub const MINI_HOLLOW_TURN: f64 = 0.16;
pub const MINI_HOLLOW_AIM_CONE_DEGREES: f64 = 8.0;
pub const MINI_HOLLOW_EMERGE_TICKS: i32 = 5;
pub const MINI_HOLLOW_CLAP_TICKS: i32 = 6;

/// `GhostFormAbility` (`astral_projection`): energy 15, cooldown 300, mastery 4.
pub const GHOST_FORM_ENERGY: f32 = 15.0;
pub const GHOST_FORM_COOLDOWN_TICKS: i32 = 300;
pub const GHOST_FORM_UNLOCK_MASTERY: u32 = 4;

/// `SpecialHollowAbility` (`toku_hollow`): energy 60, cooldown 1200, mastery 8, damage 32.0, range 32.0, knockback 2.4.
pub const SPECIAL_HOLLOW_ENERGY: f32 = 60.0;
pub const SPECIAL_HOLLOW_COOLDOWN_TICKS: i32 = 1200;
pub const SPECIAL_HOLLOW_UNLOCK_MASTERY: u32 = 8;
pub const SPECIAL_HOLLOW_DAMAGE: f32 = 32.0;
pub const SPECIAL_HOLLOW_RANGE: f32 = 32.0;
pub const SPECIAL_HOLLOW_KNOCKBACK: f32 = 2.4;
pub const SPECIAL_HOLLOW_SPEED: f64 = 0.3;
pub const SPECIAL_HOLLOW_TURN: f64 = 0.1;
pub const SPECIAL_HOLLOW_SWELL_SECONDS: f64 = 2.5;
pub const SPECIAL_HOLLOW_BLAST_RADIUS: f64 = 8.5;
pub const SPECIAL_HOLLOW_BLOCK_POWER: f32 = 6.0;
pub const SPECIAL_HOLLOW_MAX_BLOCKS: usize = 260;
pub const SPECIAL_HOLLOW_MAX_PLAYER_DAMAGE: f32 = 26.0;
pub const SPECIAL_HOLLOW_AIM_CONE_DEGREES: f64 = 10.0;
pub const SPECIAL_HOLLOW_RISE_TICKS: i32 = 24;
pub const SPECIAL_HOLLOW_RISE_HEIGHT: f64 = 4.5;

/// Canonical Horo-Horo VFX identifiers (`HoroFx.java`).
pub mod horo_fx {
    pub const QUIET: &str = "devilfruits:horo_horo/quiet";
    pub const NEGATIVE_LAUNCH: &str = "devilfruits:horo_horo/negative_launch";
    pub const NEGATIVE_HIT: &str = "devilfruits:horo_horo/negative_hit";
    pub const MINI_LAUNCH: &str = "devilfruits:horo_horo/mini_launch";
    pub const CLAP: &str = "devilfruits:horo_horo/clap";
    pub const MINI_POP: &str = "devilfruits:horo_horo/mini_pop";
    pub const GHOST_EXIT: &str = "devilfruits:horo_horo/ghost_exit";
    pub const POSSESS: &str = "devilfruits:horo_horo/possess";
    pub const GIANT_LAUNCH: &str = "devilfruits:horo_horo/giant_launch";
    pub const GIANT_ABSORB: &str = "devilfruits:horo_horo/giant_absorb";
    pub const GIANT_END: &str = "devilfruits:horo_horo/giant_end";
}

/// 1:1 port of `com.devilfruits.fruits.horo_horo.HollowFlight`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HollowFlight {
    pub pos: DVec3,
    pub heading: DVec3,
    pub speed: f64,
    pub turn: f64,
}

impl HollowFlight {
    pub fn new(pos: DVec3, heading: DVec3, speed: f64, turn: f64) -> Self {
        let h = if heading.length_squared() < 1e-8 {
            DVec3::Z
        } else {
            heading.normalize()
        };
        Self {
            pos,
            heading: h,
            speed,
            turn,
        }
    }

    pub fn step(&mut self, target: DVec3) -> f64 {
        let to = target - self.pos;
        let d = to.length();
        if d > 1e-6 {
            let want = to / d;
            let t = self.turn + (1.0 - self.turn) * ((4.0 - d) / 3.0).clamp(0.0, 1.0);
            let h = self.heading * (1.0 - t) + want * t;
            self.heading = if h.length_squared() < 1e-6 {
                want
            } else {
                h.normalize()
            };
        }
        let mv = self.speed.min(d);
        self.pos += self.heading * mv;
        target.distance(self.pos)
    }
}

/// Computes exact Negative Hollow depression duration in ticks (`NegativeHollowAbility.activate` + `Negative.apply`):
/// `round(negativeSeconds * 20.0 * (1.0 + (mastery - 1) / 27.0))`, scaled by `0.4` on bosses.
pub fn negative_duration_ticks(mastery: u32, is_boss: bool) -> i32 {
    let base = (NEGATIVE_HOLLOW_SECONDS * 20.0 * (1.0 + (mastery.max(1) - 1) as f64 / 27.0))
        .round() as i32;
    if is_boss {
        ((base as f64) * HORO_BOSS_NEGATIVE_FACTOR).round() as i32
    } else {
        base
    }
}

/// Computes soft tether pullback velocity when the astral ghost exceeds `HORO_GHOST_LEASH_BLOCKS` (`GhostForm.leash`).
pub fn ghost_leash_pull_velocity(
    ghost_pos: DVec3,
    body_pos: DVec3,
    current_vel: DVec3,
    leash: f64,
) -> (DVec3, Option<DVec3>) {
    let off = ghost_pos - body_pos;
    let d = off.length();
    if d <= leash || d < 1e-6 {
        return (current_vel, None);
    }
    let home = -off / d;
    let pull = ((d - leash) * 0.12).min(0.8);
    let mut v = current_vel;
    let outward = -v.dot(home);
    if outward > 0.0 {
        v += home * outward;
    }
    let new_vel = v + home * pull;
    let clamp_pos = if d > leash + 8.0 {
        Some(body_pos + off * ((leash + 8.0) / d))
    } else {
        None
    };
    (new_vel, clamp_pos)
}

/// Horo-Horo no Mi module (`HoroHoro.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct HoroHoroModule;

impl HoroHoroModule {
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
            id: AbilityId::new(FruitId::HoroHoro, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms,
            stats,
            is_ultimate,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        }
    }

    fn negative_hollow_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: NEGATIVE_HOLLOW_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: NEGATIVE_HOLLOW_COOLDOWN_TICKS,
            unlock_mastery: NEGATIVE_HOLLOW_UNLOCK_MASTERY,
            damage: NEGATIVE_HOLLOW_DAMAGE,
            range: NEGATIVE_HOLLOW_RANGE,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn mini_hollow_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: MINI_HOLLOW_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: MINI_HOLLOW_COOLDOWN_TICKS,
            unlock_mastery: MINI_HOLLOW_UNLOCK_MASTERY,
            damage: MINI_HOLLOW_DAMAGE,
            range: MINI_HOLLOW_RANGE,
            knockback: MINI_HOLLOW_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn astral_projection_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: GHOST_FORM_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: GHOST_FORM_COOLDOWN_TICKS,
            unlock_mastery: GHOST_FORM_UNLOCK_MASTERY,
            damage: 0.0,
            range: 1.0,
            knockback: 0.0,
            ..AbilityStats::default()
        }
    }

    fn special_hollow_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SPECIAL_HOLLOW_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SPECIAL_HOLLOW_COOLDOWN_TICKS,
            unlock_mastery: SPECIAL_HOLLOW_UNLOCK_MASTERY,
            damage: SPECIAL_HOLLOW_DAMAGE,
            range: SPECIAL_HOLLOW_RANGE,
            knockback: SPECIAL_HOLLOW_KNOCKBACK,
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
            "ghost_form" => "astral_projection",
            "toku_hollow" => "special_hollow",
            other => other,
        }
    }

    fn exec_negative_hollow(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::negative_hollow_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let debuff_ticks = negative_duration_ticks(ctx.player.mastery, false);

        res.animation_clip = Some("devilfruits:horo_horo/extend_arm".to_string());
        res.spawned_projectiles = 1;

        let target = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            NEGATIVE_HOLLOW_AIM_CONE_DEGREES.to_radians().max(0.45),
        )
        .or_else(|| {
            CombatTargeting::in_cone(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.5)
                .into_iter()
                .next()
        });

        let right = DVec3::new(-look.z, 0.0, look.x).normalize_or_zero();
        let hand = eye + look * 0.9 + right * 0.35 + DVec3::new(0.0, -0.35, 0.0);

        ctx.world_fx.emit_fx(
            horo_fx::NEGATIVE_LAUNCH,
            0,
            hand,
            look,
            NEGATIVE_HOLLOW_SPEED as f32,
            ctx.ability_ctx.tick,
            debuff_ticks,
            64.0,
        );

        if let Some(t) = target {
            let mut flight =
                HollowFlight::new(hand, look, NEGATIVE_HOLLOW_SPEED, NEGATIVE_HOLLOW_TURN);
            for _ in 0..NEGATIVE_HOLLOW_MAX_FLIGHT_TICKS {
                if flight.step(t.center) <= t.width * 0.5 + 0.3 {
                    break;
                }
            }
            ctx.world_fx.emit_fx(
                horo_fx::NEGATIVE_HIT,
                0,
                flight.pos,
                flight.heading,
                1.0,
                ctx.ability_ctx.tick,
                debuff_ticks,
                64.0,
            );
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::HoroHollow,
                knockback_impulse: DVec3::ZERO,
                stun_ticks: debuff_ticks,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_mini_hollow(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::mini_hollow_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let total_dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let chest = eye + look * 0.7 + DVec3::new(0.0, -0.45, 0.0);

        let target = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            MINI_HOLLOW_AIM_CONE_DEGREES.to_radians().max(0.5),
        )
        .or_else(|| {
            CombatTargeting::in_cone(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.55)
                .into_iter()
                .next()
        });

        match event {
            AbilityEvent::Toggled { enabled: true, .. } => {
                res.animation_clip = Some("devilfruits:horo_horo/throw_rap".to_string());
                res.spawned_projectiles = MINI_HOLLOW_GHOST_COUNT;
                ctx.world_fx.emit_fx(
                    horo_fx::MINI_LAUNCH,
                    0,
                    chest,
                    look,
                    MINI_HOLLOW_SPEED as f32,
                    ctx.ability_ctx.tick,
                    MINI_HOLLOW_GHOST_COUNT as i32,
                    64.0,
                );
            }
            AbilityEvent::Toggled { enabled: false, .. } | AbilityEvent::CastInstant { .. } => {
                // Clap detonation: pops all 4 attached mini ghosts at 100% damage
                res.animation_clip = Some("devilfruits:horo_horo/clap".to_string());
                res.spawned_projectiles = MINI_HOLLOW_GHOST_COUNT;
                ctx.world_fx.emit_fx(
                    horo_fx::CLAP,
                    0,
                    eye,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
                if let Some(t) = target {
                    let push =
                        look * (MINI_HOLLOW_KNOCKBACK as f64) + DVec3::new(0.0, 0.12, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: total_dmg,
                        damage_type: GrandLineDamageType::HoroHollow,
                        knockback_impulse: push,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    for i in 0..MINI_HOLLOW_GHOST_COUNT {
                        ctx.world_fx.emit_fx(
                            horo_fx::MINI_POP,
                            0,
                            t.center,
                            DVec3::ZERO,
                            1.0,
                            ctx.ability_ctx.tick,
                            i as i32,
                            64.0,
                        );
                    }
                }
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_astral_projection(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let entering = match event {
            AbilityEvent::Toggled { enabled, .. } => *enabled,
            AbilityEvent::FormSwitchRequested { .. } | AbilityEvent::CastInstant { .. } => {
                ctx.player.active_form != "ghost"
            }
            _ => false,
        };

        if entering {
            ctx.player.active_form = "ghost".to_string();
            ctx.form_runtime.set_flag(form_flags::FLYING, true);
            ctx.form_runtime.set_flag(GHOST_FLAG_OUT, true);
            ctx.form_runtime.set_flag(GHOST_FLAG_POSSESSING, false);
            ctx.form_runtime.set_flag(GHOST_FLAG_RECALLING, false);
            res.form_switch_to = Some("ghost".to_string());
            res.caster_velocity_impulse = Some(DVec3::new(0.0, 0.4, 0.0));
            ctx.world_fx.emit_fx(
                horo_fx::GHOST_EXIT,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                0,
                64.0,
            );
        } else {
            ctx.player.active_form = "base".to_string();
            ctx.form_runtime.set_flag(form_flags::FLYING, false);
            ctx.form_runtime.set_flag(GHOST_FLAG_OUT, false);
            ctx.form_runtime.set_flag(GHOST_FLAG_POSSESSING, false);
            ctx.form_runtime.set_flag(GHOST_FLAG_RECALLING, false);
            ctx.form_runtime.safe_landing.arm();
            res.form_switch_to = Some("base".to_string());
            res.animation_clip = Some("devilfruits:horo_horo/possess".to_string());
            ctx.world_fx.emit_fx(
                horo_fx::POSSESS,
                0,
                ctx.ability_ctx.eye_pos,
                DVec3::ZERO,
                1.0,
                ctx.ability_ctx.tick,
                HORO_POSSESS_TICKS,
                64.0,
            );
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_special_hollow(&self, ctx: &mut FruitContext<'_>, res: &mut AbilityExecutionResult) {
        let stats = Self::special_hollow_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;
        let range =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
        let base_dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
        let swell_ticks = (SPECIAL_HOLLOW_SWELL_SECONDS * 20.0).round() as i32;

        res.damage_type = Some(GrandLineDamageType::HoroBlast);
        res.animation_clip = Some("devilfruits:horo_horo/kamikaze_raise".to_string());
        res.spawned_projectiles = 1;

        let start = eye + DVec3::new(0.0, SPECIAL_HOLLOW_RISE_HEIGHT, 0.0);
        ctx.world_fx.emit_fx(
            horo_fx::GIANT_LAUNCH,
            0,
            start,
            look,
            SPECIAL_HOLLOW_SPEED as f32,
            ctx.ability_ctx.tick,
            swell_ticks,
            128.0,
        );

        let primary = CombatTargeting::aimed(
            ctx.nearby_targets,
            ctx.player_id,
            eye,
            look,
            range,
            SPECIAL_HOLLOW_AIM_CONE_DEGREES.to_radians().max(0.55),
        )
        .or_else(|| {
            CombatTargeting::in_cone(ctx.nearby_targets, ctx.player_id, eye, look, range, 0.65)
                .into_iter()
                .next()
        });

        let blast_center = primary.map(|p| p.center).unwrap_or(eye + look * 8.0);
        ctx.world_fx.emit_fx(
            horo_fx::GIANT_ABSORB,
            0,
            blast_center,
            DVec3::ZERO,
            1.0,
            ctx.ability_ctx.tick,
            swell_ticks,
            128.0,
        );

        for t in CombatTargeting::in_radius(
            ctx.nearby_targets,
            ctx.player_id,
            blast_center,
            SPECIAL_HOLLOW_BLAST_RADIUS,
        ) {
            let d = t.center.distance(blast_center);
            let f = (1.0 - d / SPECIAL_HOLLOW_BLAST_RADIUS).max(0.0);
            if f > 0.0 {
                let raw_dmg = (base_dmg as f64 * (f * f + f) * 0.5) as f32;
                let final_dmg = raw_dmg.min(SPECIAL_HOLLOW_MAX_PLAYER_DAMAGE);
                let out = (t.center - blast_center).normalize_or_zero();
                let dir = if out.length_squared() < 1e-6 {
                    DVec3::Y
                } else {
                    out
                };
                let k = (SPECIAL_HOLLOW_KNOCKBACK as f64) * f;
                let push = dir * k + DVec3::new(0.0, k * 0.3, 0.0);
                let stun = if Some(t.id) == primary.map(|p| p.id) {
                    swell_ticks + 2
                } else {
                    0
                };
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: final_dmg,
                    damage_type: GrandLineDamageType::HoroBlast,
                    knockback_impulse: push,
                    stun_ticks: stun,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
            }
        }

        let mut budget = crate::core::ability::CastBudget::new(
            SPECIAL_HOLLOW_MAX_BLOCKS,
            ctx.config.max_debris_entities_per_cast,
        );
        res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
            blast_center,
            3.0,
            &mut budget,
            false,
            ctx.ability_ctx.tick,
            ctx.config,
            ctx.game_rules,
            |_| ("minecraft:stone".to_string(), 1.5, false),
        );

        ctx.world_fx.emit_fx(
            horo_fx::GIANT_END,
            -1,
            blast_center,
            DVec3::ZERO,
            1.0,
            ctx.ability_ctx.tick,
            0,
            128.0,
        );
        ctx.world_fx.emit_shake(blast_center, 0.9, 36);
        res.screen_shake = Some((0.9, 36));
        res.vfx_events = ctx.world_fx.drain_vfx();
    }
}

impl DevilFruitModule for HoroHoroModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::HoroHoro
    }

    fn forms(&self) -> Vec<FruitFormDef> {
        let mut forms = FruitFormDef::builtin_forms(FruitId::HoroHoro);
        for f in &mut forms {
            if f.id.name == "ghost" && !f.abilities.iter().any(|a| a == "astral_projection") {
                f.abilities.push("astral_projection".to_string());
                f.abilities.push("ghost_form".to_string());
                f.abilities.push("toku_hollow".to_string());
            }
            if f.id.name == "base" {
                f.abilities.push("ghost_form".to_string());
                f.abilities.push("toku_hollow".to_string());
            }
        }
        forms
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let both_forms = vec!["base".to_string(), "ghost".to_string()];
        vec![
            Self::make_ability(
                "negative_hollow",
                "Negative Hollow",
                CastType::Instant,
                both_forms.clone(),
                Self::negative_hollow_stats(),
                false,
            ),
            Self::make_ability(
                "mini_hollow",
                "Mini Hollow",
                CastType::Toggle,
                both_forms.clone(),
                Self::mini_hollow_stats(),
                false,
            ),
            Self::make_ability(
                "astral_projection",
                "Astral Projection (Ghost Form)",
                CastType::Toggle,
                both_forms.clone(),
                Self::astral_projection_stats(),
                false,
            ),
            Self::make_ability(
                "special_hollow",
                "Toku Hollow (Special Hollow)",
                CastType::Instant,
                both_forms.clone(),
                Self::special_hollow_stats(),
                true,
            ),
            // Spec aliases
            Self::make_ability(
                "ghost_form",
                "Ghost Form",
                CastType::Toggle,
                both_forms.clone(),
                Self::astral_projection_stats(),
                false,
            ),
            Self::make_ability(
                "toku_hollow",
                "Toku Hollow",
                CastType::Instant,
                both_forms,
                Self::special_hollow_stats(),
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
            damage_type: Some(GrandLineDamageType::HoroHollow),
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
            "negative_hollow" => {
                self.exec_negative_hollow(ctx, &mut res);
            }
            "mini_hollow" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_mini_hollow(ctx, event, &mut res);
            }
            "astral_projection" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_astral_projection(ctx, event, &mut res);
            }
            "special_hollow" => {
                self.exec_special_hollow(ctx, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.player.active_form == "ghost" {
            if ctx.suppressed || ctx.submerged_in_water {
                ctx.player.active_form = "base".to_string();
                ctx.form_runtime.set_flag(form_flags::FLYING, false);
                ctx.form_runtime.set_flag(GHOST_FLAG_OUT, false);
                ctx.form_runtime.set_flag(GHOST_FLAG_POSSESSING, false);
                ctx.form_runtime.set_flag(GHOST_FLAG_RECALLING, false);
                res.form_switch_to = Some("base".to_string());
            } else {
                ctx.form_runtime.set_flag(form_flags::FLYING, true);
                ctx.form_runtime.set_flag(GHOST_FLAG_OUT, true);
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
        if player.active_form == "ghost" || form_runtime.has_flag(GHOST_FLAG_OUT) {
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
    fn horo_abilities_and_constants_match_java() {
        let m = HoroHoroModule::new();
        assert_eq!(m.fruit_id(), FruitId::HoroHoro);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::HoroHollow);

        let neg = m.ability("negative_hollow").unwrap();
        assert_eq!(neg.stats.energy_cost, 18.0);
        assert_eq!(neg.stats.cooldown_ticks, 160);
        assert_eq!(neg.stats.unlock_mastery, 1);
        assert_eq!(neg.stats.damage, 2.0);
        assert_eq!(neg.stats.range, 24.0);

        let mini = m.ability("mini_hollow").unwrap();
        assert_eq!(mini.cast_type, CastType::Toggle);
        assert_eq!(mini.stats.energy_cost, 22.0);
        assert_eq!(mini.stats.cooldown_ticks, 200);
        assert_eq!(mini.stats.unlock_mastery, 2);
        assert_eq!(mini.stats.damage, 14.0);

        let astral = m.ability("astral_projection").unwrap();
        assert_eq!(astral.stats.energy_cost, 15.0);
        assert_eq!(astral.stats.cooldown_ticks, 300);
        assert_eq!(astral.stats.unlock_mastery, 4);

        let toku = m.ability("toku_hollow").unwrap();
        assert!(toku.is_ultimate);
        assert_eq!(toku.stats.energy_cost, 60.0);
        assert_eq!(toku.stats.cooldown_ticks, 1200);
        assert_eq!(toku.stats.unlock_mastery, 8);
        assert_eq!(toku.stats.damage, 32.0);
        assert_eq!(toku.stats.range, 32.0);

        assert_eq!(negative_duration_ticks(1, false), 80);
        assert_eq!(negative_duration_ticks(10, false), 107);
        assert_eq!(negative_duration_ticks(1, true), 32);
    }

    #[test]
    fn horo_negative_hollow_mini_hollow_and_toku_blast() {
        let m = HoroHoroModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::HoroHoro),
            mastery: 8,
            energy: 200.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut wfx = WorldFxManager::default();
        let targets = [test_target(2, DVec3::new(0.0, 0.0, 6.0))];

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
                fruit: FruitId::HoroHoro,
                ability_name: "negative_hollow".into(),
                mastery: 8,
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

        let neg_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "negative_hollow".into(),
                mastery: 8,
            },
        );
        assert_eq!(neg_res.hits.len(), 1);
        assert!(neg_res.hits[0].stun_ticks >= 100);

        let mini_launch = m.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "mini_hollow".into(),
                enabled: true,
            },
        );
        assert_eq!(mini_launch.spawned_projectiles, 4);

        let mini_pop = m.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "mini_hollow".into(),
                enabled: false,
            },
        );
        assert_eq!(mini_pop.hits.len(), 1);
        assert!(mini_pop.hits[0].damage >= 14.0);

        let toku_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "toku_hollow".into(),
                mastery: 8,
            },
        );
        assert_eq!(toku_res.damage_type, Some(GrandLineDamageType::HoroBlast));
        assert_eq!(toku_res.hits.len(), 1);
        assert!(toku_res.hits[0].damage <= SPECIAL_HOLLOW_MAX_PLAYER_DAMAGE);
        assert!(toku_res.screen_shake.is_some());
    }

    #[test]
    fn horo_astral_projection_flight_tether_and_suppression_revert() {
        let m = HoroHoroModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::HoroHoro),
            mastery: 6,
            energy: 200.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut wfx = WorldFxManager::default();

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
                fruit: FruitId::HoroHoro,
                ability_name: "astral_projection".into(),
                mastery: 6,
                charge: 1.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 150,
                eye_pos: DVec3::ZERO,
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

        let enter_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "astral_projection".into(),
                enabled: true,
            },
        );
        assert_eq!(enter_res.form_switch_to.as_deref(), Some("ghost"));
        assert!(ctx.form_runtime.has_flag(GHOST_FLAG_OUT));
        let mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(mods.passes_through_leaves);
        assert!(mods.reset_above_ground_ticks);

        // Leash pullback beyond 64 blocks
        let (pull_v, clamp_pos) = ghost_leash_pull_velocity(
            DVec3::new(0.0, 0.0, 80.0),
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, 1.0),
            HORO_GHOST_LEASH_BLOCKS,
        );
        assert!(pull_v.z < 0.0);
        assert!(clamp_pos.is_some());

        // Suppression immediately recalls ghost to body
        ctx.suppressed = true;
        let tick_res = m.on_tick(&mut ctx);
        assert_eq!(tick_res.form_switch_to.as_deref(), Some("base"));
        assert_eq!(ctx.player.active_form, "base");
    }
}
