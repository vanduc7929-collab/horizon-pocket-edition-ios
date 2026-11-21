//! Supa-Supa no Mi (Dice-Dice Fruit) — 1:1 port from Grand Line 0.4.0 Java
//! (`com.devilfruits.fruits.supa_supa`).
//!
//! Includes:
//! - Canonical Java abilities: `spar_claw` (3-step Claw Rake -> Atomic Spar X combo),
//!   `steel_body` (`spider` defense parry, projectile deflection, energy absorption, knockback immunity),
//!   `blade_skates` (`atomic_spurt` skating momentum & lunge), `spiral_hollow` (spinning saw arm drill),
//!   `super_slash` (`atomic_spatula` / charged ranged planar cut).
//! - Spec aliases (`spider`, `claw_slash`, `spark_daisy`, `atomic_spurt`, `atomic_spatula`)
//!   supported seamlessly alongside the canonical `FruitId::SupaSupa` base ability keys.
//! - [`SkateMotion`] physics stepper and [`BladeLimbs`] synced form-flag encoder.

use glam::DVec3;
use horizon_world::grandline_hooks::{EntityBodyModifiers, EntityMassTier};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Grand Line 0.4.0 Java constants
// ---------------------------------------------------------------------------

/// `BladeLimbs` synced form flag bits (`FormFlags.fruit(0..=3)` and arm mode mask).
pub const BLADE_LIMBS_RIGHT_ARM: u32 = form_flags::fruit_bit(0);
pub const BLADE_LIMBS_LEFT_ARM: u32 = form_flags::fruit_bit(1);
pub const BLADE_LIMBS_LEGS: u32 = form_flags::fruit_bit(2);
pub const BLADE_LIMBS_STEEL: u32 = form_flags::fruit_bit(3);
pub const BLADE_LIMBS_MODE_SHIFT: u32 = 12;
pub const BLADE_LIMBS_MODE_MASK: u32 = 28_672;

/// `SparClawAbility`: energy 7, cooldown 14, mastery 1, damage 6.0, range 3.4, knockback 0.35.
pub const SPAR_CLAW_ENERGY: f32 = 7.0;
pub const SPAR_CLAW_COOLDOWN_TICKS: i32 = 14;
pub const SPAR_CLAW_UNLOCK_MASTERY: u32 = 1;
pub const SPAR_CLAW_DAMAGE: f32 = 6.0;
pub const SPAR_CLAW_RANGE: f32 = 3.4;
pub const SPAR_CLAW_KNOCKBACK: f32 = 0.35;
pub const SPAR_CLAW_COMBO_STEPS: u32 = 3;
pub const SPAR_CLAW_COMBO_WINDOW_TICKS: i32 = 16;
pub const SPAR_CLAW_ARC_DEGREES: f64 = 80.0;
pub const SPAR_CLAW_THROUGH_MASTERY: u32 = 6;
pub const SPAR_CLAW_THROUGH_DEPTH: f64 = 2.5;
pub const SPAR_CLAW_THROUGH_DAMAGE_FACTOR: f32 = 0.8;
pub const SPAR_CLAW_ATOMIC_DAMAGE: f32 = 14.0;
pub const SPAR_CLAW_ATOMIC_RANGE: f64 = 6.5;
pub const SPAR_CLAW_ATOMIC_HALF_WIDTH: f64 = 2.2;
pub const SPAR_CLAW_ATOMIC_MAX_BLOCKS: usize = 70;
pub const SPAR_CLAW_FINISHER_COST_MULT: f32 = 2.4;

/// `SteelBodyAbility` (`spider`): energy 10, drain 5/s, cooldown 60, mastery 2, max 30s (600 ticks).
pub const STEEL_BODY_ENERGY: f32 = 10.0;
pub const STEEL_BODY_DRAIN_PER_SEC: f32 = 5.0;
pub const STEEL_BODY_COOLDOWN_TICKS: i32 = 60;
pub const STEEL_BODY_UNLOCK_MASTERY: u32 = 2;
pub const STEEL_BODY_MAX_CHANNEL_TICKS: i32 = 600;
pub const STEEL_BODY_DAMAGE_MULTIPLIER: f32 = 0.05;
pub const STEEL_BODY_DRAIN_PER_DAMAGE: f32 = 1.2;
pub const STEEL_BODY_SPEED_MULTIPLIER: f64 = 0.25;
pub const STEEL_WALL_DAMAGE_THRESHOLD: f32 = 20.0;

/// `BladeSkatesAbility` (`atomic_spurt`): energy 6, drain 3/s, cooldown 20, mastery 3, damage 5.0, range 3.5, knockback 0.8.
pub const BLADE_SKATES_ENERGY: f32 = 6.0;
pub const BLADE_SKATES_DRAIN_PER_SEC: f32 = 3.0;
pub const BLADE_SKATES_COOLDOWN_TICKS: i32 = 20;
pub const BLADE_SKATES_UNLOCK_MASTERY: u32 = 3;
pub const BLADE_SKATES_DAMAGE: f32 = 5.0;
pub const BLADE_SKATES_RANGE: f32 = 3.5;
pub const BLADE_SKATES_KNOCKBACK: f32 = 0.8;
pub const BLADE_SKATES_MAX_SPEED: f64 = 0.85;
pub const BLADE_SKATES_ACCELERATION: f64 = 0.045;
pub const BLADE_SKATES_GLIDE: f64 = 0.985;
pub const BLADE_SKATES_BRAKE: f64 = 0.86;
pub const BLADE_SKATES_TURN_RATE: f64 = 0.16;
pub const BLADE_SKATES_LUNGE_DAMAGE_PER_SPEED: f32 = 12.0;
pub const BLADE_SKATES_LUNGE_COOLDOWN_TICKS: i32 = 14;

/// `SpiralHollowAbility`: energy 16, drain 6/s, cooldown 120, mastery 5, damage 2.6, range 4.2, knockback 0.12.
pub const SPIRAL_HOLLOW_ENERGY: f32 = 16.0;
pub const SPIRAL_HOLLOW_DRAIN_PER_SEC: f32 = 6.0;
pub const SPIRAL_HOLLOW_COOLDOWN_TICKS: i32 = 120;
pub const SPIRAL_HOLLOW_UNLOCK_MASTERY: u32 = 5;
pub const SPIRAL_HOLLOW_DAMAGE: f32 = 2.6;
pub const SPIRAL_HOLLOW_RANGE: f32 = 4.2;
pub const SPIRAL_HOLLOW_KNOCKBACK: f32 = 0.12;
pub const SPIRAL_HOLLOW_MAX_CHANNEL_TICKS: i32 = 80;
pub const SPIRAL_HOLLOW_HIT_INTERVAL_TICKS: i32 = 3;
pub const SPIRAL_HOLLOW_CONE_DEGREES: f64 = 32.0;
pub const SPIRAL_HOLLOW_ADVANCE_SPEED: f64 = 0.14;
pub const SPIRAL_HOLLOW_BLOCKS_PER_TICK: usize = 3;
pub const SPIRAL_HOLLOW_MAX_BLOCKS: usize = 80;

/// `SuperSlashAbility` (`spark_daisy` / `atomic_spatula`): energy 60, cooldown 400, mastery 8, damage 28.0, range 40.0, knockback 0.9.
pub const SUPER_SLASH_ENERGY: f32 = 60.0;
pub const SUPER_SLASH_COOLDOWN_TICKS: i32 = 400;
pub const SUPER_SLASH_UNLOCK_MASTERY: u32 = 8;
pub const SUPER_SLASH_DAMAGE: f32 = 28.0;
pub const SUPER_SLASH_RANGE: f32 = 40.0;
pub const SUPER_SLASH_KNOCKBACK: f32 = 0.9;
pub const SUPER_SLASH_CHARGE_TICKS: i32 = 30;
pub const SUPER_SLASH_HEIGHT: i32 = 5;
pub const SUPER_SLASH_SPEED: f64 = 2.0;
pub const SUPER_SLASH_MAX_BLOCKS: usize = 400;

/// Canonical Supa-Supa VFX identifiers (`SupaFx.java`).
pub mod supa_fx {
    pub const CLAW: &str = "devilfruits:supa_supa/claw";
    pub const CUT_HIT: &str = "devilfruits:supa_supa/cut_hit";
    pub const ATOMIC_SPAR: &str = "devilfruits:supa_supa/atomic_spar";
    pub const CLANG: &str = "devilfruits:supa_supa/clang";
    pub const LUNGE: &str = "devilfruits:supa_supa/lunge";
    pub const SPIRAL: &str = "devilfruits:supa_supa/spiral";
    pub const SPIRAL_BITE: &str = "devilfruits:supa_supa/spiral_bite";
    pub const SLASH: &str = "devilfruits:supa_supa/slash";
    pub const SLASH_END: &str = "devilfruits:supa_supa/slash_end";
}

/// Arm blade transformation mode (`BladeLimbs.ArmMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ArmMode {
    None = 0,
    Claws = 1,
    Forearm = 2,
    XBlades = 3,
    Saw = 4,
}

impl ArmMode {
    pub const fn from_ordinal(ord: u32) -> Self {
        match ord {
            1 => Self::Claws,
            2 => Self::Forearm,
            3 => Self::XBlades,
            4 => Self::Saw,
            _ => Self::None,
        }
    }
}

/// Encodes `BladeLimbs.flags(mode, right, left, legs, steel)`.
pub const fn encode_blade_limb_flags(
    mode: ArmMode,
    right: bool,
    left: bool,
    legs: bool,
    steel: bool,
) -> u32 {
    let arms = !matches!(mode, ArmMode::None) && (right || left);
    let arm_bits = if arms {
        ((mode as u32) << BLADE_LIMBS_MODE_SHIFT)
            | (if right { BLADE_LIMBS_RIGHT_ARM } else { 0 })
            | (if left { BLADE_LIMBS_LEFT_ARM } else { 0 })
    } else {
        0
    };
    arm_bits
        | (if legs { BLADE_LIMBS_LEGS } else { 0 })
        | (if steel { BLADE_LIMBS_STEEL } else { 0 })
}

/// Skating momentum physics parameters (`SkateMotion.Params`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkateParams {
    pub max_speed: f64,
    pub acceleration: f64,
    pub glide: f64,
    pub brake: f64,
    pub turn_rate: f64,
}

impl Default for SkateParams {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl SkateParams {
    pub const DEFAULT: Self = Self {
        max_speed: BLADE_SKATES_MAX_SPEED,
        acceleration: BLADE_SKATES_ACCELERATION,
        glide: BLADE_SKATES_GLIDE,
        brake: BLADE_SKATES_BRAKE,
        turn_rate: BLADE_SKATES_TURN_RATE,
    };
}

/// 1:1 port of `com.devilfruits.fruits.supa_supa.SkateMotion`.
pub struct SkateMotion;

impl SkateMotion {
    pub fn step(
        v: DVec3,
        forward: f64,
        strafe: f64,
        yaw_deg: f32,
        on_ground: bool,
        braking: bool,
        p: SkateParams,
    ) -> DVec3 {
        let r = (yaw_deg as f64).to_radians();
        let fx = -r.sin();
        let fz = r.cos();
        let sx = fz;
        let sz = -fx;
        let mut vx = v.x;
        let mut vz = v.z;
        let speed = (vx * vx + vz * vz).sqrt();
        if !on_ground {
            vx = vx * 0.99 + fx * p.acceleration * 0.1 * forward.max(0.0);
            vz = vz * 0.99 + fz * p.acceleration * 0.1 * forward.max(0.0);
            return Self::cap(vx, vz, p.max_speed);
        }

        let ix = fx * forward.max(0.35) + sx * strafe * 0.8;
        let iz = fz * forward.max(0.35) + sz * strafe * 0.8;
        let il = (ix * ix + iz * iz).sqrt();
        if speed > 1e-4 && il > 1e-4 {
            let a0 = vz.atan2(vx);
            let a1 = (iz / il).atan2(ix / il);
            let mut d = (a1 - a0).to_degrees() % 360.0;
            if d >= 180.0 {
                d -= 360.0;
            } else if d < -180.0 {
                d += 360.0;
            }
            let max_turn_deg = p.turn_rate.to_degrees();
            let turn = d.clamp(-max_turn_deg, max_turn_deg).to_radians();
            let a = a0 + turn;
            let keep = 1.0 - 0.06 * turn.abs();
            vx = a.cos() * speed * keep;
            vz = a.sin() * speed * keep;
        }

        if forward > 0.0 {
            vx += fx * p.acceleration * forward;
            vz += fz * p.acceleration * forward;
        }
        if strafe != 0.0 {
            vx += sx * p.acceleration * 0.3 * strafe;
            vz += sz * p.acceleration * 0.3 * strafe;
        }
        let k = if !braking && forward >= 0.0 {
            p.glide
        } else {
            p.brake
        };
        Self::cap(vx * k, vz * k, p.max_speed)
    }

    fn cap(mut vx: f64, mut vz: f64, max: f64) -> DVec3 {
        let s = (vx * vx + vz * vz).sqrt();
        if s > max && s > 1e-9 {
            vx *= max / s;
            vz *= max / s;
        }
        DVec3::new(vx, 0.0, vz)
    }

    pub fn too_fast(travel: f64, max_speed: f64) -> bool {
        travel > max_speed * 1.25 + 0.15
    }
}

/// Evaluates `SteelBodyAbility.absorb`: returns `(damage_through, energy_drained, triggered_steel_wall)`.
/// Blades, swords, axes, tridents, and non-seastone projectiles deal `0.0` damage (`steel_proof = true`);
/// other non-Haki/non-Seastone blows deal `5%` (`STEEL_BODY_DAMAGE_MULTIPLIER`) and drain `1.2` energy per deflected point.
pub fn steel_body_absorb(
    incoming_damage: f32,
    is_blade_or_projectile: bool,
    bypasses_steel: bool,
) -> (f32, f32, bool) {
    if incoming_damage <= 0.0 || bypasses_steel {
        return (incoming_damage.max(0.0), 0.0, false);
    }
    let through = if is_blade_or_projectile {
        0.0
    } else {
        incoming_damage * STEEL_BODY_DAMAGE_MULTIPLIER
    };
    let aside = (incoming_damage - through).max(0.0);
    let energy_drain = aside * STEEL_BODY_DRAIN_PER_DAMAGE;
    let wall_triggered = aside >= STEEL_WALL_DAMAGE_THRESHOLD;
    (through, energy_drain, wall_triggered)
}

/// Supa-Supa no Mi module (`SupaSupa.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SupaSupaModule;

impl SupaSupaModule {
    pub const fn new() -> Self {
        Self
    }

    fn make_ability(
        name: &str,
        display_name: &str,
        cast_type: CastType,
        stats: AbilityStats,
        is_ultimate: bool,
        stops_movement_while_charging: bool,
    ) -> AbilityDef {
        AbilityDef {
            id: AbilityId::new(FruitId::SupaSupa, name),
            display_name: display_name.to_string(),
            cast_type,
            allowed_forms: vec!["base".to_string()],
            stats,
            is_ultimate,
            stops_movement_while_charging,
            allows_cancel_on_hurt: false,
        }
    }

    fn spar_claw_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SPAR_CLAW_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SPAR_CLAW_COOLDOWN_TICKS,
            unlock_mastery: SPAR_CLAW_UNLOCK_MASTERY,
            damage: SPAR_CLAW_DAMAGE,
            range: SPAR_CLAW_RANGE,
            knockback: SPAR_CLAW_KNOCKBACK,
            max_combo_steps: SPAR_CLAW_COMBO_STEPS,
            combo_window_ticks: SPAR_CLAW_COMBO_WINDOW_TICKS,
            ..AbilityStats::default()
        }
    }

    fn steel_body_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: STEEL_BODY_ENERGY,
            energy_per_second: STEEL_BODY_DRAIN_PER_SEC,
            cooldown_ticks: STEEL_BODY_COOLDOWN_TICKS,
            unlock_mastery: STEEL_BODY_UNLOCK_MASTERY,
            damage: 0.0,
            range: 1.5,
            knockback: 0.0,
            max_channel_ticks: STEEL_BODY_MAX_CHANNEL_TICKS,
            min_channel_ticks: 0,
            ..AbilityStats::default()
        }
    }

    fn blade_skates_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: BLADE_SKATES_ENERGY,
            energy_per_second: BLADE_SKATES_DRAIN_PER_SEC,
            cooldown_ticks: BLADE_SKATES_COOLDOWN_TICKS,
            unlock_mastery: BLADE_SKATES_UNLOCK_MASTERY,
            damage: BLADE_SKATES_DAMAGE,
            range: BLADE_SKATES_RANGE,
            knockback: BLADE_SKATES_KNOCKBACK,
            ..AbilityStats::default()
        }
    }

    fn spiral_hollow_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SPIRAL_HOLLOW_ENERGY,
            energy_per_second: SPIRAL_HOLLOW_DRAIN_PER_SEC,
            cooldown_ticks: SPIRAL_HOLLOW_COOLDOWN_TICKS,
            unlock_mastery: SPIRAL_HOLLOW_UNLOCK_MASTERY,
            damage: SPIRAL_HOLLOW_DAMAGE,
            range: SPIRAL_HOLLOW_RANGE,
            knockback: SPIRAL_HOLLOW_KNOCKBACK,
            max_channel_ticks: SPIRAL_HOLLOW_MAX_CHANNEL_TICKS,
            min_channel_ticks: 0,
            ..AbilityStats::default()
        }
    }

    fn super_slash_stats() -> AbilityStats {
        AbilityStats {
            enabled: true,
            energy_cost: SUPER_SLASH_ENERGY,
            energy_per_second: 0.0,
            cooldown_ticks: SUPER_SLASH_COOLDOWN_TICKS,
            unlock_mastery: SUPER_SLASH_UNLOCK_MASTERY,
            damage: SUPER_SLASH_DAMAGE,
            range: SUPER_SLASH_RANGE,
            knockback: SUPER_SLASH_KNOCKBACK,
            max_charge_ticks: SUPER_SLASH_CHARGE_TICKS,
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
            "claw_slash" => "spar_claw",
            "spider" => "steel_body",
            "atomic_spurt" => "blade_skates",
            "spark_daisy" | "atomic_spatula" => "super_slash",
            other => other,
        }
    }

    fn exec_spar_claw(
        &self,
        ctx: &mut FruitContext<'_>,
        step: u32,
        is_finisher: bool,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::spar_claw_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let eye = ctx.ability_ctx.eye_pos;

        if !is_finisher && step < 3 {
            let left = step == 2;
            ctx.form_runtime.flags = encode_blade_limb_flags(
                ArmMode::Claws,
                true,
                true,
                ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
            );
            res.animation_clip = Some(
                if left {
                    "devilfruits:supa_supa/claw_rake_left"
                } else {
                    "devilfruits:supa_supa/claw_rake_right"
                }
                .to_string(),
            );

            let dmg = stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
            let reach =
                stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;
            let half_angle = (SPAR_CLAW_ARC_DEGREES * 0.5).to_radians();
            let mut cone_hits = CombatTargeting::in_cone(
                ctx.nearby_targets,
                ctx.player_id,
                eye,
                look,
                reach,
                half_angle,
            );
            cone_hits.sort_by(|a, b| {
                a.center
                    .distance_squared(eye)
                    .partial_cmp(&b.center.distance_squared(eye))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut through = false;
            let push = look * (SPAR_CLAW_KNOCKBACK as f64) + DVec3::new(0.0, 0.12, 0.0);
            for (idx, t) in cone_hits.iter().enumerate() {
                if idx > 0 && ctx.player.mastery < SPAR_CLAW_THROUGH_MASTERY {
                    break;
                }
                let hit_dmg = if idx == 0 {
                    dmg
                } else {
                    through = true;
                    dmg * SPAR_CLAW_THROUGH_DAMAGE_FACTOR
                };
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: hit_dmg,
                    damage_type: GrandLineDamageType::SupaBlade,
                    knockback_impulse: push,
                    stun_ticks: 0,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
            }

            if let Some(first) = cone_hits.first().copied() {
                if ctx.player.mastery >= SPAR_CLAW_THROUGH_MASTERY {
                    let from = first.center;
                    let to = from + look * (SPAR_CLAW_THROUGH_DEPTH + first.width * 0.5);
                    let through_push =
                        look * (SPAR_CLAW_KNOCKBACK as f64) + DVec3::new(0.0, 0.1, 0.0);
                    for t in CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        from,
                        to,
                        0.6,
                    ) {
                        if !res.hits.iter().any(|h| h.target_id == t.id) {
                            through = true;
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: dmg * SPAR_CLAW_THROUGH_DAMAGE_FACTOR,
                                damage_type: GrandLineDamageType::SupaBlade,
                                knockback_impulse: through_push,
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

            let extra = (if left { 1 } else { 0 }) | (if through { 2 } else { 0 });
            ctx.world_fx.emit_fx(
                supa_fx::CLAW,
                0,
                eye,
                look,
                reach as f32,
                ctx.ability_ctx.tick,
                extra,
                64.0,
            );
        } else {
            // Step 3 finisher: Atomic Spar X-cut
            ctx.form_runtime.flags = encode_blade_limb_flags(
                ArmMode::XBlades,
                true,
                true,
                ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
            );
            res.animation_clip = Some("devilfruits:supa_supa/atomic_spar".to_string());

            let base_scaled =
                stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers);
            let atomic_dmg = SPAR_CLAW_ATOMIC_DAMAGE * (base_scaled / SPAR_CLAW_DAMAGE);
            let end = eye + look * SPAR_CLAW_ATOMIC_RANGE;
            let push =
                look * ((SPAR_CLAW_KNOCKBACK * 1.6) as f64) + DVec3::new(0.0, 0.25, 0.0);

            for t in CombatTargeting::in_capsule(
                ctx.nearby_targets,
                ctx.player_id,
                eye,
                end,
                SPAR_CLAW_ATOMIC_HALF_WIDTH,
            ) {
                res.hits.push(AbilityHitEffect {
                    target_id: t.id,
                    damage: atomic_dmg,
                    damage_type: GrandLineDamageType::SupaBlade,
                    knockback_impulse: push,
                    stun_ticks: 0,
                    chill_stacks_added: 0,
                    chill_burst: None,
                    freeze_ticks: 0,
                    set_fire_seconds: 0,
                });
            }

            let mut budget = crate::core::ability::CastBudget::new(
                SPAR_CLAW_ATOMIC_MAX_BLOCKS,
                ctx.config.max_debris_entities_per_cast,
            );
            let broken = ctx.world_fx.carve_spherical_crater(
                eye + look * 2.0,
                1.5,
                &mut budget,
                true,
                ctx.ability_ctx.tick,
                ctx.config,
                ctx.game_rules,
                |_| ("minecraft:stone".to_string(), 1.5, false),
            );
            res.destroyed_blocks = broken;

            ctx.world_fx.emit_fx(
                supa_fx::ATOMIC_SPAR,
                0,
                eye,
                look,
                SPAR_CLAW_ATOMIC_RANGE as f32,
                ctx.ability_ctx.tick,
                (SPAR_CLAW_ATOMIC_HALF_WIDTH * 16.0).round() as i32,
                96.0,
            );
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_steel_body(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        match event {
            AbilityEvent::ChannelStarted { .. } | AbilityEvent::CastInstant { .. } => {
                ctx.form_runtime.set_flag(BLADE_LIMBS_LEGS, false);
                ctx.form_runtime.set_flag(BLADE_LIMBS_STEEL, true);
                res.animation_clip = Some("devilfruits:supa_supa/steel_guard".to_string());
                ctx.world_fx.emit_fx(
                    supa_fx::CLANG,
                    0,
                    ctx.ability_ctx.eye_pos,
                    ctx.ability_ctx.look_dir,
                    1.0,
                    ctx.ability_ctx.tick,
                    1,
                    48.0,
                );
            }
            AbilityEvent::ChannelTick { .. } => {
                ctx.form_runtime.set_flag(BLADE_LIMBS_STEEL, true);
                res.animation_clip = Some("devilfruits:supa_supa/steel_guard".to_string());
            }
            AbilityEvent::ChannelEnded { .. } => {
                ctx.form_runtime.set_flag(BLADE_LIMBS_STEEL, false);
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_blade_skates(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::blade_skates_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        match event {
            AbilityEvent::Toggled { enabled, .. } => {
                ctx.form_runtime.set_flag(BLADE_LIMBS_LEGS, *enabled);
                if *enabled {
                    let step_vel = SkateMotion::step(
                        DVec3::ZERO,
                        1.0,
                        0.0,
                        0.0,
                        ctx.on_ground,
                        false,
                        SkateParams::DEFAULT,
                    );
                    res.caster_velocity_impulse = Some(
                        flat * step_vel.length().max(BLADE_SKATES_ACCELERATION * 4.0),
                    );
                }
            }
            AbilityEvent::ToggleTick { .. } | AbilityEvent::CastInstant { .. } => {
                ctx.form_runtime.set_flag(BLADE_LIMBS_LEGS, true);
                let speed = BLADE_SKATES_MAX_SPEED * 0.65;
                res.caster_velocity_impulse = Some(flat * speed);
                res.animation_clip = Some("devilfruits:supa_supa/skate_lunge".to_string());

                let base_dmg = stats.scaled_damage(
                    ctx.player.mastery,
                    ctx.config,
                    &ctx.ability_ctx.modifiers,
                );
                let lunge_dmg = base_dmg + (BLADE_SKATES_LUNGE_DAMAGE_PER_SPEED * (speed as f32));
                let reach = (stats.scaled_range(
                    ctx.player.mastery,
                    ctx.config,
                    &ctx.ability_ctx.modifiers,
                ) as f64)
                    + speed * 2.0;
                let from = ctx.ability_ctx.eye_pos + DVec3::new(0.0, -0.4, 0.0);
                let to = from + look * reach;
                let push = look * ((BLADE_SKATES_KNOCKBACK as f64) * (1.0 + speed))
                    + DVec3::new(0.0, 0.25, 0.0);

                for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, from, to, 1.0)
                {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: lunge_dmg,
                        damage_type: GrandLineDamageType::SupaBlade,
                        knockback_impulse: push,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    supa_fx::LUNGE,
                    0,
                    ctx.ability_ctx.eye_pos,
                    look,
                    reach as f32,
                    ctx.ability_ctx.tick,
                    (speed * 100.0) as i32,
                    64.0,
                );
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_spiral_hollow(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::spiral_hollow_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let from = ctx.ability_ctx.eye_pos + DVec3::new(0.0, -0.35, 0.0);
        let reach =
            stats.scaled_range(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) as f64;

        match event {
            AbilityEvent::ChannelStarted { .. } => {
                ctx.form_runtime.flags = encode_blade_limb_flags(
                    ArmMode::Saw,
                    true,
                    true,
                    ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                    ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
                );
                res.animation_clip = Some("devilfruits:supa_supa/saw_stance".to_string());
                ctx.world_fx.emit_fx(
                    supa_fx::SPIRAL,
                    0,
                    ctx.ability_ctx.eye_pos,
                    look,
                    reach as f32,
                    ctx.ability_ctx.tick,
                    SPIRAL_HOLLOW_MAX_CHANNEL_TICKS,
                    64.0,
                );
            }
            AbilityEvent::ChannelTick { elapsed_ticks, .. } => {
                res.animation_clip = Some("devilfruits:supa_supa/saw_stance".to_string());
                if ctx.on_ground && *elapsed_ticks > 4 {
                    res.caster_velocity_impulse = Some(flat * SPIRAL_HOLLOW_ADVANCE_SPEED);
                }
                if (*elapsed_ticks % SPIRAL_HOLLOW_HIT_INTERVAL_TICKS) == 0 {
                    let dmg = stats.scaled_damage(
                        ctx.player.mastery,
                        ctx.config,
                        &ctx.ability_ctx.modifiers,
                    );
                    let half = SPIRAL_HOLLOW_CONE_DEGREES.to_radians();
                    let push =
                        look * (SPIRAL_HOLLOW_KNOCKBACK as f64) + DVec3::new(0.0, 0.04, 0.0);
                    for t in CombatTargeting::in_cone(
                        ctx.nearby_targets,
                        ctx.player_id,
                        from,
                        look,
                        reach,
                        half,
                    ) {
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: dmg,
                            damage_type: GrandLineDamageType::SupaBlade,
                            knockback_impulse: push,
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                    ctx.world_fx.emit_fx(
                        supa_fx::SPIRAL_BITE,
                        0,
                        from + look * 1.4,
                        look,
                        1.0,
                        ctx.ability_ctx.tick,
                        SPIRAL_HOLLOW_BLOCKS_PER_TICK as i32,
                        48.0,
                    );
                }
            }
            AbilityEvent::CastInstant { .. } => {
                // Instant direct invoke convenience
                res.animation_clip = Some("devilfruits:supa_supa/saw_stance".to_string());
                res.caster_velocity_impulse = Some(flat * SPIRAL_HOLLOW_ADVANCE_SPEED);
                let dmg = stats.scaled_damage(
                    ctx.player.mastery,
                    ctx.config,
                    &ctx.ability_ctx.modifiers,
                );
                let half = SPIRAL_HOLLOW_CONE_DEGREES.to_radians();
                let push = look * (SPIRAL_HOLLOW_KNOCKBACK as f64) + DVec3::new(0.0, 0.04, 0.0);
                for t in CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    from,
                    look,
                    reach,
                    half,
                ) {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::SupaBlade,
                        knockback_impulse: push,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_fx(
                    supa_fx::SPIRAL,
                    0,
                    ctx.ability_ctx.eye_pos,
                    look,
                    reach as f32,
                    ctx.ability_ctx.tick,
                    SPIRAL_HOLLOW_MAX_CHANNEL_TICKS,
                    64.0,
                );
            }
            AbilityEvent::ChannelEnded { .. } => {
                ctx.form_runtime.flags = encode_blade_limb_flags(
                    ArmMode::None,
                    false,
                    false,
                    ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                    ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
                );
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn exec_super_slash(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
        res: &mut AbilityExecutionResult,
    ) {
        let stats = Self::super_slash_stats();
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        match event {
            AbilityEvent::ChargeStarted { .. } => {
                ctx.form_runtime.flags = encode_blade_limb_flags(
                    ArmMode::XBlades,
                    true,
                    true,
                    ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                    ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
                );
                res.animation_clip = Some("devilfruits:supa_supa/slash_raise".to_string());
            }
            AbilityEvent::ChargeCancelled { .. } => {
                ctx.form_runtime.flags = encode_blade_limb_flags(
                    ArmMode::None,
                    false,
                    false,
                    ctx.form_runtime.has_flag(BLADE_LIMBS_LEGS),
                    ctx.form_runtime.has_flag(BLADE_LIMBS_STEEL),
                );
            }
            AbilityEvent::ChargeReleased { charge, .. } => {
                let c = charge.clamp(0.0, 1.0);
                self.fire_super_slash(ctx, &stats, look, c, res);
            }
            AbilityEvent::CastInstant { .. } => {
                let c = ctx.ability_ctx.charge.clamp(0.0, 1.0);
                self.fire_super_slash(ctx, &stats, look, c, res);
            }
            _ => {}
        }
        res.vfx_events = ctx.world_fx.drain_vfx();
    }

    fn fire_super_slash(
        &self,
        ctx: &mut FruitContext<'_>,
        stats: &AbilityStats,
        look: DVec3,
        charge: f32,
        res: &mut AbilityExecutionResult,
    ) {
        res.animation_clip = Some("devilfruits:supa_supa/slash_cut".to_string());
        res.spawned_projectiles = 1;
        let power = 0.45 + 0.55 * charge;
        let dmg =
            stats.scaled_damage(ctx.player.mastery, ctx.config, &ctx.ability_ctx.modifiers) * power;
        let range = (stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) * power) as f64;
        let start = ctx.ability_ctx.eye_pos;
        let end = start + look * range;
        let side = DVec3::new(-look.z, 0.0, look.x).normalize_or_zero();
        let half_h = (SUPER_SLASH_HEIGHT as f64) * 0.5;

        for t in CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, start, end, half_h)
        {
            let sign = if (t.center - start).dot(side) >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let push = side * (sign * (SUPER_SLASH_KNOCKBACK as f64))
                + look * 0.3
                + DVec3::new(0.0, 0.3, 0.0);
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: GrandLineDamageType::SupaBlade,
                knockback_impulse: push,
                stun_ticks: 0,
                chill_stacks_added: 0,
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: 0,
            });
        }

        let mut budget = crate::core::ability::CastBudget::new(
            SUPER_SLASH_MAX_BLOCKS,
            ctx.config.max_debris_entities_per_cast,
        );
        let broken = ctx.world_fx.carve_spherical_crater(
            start + look * (range * 0.35),
            2.0,
            &mut budget,
            true,
            ctx.ability_ctx.tick,
            ctx.config,
            ctx.game_rules,
            |_| ("minecraft:stone".to_string(), 1.5, false),
        );
        res.destroyed_blocks = broken;

        let lifetime = (range / SUPER_SLASH_SPEED).ceil() as i32;
        ctx.world_fx.emit_fx(
            supa_fx::SLASH,
            0,
            start,
            look * SUPER_SLASH_SPEED,
            SUPER_SLASH_HEIGHT as f32,
            ctx.ability_ctx.tick,
            lifetime,
            160.0,
        );
        ctx.world_fx.emit_shake(start, 0.65, 32);
        res.screen_shake = Some((0.65, 32));
    }
}

impl DevilFruitModule for SupaSupaModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::SupaSupa
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        vec![
            Self::make_ability(
                "spar_claw",
                "Spar Claw",
                CastType::Combo,
                Self::spar_claw_stats(),
                false,
                false,
            ),
            Self::make_ability(
                "steel_body",
                "Spider (Steel Body)",
                CastType::Held,
                Self::steel_body_stats(),
                false,
                true,
            ),
            Self::make_ability(
                "blade_skates",
                "Atomic Spurt (Blade Skates)",
                CastType::Toggle,
                Self::blade_skates_stats(),
                false,
                false,
            ),
            Self::make_ability(
                "spiral_hollow",
                "Spiral Hollow",
                CastType::Held,
                Self::spiral_hollow_stats(),
                false,
                false,
            ),
            Self::make_ability(
                "super_slash",
                "Spark Daisy (Super Slash)",
                CastType::Charged,
                Self::super_slash_stats(),
                true,
                true,
            ),
            // Spec aliases so direct lookup by One Piece technique names also succeeds 1:1
            Self::make_ability(
                "claw_slash",
                "Claw Slash",
                CastType::Combo,
                Self::spar_claw_stats(),
                false,
                false,
            ),
            Self::make_ability(
                "spider",
                "Spider",
                CastType::Held,
                Self::steel_body_stats(),
                false,
                true,
            ),
            Self::make_ability(
                "atomic_spurt",
                "Atomic Spurt",
                CastType::Toggle,
                Self::blade_skates_stats(),
                false,
                false,
            ),
            Self::make_ability(
                "spark_daisy",
                "Spark Daisy",
                CastType::Charged,
                Self::super_slash_stats(),
                true,
                true,
            ),
            Self::make_ability(
                "atomic_spatula",
                "Atomic Spatula",
                CastType::Charged,
                Self::super_slash_stats(),
                true,
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
            damage_type: Some(GrandLineDamageType::SupaBlade),
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
            "spar_claw" => match event {
                AbilityEvent::ComboStep {
                    step, is_finisher, ..
                } => {
                    res.cast_result = Some(CastResult::ComboStep);
                    self.exec_spar_claw(ctx, *step, *is_finisher, &mut res);
                }
                AbilityEvent::CastInstant { .. } => {
                    let step = ctx.ability_ctx.combo_step.max(1);
                    self.exec_spar_claw(ctx, step, step >= 3, &mut res);
                }
                _ => {}
            },
            "steel_body" => {
                if matches!(
                    event,
                    AbilityEvent::ChannelStarted { .. } | AbilityEvent::ChannelTick { .. }
                ) {
                    res.cast_result = Some(CastResult::Channelling);
                }
                self.exec_steel_body(ctx, event, &mut res);
            }
            "blade_skates" => {
                if matches!(event, AbilityEvent::Toggled { .. }) {
                    res.cast_result = Some(CastResult::Toggled);
                }
                self.exec_blade_skates(ctx, event, &mut res);
            }
            "spiral_hollow" => {
                if matches!(
                    event,
                    AbilityEvent::ChannelStarted { .. } | AbilityEvent::ChannelTick { .. }
                ) {
                    res.cast_result = Some(CastResult::Channelling);
                }
                self.exec_spiral_hollow(ctx, event, &mut res);
            }
            "super_slash" => {
                if matches!(event, AbilityEvent::ChargeStarted { .. }) {
                    res.cast_result = Some(CastResult::Charging);
                }
                self.exec_super_slash(ctx, event, &mut res);
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        if ctx.suppressed {
            ctx.form_runtime.flags &= !(BLADE_LIMBS_RIGHT_ARM
                | BLADE_LIMBS_LEFT_ARM
                | BLADE_LIMBS_LEGS
                | BLADE_LIMBS_STEEL
                | BLADE_LIMBS_MODE_MASK);
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
        if form_runtime.has_flag(BLADE_LIMBS_STEEL) {
            // Steel Body grants +1.0 knockback resistance and immovable steel stance
            mods.immovable = true;
            mods.mass_tier = EntityMassTier::Heavy;
        }
        if form_runtime.has_flag(BLADE_LIMBS_LEGS) {
            // Blade Skates reduce ground friction to ice-like glide
            mods.friction_override = Some(BLADE_SKATES_GLIDE as f32);
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
    fn supa_abilities_and_aliases_match_java_stats() {
        let m = SupaSupaModule::new();
        assert_eq!(m.fruit_id(), FruitId::SupaSupa);
        assert_eq!(m.primary_damage_type(), GrandLineDamageType::SupaBlade);

        let claw = m.ability("spar_claw").unwrap();
        assert_eq!(claw.cast_type, CastType::Combo);
        assert_eq!(claw.stats.energy_cost, 7.0);
        assert_eq!(claw.stats.cooldown_ticks, 14);
        assert_eq!(claw.stats.unlock_mastery, 1);
        assert_eq!(claw.stats.damage, 6.0);
        assert_eq!(claw.stats.range, 3.4);

        let spider = m.ability("spider").unwrap();
        assert_eq!(spider.stats.energy_cost, 10.0);
        assert_eq!(spider.stats.energy_per_second, 5.0);
        assert_eq!(spider.stats.cooldown_ticks, 60);
        assert_eq!(spider.stats.unlock_mastery, 2);

        let skates = m.ability("atomic_spurt").unwrap();
        assert_eq!(skates.cast_type, CastType::Toggle);
        assert_eq!(skates.stats.energy_cost, 6.0);
        assert_eq!(skates.stats.unlock_mastery, 3);

        let spiral = m.ability("spiral_hollow").unwrap();
        assert_eq!(spiral.cast_type, CastType::Held);
        assert_eq!(spiral.stats.energy_cost, 16.0);
        assert_eq!(spiral.stats.unlock_mastery, 5);
        assert_eq!(spiral.stats.damage, 2.6);

        let slash = m.ability("super_slash").unwrap();
        assert!(slash.is_ultimate);
        assert_eq!(slash.stats.energy_cost, 60.0);
        assert_eq!(slash.stats.cooldown_ticks, 400);
        assert_eq!(slash.stats.unlock_mastery, 8);
        assert_eq!(slash.stats.damage, 28.0);
        assert_eq!(slash.stats.range, 40.0);
    }

    #[test]
    fn supa_spar_claw_combo_and_atomic_spar_finisher() {
        let m = SupaSupaModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::SupaSupa),
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
        let targets = [
            test_target(2, DVec3::new(0.0, 0.0, 2.0)),
            test_target(3, DVec3::new(0.0, 0.0, 4.5)),
        ];

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
                fruit: FruitId::SupaSupa,
                ability_name: "spar_claw".into(),
                mastery: 8,
                charge: 1.0,
                combo_step: 1,
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

        // Step 1 hits primary target and cuts through to target behind (mastery 8 >= 6)
        let r1 = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ComboStep {
                ability: "spar_claw".into(),
                step: 1,
                is_finisher: false,
            },
        );
        assert_eq!(r1.hits.len(), 2);
        assert!(r1.hits[0].damage > r1.hits[1].damage);

        // Step 3 finisher executes Atomic Spar X cut and carves blocks
        let r3 = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ComboStep {
                ability: "spar_claw".into(),
                step: 3,
                is_finisher: true,
            },
        );
        assert_eq!(r3.hits.len(), 2);
        assert!(r3.hits[0].damage > r1.hits[0].damage);
        assert!(!r3.destroyed_blocks.is_empty());
    }

    #[test]
    fn supa_steel_body_parry_skates_and_super_slash() {
        let m = SupaSupaModule::new();
        let (through_blade, drain_blade, _) = steel_body_absorb(15.0, true, false);
        assert_eq!(through_blade, 0.0);
        assert!((drain_blade - 18.0).abs() < 1e-4);

        let (through_blunt, _, wall) = steel_body_absorb(25.0, false, false);
        assert!((through_blunt - 1.25).abs() < 1e-4);
        assert!(wall);

        let skate_v = SkateMotion::step(
            DVec3::new(0.0, 0.0, 0.2),
            1.0,
            0.0,
            0.0,
            true,
            false,
            SkateParams::DEFAULT,
        );
        assert!(skate_v.z > 0.2 && skate_v.z <= BLADE_SKATES_MAX_SPEED);
        assert!(SkateMotion::too_fast(1.5, BLADE_SKATES_MAX_SPEED));

        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::SupaSupa),
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
        let targets = [test_target(9, DVec3::new(0.0, 0.0, 12.0))];

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
                fruit: FruitId::SupaSupa,
                ability_name: "super_slash".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 200,
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

        let _ = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChannelStarted {
                ability: "steel_body".into(),
            },
        );
        let mods = m.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(mods.is_immovable());

        let slash_res = m.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "super_slash".into(),
                charge: 1.0,
                mastery: 10,
            },
        );
        assert_eq!(slash_res.spawned_projectiles, 1);
        assert_eq!(slash_res.hits.len(), 1);
        assert!(slash_res.screen_shake.is_some());
    }
}
