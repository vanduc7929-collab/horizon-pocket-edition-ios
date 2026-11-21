//! Moku Moku no Mi (Smoke Logia) — Grand Line 0.4.0 1:1 port.
//!
//! Mirrors `fruits/moku_moku/{MokuMoku,MokuAbilities,MokuEvents,MokuFx,MokuEnvironment,SmokeGrab}.java`
//! and `fruits/moku_moku/ability/{SmokeTechnique,SmokeArmAbility,WhiteBlowAbility,
//! WhiteLauncherAbility,SmokeFlightAbility,SmokeFlight,WhiteSnakeAbility,WhiteChaseAbility,WhiteOutAbility,SmokeClouds}.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType, RestraintRules};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::logia::limbs;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Canonical Moku-Moku constants (`MokuMoku.java`, `SmokeGrab.java`, abilities)
// ---------------------------------------------------------------------------

/// `MokuMoku.java` & `SmokeGrab.java` settings.
pub const SMOKE_CHOKE_AIR_PER_TICK: i32 = 6;
pub const SMOKE_LINGER_RAIN_FACTOR: f64 = 0.7;
pub const SMOKE_LINGER_THUNDER_FACTOR: f64 = 0.55;

/// `SmokeArmAbility.java` constants.
pub const SMOKE_ARM_ENERGY: f32 = 8.0;
pub const SMOKE_ARM_COOLDOWN_TICKS: i32 = 60;
pub const SMOKE_ARM_UNLOCK_MASTERY: u32 = 1;
pub const SMOKE_ARM_RANGE: f32 = 18.0;
pub const SMOKE_ARM_KNOCKBACK: f32 = 0.4;
pub const SMOKE_ARM_OUT_SPEED: f64 = 3.0;
pub const SMOKE_ARM_BACK_SPEED: f64 = 4.0;
pub const SMOKE_ARM_HOMING_DEGREES: f64 = 6.0;
pub const SMOKE_ARM_RADIUS: f64 = 0.7;
pub const SMOKE_ARM_RETURN_FACTOR: f32 = 0.5;
pub const SMOKE_ARM_PUNCH_FACTOR: f32 = 4.0;

/// `WhiteBlowAbility.java` constants.
pub const WHITE_BLOW_ENERGY: f32 = 18.0;
pub const WHITE_BLOW_COOLDOWN_TICKS: i32 = 160;
pub const WHITE_BLOW_UNLOCK_MASTERY: u32 = 2;
pub const WHITE_BLOW_DAMAGE: f32 = 12.0;
pub const WHITE_BLOW_RANGE: f32 = 10.0;
pub const WHITE_BLOW_KNOCKBACK: f32 = 3.0;
pub const WHITE_BLOW_WINDUP_TICKS: i32 = 8;
pub const WHITE_BLOW_FIST_RADIUS: f64 = 1.0;
pub const WHITE_BLOW_LAUNCH_BLOCKS: f64 = 12.0;
pub const WHITE_BLOW_SLAM_DAMAGE: f32 = 4.0;
pub const WHITE_BLOW_SLAM_STUN_TICKS: i32 = 20;

/// `WhiteLauncherAbility.java` constants.
pub const WHITE_LAUNCHER_ENERGY: f32 = 26.0;
pub const WHITE_LAUNCHER_COOLDOWN_TICKS: i32 = 240;
pub const WHITE_LAUNCHER_UNLOCK_MASTERY: u32 = 3;
pub const WHITE_LAUNCHER_DAMAGE: f32 = 14.0;
pub const WHITE_LAUNCHER_RANGE: f32 = 28.0;
pub const WHITE_LAUNCHER_KNOCKBACK: f32 = 1.6;
pub const WHITE_LAUNCHER_WINDUP_TICKS: i32 = 10;
pub const WHITE_LAUNCHER_SPEED: f64 = 2.2;
pub const WHITE_LAUNCHER_START_WIDTH: f64 = 1.0;
pub const WHITE_LAUNCHER_END_WIDTH: f64 = 4.0;
pub const WHITE_LAUNCHER_CLOUD_SECONDS: f64 = 4.0;
pub const WHITE_LAUNCHER_CLOUD_TICKS: i32 = 80;
pub const WHITE_LAUNCHER_CLOUD_RADIUS: f64 = 3.5;

/// `SmokeFlightAbility.java` & `SmokeFlight.java` (`smoke_flight` / `white_spark`) constants.
pub const SMOKE_FLIGHT_ENERGY: f32 = 10.0;
pub const SMOKE_FLIGHT_COOLDOWN_TICKS: i32 = 100;
pub const SMOKE_FLIGHT_UNLOCK_MASTERY: u32 = 4;
pub const SMOKE_FLIGHT_KNOCKBACK: f32 = 0.6;
pub const SMOKE_FLIGHT_SPEED: f64 = 1.6;
pub const SMOKE_FLIGHT_BOOST_SPEED: f64 = 3.0;
pub const SMOKE_FLIGHT_HARD_CAP: f64 = 9.5;
pub const SMOKE_FLIGHT_RAMP_TICKS: i32 = 10;
pub const SMOKE_FLIGHT_DRAIN_PER_SECOND: f32 = 4.0;
pub const SMOKE_FLIGHT_MAX_SECONDS: f64 = 60.0;

/// `WhiteSnakeAbility.java` constants.
pub const WHITE_SNAKE_ENERGY: f32 = 15.0;
pub const WHITE_SNAKE_COOLDOWN_TICKS: i32 = 280;
pub const WHITE_SNAKE_UNLOCK_MASTERY: u32 = 5;
pub const WHITE_SNAKE_DAMAGE: f32 = 2.0;
pub const WHITE_SNAKE_RANGE: f32 = 24.0;
pub const WHITE_SNAKE_SPEED: f64 = 2.4;
pub const WHITE_SNAKE_CONE_DEGREES: f64 = 8.0;
pub const WHITE_SNAKE_PULL_TICKS: i32 = 8;
pub const WHITE_SNAKE_HOLD_DISTANCE: f64 = 3.0;
pub const WHITE_SNAKE_MAX_SECONDS: f64 = 6.0;
pub const WHITE_SNAKE_MAX_CHANNEL_TICKS: i32 = 148; // 20 + 8 + 6*20
pub const WHITE_SNAKE_DRAIN_PER_SECOND: f32 = 6.0;
pub const WHITE_SNAKE_BREAK_DISTANCE: f64 = 30.0;
pub const WHITE_SNAKE_MAX_STRUGGLE_CUT: f32 = 0.5;
pub const WHITE_SNAKE_STRUGGLE_PRESSES: f32 = 24.0;

/// `WhiteChaseAbility.java` (`white_chase` / `white_vine`) constants.
pub const WHITE_CHASE_ENERGY: f32 = 28.0;
pub const WHITE_CHASE_COOLDOWN_TICKS: i32 = 320;
pub const WHITE_CHASE_UNLOCK_MASTERY: u32 = 7;
pub const WHITE_CHASE_DAMAGE: f32 = 3.0;
pub const WHITE_CHASE_RANGE: f32 = 20.0;
pub const WHITE_CHASE_MAX_TARGETS: usize = 3;
pub const WHITE_CHASE_CONE_DEGREES: f64 = 20.0;
pub const WHITE_CHASE_SPEED: f64 = 1.6;
pub const WHITE_CHASE_BIND_TICKS: i32 = 60; // 3.0s * 20
pub const WHITE_CHASE_MAX_STRUGGLE_CUT: f32 = 0.5;
pub const WHITE_CHASE_STRUGGLE_PRESSES: f32 = 14.0;

/// `WhiteOutAbility.java` constants.
pub const WHITE_OUT_ENERGY: f32 = 90.0;
pub const WHITE_OUT_COOLDOWN_TICKS: i32 = 2400;
pub const WHITE_OUT_UNLOCK_MASTERY: u32 = 9;
pub const WHITE_OUT_DAMAGE: f32 = 1.0;
pub const WHITE_OUT_RANGE: f32 = 20.0;
pub const WHITE_OUT_POUR_TICKS: i32 = 12;
pub const WHITE_OUT_GROW_TICKS: i32 = 30;
pub const WHITE_OUT_STAND_TICKS: i32 = 200; // 10.0s * 20
pub const WHITE_OUT_BIND_RADIUS: f64 = 5.0;
pub const WHITE_OUT_BIND_TICKS: i32 = 60; // 3.0s * 20
pub const WHITE_OUT_AMBUSH_MULT: f32 = 1.25;
pub const WHITE_OUT_SNAKE_REACH_MULT: f64 = 1.5;
pub const WHITE_OUT_VISIBILITY: f64 = 3.5;
pub const WHITE_OUT_SERVER_STRIDE: i32 = 4;

/// Canonical Moku-Moku VFX resource IDs (`MokuFx.java`).
pub mod moku_fx {
    pub const AWAKEN: &str = "devilfruits:moku_moku/awaken";
    pub const LEVEL_UP: &str = "devilfruits:moku_moku/level_up";
    pub const REINCARNATE: &str = "devilfruits:moku_moku/reincarnate";
    pub const ARM: &str = "devilfruits:moku_moku/smoke_arm";
    pub const ARM_HIT: &str = "devilfruits:moku_moku/smoke_arm_hit";
    pub const BLOW_WINDUP: &str = "devilfruits:moku_moku/white_blow_windup";
    pub const BLOW: &str = "devilfruits:moku_moku/white_blow";
    pub const BLOW_LAUNCHED: &str = "devilfruits:moku_moku/white_blow_launched";
    pub const BLOW_SLAM: &str = "devilfruits:moku_moku/white_blow_slam";
    pub const LAUNCHER_CHARGE: &str = "devilfruits:moku_moku/white_launcher_charge";
    pub const LAUNCHER: &str = "devilfruits:moku_moku/white_launcher";
    pub const LAUNCHER_BURST: &str = "devilfruits:moku_moku/white_launcher_burst";
    pub const FLIGHT_START: &str = "devilfruits:moku_moku/smoke_flight_start";
    pub const FLIGHT_END: &str = "devilfruits:moku_moku/smoke_flight_end";
    pub const SNAKE: &str = "devilfruits:moku_moku/white_snake";
    pub const SNAKE_CATCH: &str = "devilfruits:moku_moku/white_snake_catch";
    pub const SNAKE_END: &str = "devilfruits:moku_moku/white_snake_end";
    pub const CHASE: &str = "devilfruits:moku_moku/white_chase";
    pub const CHASE_BIND: &str = "devilfruits:moku_moku/white_chase_bind";
    pub const WHITE_OUT: &str = "devilfruits:moku_moku/white_out";
    pub const WHITE_OUT_END: &str = "devilfruits:moku_moku/white_out_end";
    pub const COUGH: &str = "devilfruits:moku_moku/cough";
}

/// Canonical Moku-Moku animation clip IDs.
pub mod moku_anim {
    pub const SMOKE_ARM_FIRE: &str = "devilfruits:moku_moku/smoke_arm_fire";
    pub const SMOKE_ARM_RETURN: &str = "devilfruits:moku_moku/smoke_arm_return";
    pub const WHITE_BLOW: &str = "devilfruits:moku_moku/white_blow";
    pub const WHITE_LAUNCHER: &str = "devilfruits:moku_moku/white_launcher";
    pub const FLIGHT_ENTER: &str = "devilfruits:moku_moku/flight_enter";
    pub const FLIGHT_LOOP: &str = "devilfruits:moku_moku/flight_loop";
    pub const WHITE_SNAKE_CAST: &str = "devilfruits:moku_moku/white_snake_cast";
    pub const WHITE_SNAKE_HOLD: &str = "devilfruits:moku_moku/white_snake_hold";
    pub const WHITE_SNAKE_RELEASE: &str = "devilfruits:moku_moku/white_snake_release";
    pub const WHITE_CHASE: &str = "devilfruits:moku_moku/white_chase";
    pub const WHITE_OUT: &str = "devilfruits:moku_moku/white_out";
}

/// Moku Moku no Mi Devil Fruit module (`MokuMoku.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MokuMokuModule;

impl MokuMokuModule {
    pub const fn new() -> Self {
        Self
    }

    /// Exact Restraint rules for `WhiteSnakeAbility` (`carry = true`, `leash = 30.0`, `struggle(0.5, 24.0)`).
    pub fn white_snake_restraint_rules() -> RestraintRules {
        RestraintRules {
            kind: "devilfruits:moku_moku/white_snake".into(),
            ticks: WHITE_SNAKE_PULL_TICKS + (WHITE_SNAKE_MAX_SECONDS * 20.0) as i32,
            max_struggle_cut: WHITE_SNAKE_MAX_STRUGGLE_CUT,
            struggle_presses: WHITE_SNAKE_STRUGGLE_PRESSES,
            leash: WHITE_SNAKE_BREAK_DISTANCE,
            carried: true,
            haki_breaks: true,
            seastone_breaks: true,
        }
    }

    /// Exact Restraint rules for `WhiteChaseAbility` / `white_vine` (`ticks = 60`, `struggle(0.5, 14.0)`).
    pub fn white_chase_restraint_rules() -> RestraintRules {
        RestraintRules {
            kind: "devilfruits:moku_moku/white_chase".into(),
            ticks: WHITE_CHASE_BIND_TICKS,
            max_struggle_cut: WHITE_CHASE_MAX_STRUGGLE_CUT,
            struggle_presses: WHITE_CHASE_STRUGGLE_PRESSES,
            leash: 0.0,
            carried: false,
            haki_breaks: true,
            seastone_breaks: true,
        }
    }

    /// Exact Restraint rules for `WhiteOutAbility` centre bind (`ticks = 60`, `struggle(0.5, 14.0)`).
    pub fn white_out_restraint_rules() -> RestraintRules {
        RestraintRules {
            kind: "devilfruits:moku_moku/white_out".into(),
            ticks: WHITE_OUT_BIND_TICKS,
            max_struggle_cut: 0.5,
            struggle_presses: 14.0,
            leash: 0.0,
            carried: false,
            haki_breaks: true,
            seastone_breaks: true,
        }
    }

    /// Dissolves limbs briefly (`SmokeTechnique.smokeArm` / `smokeArms` / `smokeLegs`).
    fn dissolve_limbs(ctx: &mut FruitContext<'_>, limb_mask: u8, ticks: i32) {
        let now = ctx.ability_ctx.tick;
        if let Some(logia) = ctx.logia.as_deref_mut() {
            for idx in 0..limbs::COUNT {
                if (limb_mask & limbs::bit(idx)) != 0 {
                    logia.dissolve_limb(idx, now, ticks);
                }
            }
        }
    }

    /// Normalizes ability alias names (`"white_vine"` -> `"white_chase"` or vice versa, `"white_spark"` -> `"smoke_flight"`).
    fn canonical_ability_name(name: &str) -> &str {
        let stripped = name
            .split(':')
            .next_back()
            .unwrap_or(name)
            .split('/')
            .next_back()
            .unwrap_or(name);
        match stripped {
            "white_chase" => "white_vine",
            "white_spark" => "smoke_flight",
            other => other,
        }
    }
}

impl DevilFruitModule for MokuMokuModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::MokuMoku
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let allowed = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "smoke_arm"),
                display_name: "Smoke Arm".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SMOKE_ARM_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SMOKE_ARM_COOLDOWN_TICKS,
                    unlock_mastery: SMOKE_ARM_UNLOCK_MASTERY,
                    damage: SMOKE_ARM_PUNCH_FACTOR,
                    range: SMOKE_ARM_RANGE,
                    knockback: SMOKE_ARM_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "white_blow"),
                display_name: "White Blow".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: WHITE_BLOW_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: WHITE_BLOW_COOLDOWN_TICKS,
                    unlock_mastery: WHITE_BLOW_UNLOCK_MASTERY,
                    damage: WHITE_BLOW_DAMAGE,
                    range: WHITE_BLOW_RANGE,
                    knockback: WHITE_BLOW_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "white_launcher"),
                display_name: "White Launcher".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: WHITE_LAUNCHER_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: WHITE_LAUNCHER_COOLDOWN_TICKS,
                    unlock_mastery: WHITE_LAUNCHER_UNLOCK_MASTERY,
                    damage: WHITE_LAUNCHER_DAMAGE,
                    range: WHITE_LAUNCHER_RANGE,
                    knockback: WHITE_LAUNCHER_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "smoke_flight"),
                display_name: "Smoke Flight".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SMOKE_FLIGHT_ENERGY,
                    energy_per_second: SMOKE_FLIGHT_DRAIN_PER_SECOND,
                    cooldown_ticks: SMOKE_FLIGHT_COOLDOWN_TICKS,
                    unlock_mastery: SMOKE_FLIGHT_UNLOCK_MASTERY,
                    damage: 0.0,
                    range: 0.0,
                    knockback: SMOKE_FLIGHT_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "white_snake"),
                display_name: "White Snake".into(),
                cast_type: CastType::Held,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: WHITE_SNAKE_ENERGY,
                    energy_per_second: WHITE_SNAKE_DRAIN_PER_SECOND,
                    cooldown_ticks: WHITE_SNAKE_COOLDOWN_TICKS,
                    unlock_mastery: WHITE_SNAKE_UNLOCK_MASTERY,
                    damage: WHITE_SNAKE_DAMAGE,
                    range: WHITE_SNAKE_RANGE,
                    knockback: 0.0,
                    max_channel_ticks: WHITE_SNAKE_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "white_vine"),
                display_name: "White Vine".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: WHITE_CHASE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: WHITE_CHASE_COOLDOWN_TICKS,
                    unlock_mastery: WHITE_CHASE_UNLOCK_MASTERY,
                    damage: WHITE_CHASE_DAMAGE,
                    range: WHITE_CHASE_RANGE,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MokuMoku, "white_out"),
                display_name: "White Out".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: WHITE_OUT_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: WHITE_OUT_COOLDOWN_TICKS,
                    unlock_mastery: WHITE_OUT_UNLOCK_MASTERY,
                    damage: WHITE_OUT_DAMAGE,
                    range: WHITE_OUT_RANGE,
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
            damage_type: Some(GrandLineDamageType::MokuSmoke),
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
        let base_scaled_dmg = def.stats.scaled_damage(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        );
        let scaled_range = def.stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) as f64;

        match ability_name {
            "smoke_arm" => {
                res.animation_clip = Some(moku_anim::SMOKE_ARM_FIRE.into());
                let target = CombatTargeting::aimed(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    SMOKE_ARM_HOMING_DEGREES.to_radians(),
                );
                let head = target
                    .map(|t| t.center)
                    .unwrap_or(eye + look * (scaled_range - 0.2).max(1.5));
                let dist = eye.distance(head);
                let out_ticks = ((dist / SMOKE_ARM_OUT_SPEED).ceil() as i32).max(1);
                let back_ticks = ((dist / SMOKE_ARM_BACK_SPEED).ceil() as i32).max(1);
                Self::dissolve_limbs(ctx, limbs::RIGHT_ARM, out_ticks + back_ticks + 4);

                let flags = (if target.is_some() { 1 } else { 0 }) | 4 | (back_ticks << 8);
                ctx.world_fx.emit_fx(
                    moku_fx::ARM,
                    0,
                    eye,
                    head,
                    out_ticks as f32,
                    now,
                    flags,
                    96.0,
                );

                // Outward punch hit (`punchFactor = 4.0`) + return capsule hit (`returnFactor = 0.5`).
                if let Some(t) = target {
                    let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.1, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MokuSmoke,
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
                    ctx.world_fx
                        .emit_fx(moku_fx::ARM_HIT, 0, t.center, look, 1.0, now, 0, 64.0);
                }

                let return_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    head,
                    eye,
                    SMOKE_ARM_RADIUS,
                );
                for t in return_hits {
                    if !res.hits.iter().any(|h| h.target_id == t.id) {
                        let pull = -look * (def.stats.knockback as f64 * 0.4);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg * SMOKE_ARM_RETURN_FACTOR,
                            damage_type: GrandLineDamageType::MokuSmoke,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                pull,
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
            "white_blow" => {
                res.animation_clip = Some(moku_anim::WHITE_BLOW.into());
                Self::dissolve_limbs(ctx, limbs::RIGHT_ARM, WHITE_BLOW_WINDUP_TICKS + 14);

                ctx.world_fx.emit_fx(
                    moku_fx::BLOW_WINDUP,
                    0,
                    eye,
                    look,
                    WHITE_BLOW_FIST_RADIUS as f32,
                    now,
                    WHITE_BLOW_WINDUP_TICKS,
                    96.0,
                );

                // Seek target along capsule (`radius = 1.0`) or 4° cone (`WhiteBlowAbility.activate`).
                let capsule_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    eye + look * scaled_range,
                    WHITE_BLOW_FIST_RADIUS,
                );
                let target = capsule_hits.first().copied().or_else(|| {
                    CombatTargeting::aimed(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        look,
                        scaled_range,
                        4.0_f64.to_radians(),
                    )
                });
                let fist_end = target.map(|t| t.center).unwrap_or(eye + look * scaled_range);

                ctx.world_fx.emit_fx(
                    moku_fx::BLOW,
                    0,
                    eye,
                    fist_end,
                    WHITE_BLOW_FIST_RADIUS as f32,
                    now,
                    if target.is_some() { 1 } else { 0 },
                    96.0,
                );

                if let Some(t) = target {
                    // `v = launchBlocks * 0.085 = 12.0 * 0.085 = 1.02`, upward `0.38`.
                    let flat = if horiz_look.length_squared() > 1e-6 {
                        horiz_look
                    } else {
                        DVec3::Z
                    };
                    let v = WHITE_BLOW_LAUNCH_BLOCKS * 0.085;
                    let launch_impulse = flat * v + DVec3::new(0.0, 0.38, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MokuSmoke,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch_impulse,
                            t.knockback_resistance,
                        ),
                        stun_ticks: WHITE_BLOW_SLAM_STUN_TICKS,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx.emit_fx(
                        moku_fx::BLOW_LAUNCHED,
                        0,
                        t.center,
                        flat,
                        v as f32,
                        now,
                        24,
                        96.0,
                    );
                }
            }
            "white_launcher" => {
                res.animation_clip = Some(moku_anim::WHITE_LAUNCHER.into());
                res.spawned_projectiles = 1;
                Self::dissolve_limbs(ctx, limbs::ARMS, WHITE_LAUNCHER_WINDUP_TICKS + 16);

                let muzzle = eye + look * 0.9 + DVec3::new(0.0, -0.35, 0.0);
                let end = muzzle + look * scaled_range;
                let ticks = ((scaled_range / WHITE_LAUNCHER_SPEED).ceil() as i32).max(1);
                let packed_extra =
                    ticks | (((WHITE_LAUNCHER_END_WIDTH * 10.0).round() as i32) << 12);

                ctx.world_fx.emit_fx(
                    moku_fx::LAUNCHER_CHARGE,
                    0,
                    eye,
                    look,
                    1.0,
                    now,
                    WHITE_LAUNCHER_WINDUP_TICKS,
                    96.0,
                );
                // Tube / expanding smoke jet VFX (`MokuFx.LAUNCHER`) + volume cloud burst (`MokuFx.LAUNCHER_BURST`).
                ctx.world_fx.emit_fx(
                    moku_fx::LAUNCHER,
                    0,
                    muzzle,
                    look * WHITE_LAUNCHER_SPEED,
                    WHITE_LAUNCHER_START_WIDTH as f32,
                    now,
                    packed_extra,
                    128.0,
                );
                ctx.world_fx.emit_fx(
                    moku_fx::LAUNCHER_BURST,
                    0,
                    end,
                    look,
                    WHITE_LAUNCHER_CLOUD_RADIUS as f32,
                    now,
                    WHITE_LAUNCHER_CLOUD_TICKS,
                    128.0,
                );
                ctx.world_fx.emit_shake(end, 0.25, 14);
                res.screen_shake = Some((0.25, 14));

                let hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    muzzle,
                    end,
                    WHITE_LAUNCHER_END_WIDTH * 0.5,
                );
                let push =
                    look * (0.35 * def.stats.knockback as f64) + DVec3::new(0.0, 0.12, 0.0);
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MokuSmoke,
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
            }
            "smoke_flight" => match event {
                AbilityEvent::Toggled { enabled, .. } => {
                    res.cast_result = Some(CastResult::Toggled);
                    ctx.form_runtime.set_flag(form_flags::FLYING, *enabled);
                    if *enabled {
                        res.animation_clip = Some(moku_anim::FLIGHT_ENTER.into());
                        res.caster_velocity_impulse = Some(look * SMOKE_FLIGHT_SPEED);
                        let boost_packed = ((SMOKE_FLIGHT_BOOST_SPEED * 10.0).round() as i32)
                            | (SMOKE_FLIGHT_RAMP_TICKS << 12);
                        ctx.world_fx.emit_fx(
                            moku_fx::FLIGHT_START,
                            0,
                            eye,
                            look,
                            SMOKE_FLIGHT_SPEED as f32,
                            now,
                            boost_packed,
                            96.0,
                        );
                    } else {
                        ctx.world_fx.emit_fx(
                            moku_fx::FLIGHT_END,
                            0,
                            eye,
                            DVec3::ZERO,
                            1.0,
                            now,
                            0,
                            96.0,
                        );
                    }
                }
                AbilityEvent::ToggleTick { .. } => {
                    res.cast_result = Some(CastResult::Toggled);
                    res.animation_clip = Some(moku_anim::FLIGHT_LOOP.into());
                }
                _ => {}
            },
            "white_snake" => match event {
                AbilityEvent::ChannelStarted { .. } | AbilityEvent::CastInstant { .. } => {
                    res.cast_result = Some(CastResult::Channelling);
                    res.animation_clip = Some(moku_anim::WHITE_SNAKE_CAST.into());
                    res.spawned_projectiles = 1;
                    Self::dissolve_limbs(ctx, limbs::RIGHT_ARM, WHITE_SNAKE_MAX_CHANNEL_TICKS);

                    let hand = eye + look * 0.6;
                    let target = CombatTargeting::aimed(
                        ctx.nearby_targets,
                        ctx.player_id,
                        hand,
                        look,
                        scaled_range,
                        WHITE_SNAKE_CONE_DEGREES.to_radians(),
                    );
                    let end = target.map(|t| t.center).unwrap_or(hand + look * scaled_range);
                    let dist = hand.distance(end);
                    let arrive = ((dist / WHITE_SNAKE_SPEED).ceil() as i32).max(1);

                    // Smoke tube serpent VFX (`MokuFx.SNAKE`).
                    ctx.world_fx.emit_fx(
                        moku_fx::SNAKE,
                        0,
                        hand,
                        end,
                        dist as f32,
                        now,
                        arrive,
                        128.0,
                    );

                    if let Some(t) = target {
                        let hold_ticks =
                            WHITE_SNAKE_PULL_TICKS + (WHITE_SNAKE_MAX_SECONDS * 20.0) as i32;
                        let pull_spot = eye + horiz_look * WHITE_SNAKE_HOLD_DISTANCE;
                        let pull_vel = (pull_spot - t.center) * 0.45;
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::MokuChoke,
                            knockback_impulse: pull_vel,
                            stun_ticks: hold_ticks,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                        ctx.world_fx.emit_fx(
                            moku_fx::SNAKE_CATCH,
                            0,
                            t.center,
                            DVec3::ZERO,
                            WHITE_SNAKE_PULL_TICKS as f32,
                            now,
                            0,
                            128.0,
                        );
                    }
                }
                AbilityEvent::ChannelTick { elapsed_ticks, .. } => {
                    res.cast_result = Some(CastResult::Channelling);
                    res.damage_type = Some(GrandLineDamageType::MokuChoke);
                    res.animation_clip = Some(moku_anim::WHITE_SNAKE_HOLD.into());
                    if (*elapsed_ticks % 20) == 0 {
                        let target = CombatTargeting::aimed(
                            ctx.nearby_targets,
                            ctx.player_id,
                            eye,
                            look,
                            scaled_range,
                            WHITE_SNAKE_CONE_DEGREES.to_radians(),
                        );
                        if let Some(t) = target {
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: base_scaled_dmg,
                                damage_type: GrandLineDamageType::MokuChoke,
                                knockback_impulse: DVec3::ZERO,
                                stun_ticks: 20,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                            ctx.world_fx.emit_fx(
                                moku_fx::COUGH,
                                0,
                                t.center,
                                DVec3::ZERO,
                                1.0,
                                now,
                                0,
                                32.0,
                            );
                        }
                    }
                }
                AbilityEvent::ChannelEnded { .. } => {
                    res.animation_clip = Some(moku_anim::WHITE_SNAKE_RELEASE.into());
                    ctx.world_fx.emit_fx(
                        moku_fx::SNAKE_END,
                        0,
                        eye + look * WHITE_SNAKE_HOLD_DISTANCE,
                        DVec3::ZERO,
                        0.0,
                        now,
                        1,
                        128.0,
                    );
                }
                _ => {}
            },
            "white_vine" => {
                res.animation_clip = Some(moku_anim::WHITE_CHASE.into());
                Self::dissolve_limbs(ctx, limbs::ARMS, 26);

                let targets = CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    WHITE_CHASE_CONE_DEGREES.to_radians(),
                );
                let selected: Vec<_> = targets.into_iter().take(WHITE_CHASE_MAX_TARGETS).collect();
                res.spawned_projectiles = selected.len();

                for (i, t) in selected.into_iter().enumerate() {
                    let chase_ticks =
                        6 + ((eye.distance(t.center) / WHITE_CHASE_SPEED).ceil() as i32).max(2);
                    // Smoke tendril tube VFX (`MokuFx.CHASE`) + restraint bind VFX (`MokuFx.CHASE_BIND`).
                    ctx.world_fx.emit_fx(
                        moku_fx::CHASE,
                        0,
                        eye,
                        t.center,
                        chase_ticks as f32,
                        now + (i as i64) * 7919,
                        0,
                        128.0,
                    );
                    ctx.world_fx.emit_fx(
                        moku_fx::CHASE_BIND,
                        0,
                        t.center,
                        DVec3::ZERO,
                        t.height as f32,
                        now,
                        WHITE_CHASE_BIND_TICKS,
                        96.0,
                    );
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MokuSmoke,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: WHITE_CHASE_BIND_TICKS,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
            }
            "white_out" => {
                res.damage_type = Some(GrandLineDamageType::MokuChoke);
                res.animation_clip = Some(moku_anim::WHITE_OUT.into());
                Self::dissolve_limbs(ctx, limbs::ARMS, WHITE_OUT_POUR_TICKS + 6);

                let packed = WHITE_OUT_POUR_TICKS
                    | (WHITE_OUT_GROW_TICKS << 8)
                    | (WHITE_OUT_STAND_TICKS << 16);
                // Volume dome VFX (`MokuFx.WHITE_OUT`).
                ctx.world_fx.emit_fx(
                    moku_fx::WHITE_OUT,
                    0,
                    eye,
                    DVec3::ZERO,
                    scaled_range as f32,
                    now,
                    packed,
                    scaled_range + 96.0,
                );

                // Targets inside `range` are choked & slowed; targets within `bindRadius` (5.0) are restrained for 60 ticks.
                let dome_hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    scaled_range,
                );
                for t in dome_hits {
                    let dist = t.center.distance(eye);
                    let bound = dist <= WHITE_OUT_BIND_RADIUS;
                    if bound {
                        ctx.world_fx.emit_fx(
                            moku_fx::CHASE_BIND,
                            0,
                            t.center,
                            DVec3::ZERO,
                            t.height as f32,
                            now,
                            WHITE_OUT_BIND_TICKS,
                            96.0,
                        );
                    }
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MokuChoke,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: if bound { WHITE_OUT_BIND_TICKS } else { 0 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
            }
            _ => {}
        }

        res.vfx_events = ctx.world_fx.drain_vfx();
        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        if ctx.suppressed || ctx.submerged_in_water {
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
        if form_runtime.has_flag(form_flags::FLYING) {
            mods.reset_above_ground_ticks = true;
            mods.passes_through_leaves = true;
        }
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ability::{AbilityContext, AbilityModifiers, CastBudget, TechniqueHeat};
    use crate::core::combat::{CombatStatus, TargetCandidate};
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::logia::LogiaData;
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
    fn moku_moku_abilities_and_logia_profile_match_java() {
        let module = MokuMokuModule::new();
        assert_eq!(module.fruit_id(), FruitId::MokuMoku);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::MokuSmoke);

        let profile = module.logia_profile().expect("Moku-Moku is a Logia");
        assert_eq!(profile.element, "smoke");
        assert_eq!(profile.elemental_drain_per_second, 3.0);
        assert_eq!(profile.dash_range, 28.0);
        assert_eq!(profile.dash_speed, 3.5);
        assert!(profile.environment.rain_opposes);
        assert_eq!(profile.environment.opposed_regen_mult, 0.7);

        let white_blow = module.ability("white_blow").unwrap();
        assert_eq!(white_blow.stats.energy_cost, 18.0);
        assert_eq!(white_blow.stats.cooldown_ticks, 160);
        assert_eq!(white_blow.stats.unlock_mastery, 2);
        assert_eq!(white_blow.stats.damage, 12.0);
        assert_eq!(white_blow.stats.range, 10.0);
        assert_eq!(white_blow.stats.knockback, 3.0);

        let white_launcher = module.ability("white_launcher").unwrap();
        assert_eq!(white_launcher.stats.energy_cost, 26.0);
        assert_eq!(white_launcher.stats.cooldown_ticks, 240);
        assert_eq!(white_launcher.stats.unlock_mastery, 3);
        assert_eq!(white_launcher.stats.damage, 14.0);
        assert_eq!(white_launcher.stats.range, 28.0);

        let white_snake = module.ability("white_snake").unwrap();
        assert_eq!(white_snake.cast_type, CastType::Held);
        assert_eq!(white_snake.stats.energy_cost, 15.0);
        assert_eq!(white_snake.stats.cooldown_ticks, 280);
        assert_eq!(white_snake.stats.unlock_mastery, 5);
        assert_eq!(white_snake.stats.range, 24.0);

        // Both "white_vine" and "white_chase" resolve to White Vine / White Chase.
        let white_vine = module.ability("white_vine").unwrap();
        let white_chase = module.ability("white_chase").unwrap();
        assert_eq!(white_vine, white_chase);
        assert_eq!(white_vine.stats.energy_cost, 28.0);
        assert_eq!(white_vine.stats.cooldown_ticks, 320);
        assert_eq!(white_vine.stats.unlock_mastery, 7);

        // Both "smoke_flight" and "white_spark" resolve to Smoke Flight.
        let flight = module.ability("white_spark").unwrap();
        assert_eq!(flight.id.name, "smoke_flight");
        assert_eq!(flight.cast_type, CastType::Toggle);
        assert_eq!(flight.stats.energy_cost, 10.0);
        assert_eq!(flight.stats.energy_per_second, 4.0);

        let white_out = module.ability("white_out").unwrap();
        assert!(white_out.is_ultimate);
        assert_eq!(white_out.stats.energy_cost, 90.0);
        assert_eq!(white_out.stats.cooldown_ticks, 2400);
        assert_eq!(white_out.stats.unlock_mastery, 9);
        assert_eq!(white_out.stats.range, 20.0);
    }

    #[test]
    fn moku_moku_execute_all_abilities_restraint_and_vfx() {
        let module = MokuMokuModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::MokuMoku),
            mastery: 10,
            energy: 290.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = vec![
            make_target(2, DVec3::new(0.0, 64.0, 4.0)),
            make_target(3, DVec3::new(1.0, 64.0, 12.0)),
        ];

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
                fruit: FruitId::MokuMoku,
                ability_name: "white_blow".into(),
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

        // 1. White Blow launches target with stun_ticks = 20.
        let r_blow = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "white_blow".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_blow.hits.len(), 1);
        assert_eq!(r_blow.hits[0].stun_ticks, WHITE_BLOW_SLAM_STUN_TICKS);
        assert!(r_blow.hits[0].knockback_impulse.z > 0.9);

        // 2. White Launcher emits tube & cloud burst VFX and hits both targets in expanding jet.
        let r_launcher = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "white_launcher".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_launcher.hits.len(), 2);
        assert!(
            r_launcher
                .vfx_events
                .iter()
                .any(|e| e.fx_id == moku_fx::LAUNCHER)
        );
        assert!(
            r_launcher
                .vfx_events
                .iter()
                .any(|e| e.fx_id == moku_fx::LAUNCHER_BURST)
        );

        // 3. White Snake catches target, pulls them in, and applies choke hold.
        let r_snake = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ChannelStarted {
                ability: "white_snake".into(),
            },
        );
        assert_eq!(r_snake.hits.len(), 1);
        assert_eq!(r_snake.hits[0].damage_type, GrandLineDamageType::MokuChoke);
        assert!(
            r_snake
                .vfx_events
                .iter()
                .any(|e| e.fx_id == moku_fx::SNAKE_CATCH)
        );

        // Verify White Snake RestraintRules struggle mechanics.
        let mut status = CombatStatus::default();
        assert!(status.apply_restraint(
            Some([1; 16]),
            MokuMokuModule::white_snake_restraint_rules(),
            false,
            100
        ));
        assert!(status.is_restrained(110));

        // 4. White Vine / White Chase binds up to 3 targets in cone for 60 ticks.
        let r_vine = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "white_vine".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_vine.hits.len(), 2);
        assert_eq!(r_vine.hits[0].stun_ticks, WHITE_CHASE_BIND_TICKS);

        // 5. White Out creates dome volume VFX, binds target within 5 blocks, and chokes outer target.
        let r_out = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "white_out".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_out.hits.len(), 2);
        assert_eq!(r_out.hits[0].stun_ticks, WHITE_OUT_BIND_TICKS); // dist = 4.0 <= 5.0
        assert_eq!(r_out.hits[1].stun_ticks, 0); // dist > 5.0
        assert!(
            r_out
                .vfx_events
                .iter()
                .any(|e| e.fx_id == moku_fx::WHITE_OUT)
        );

        // 6. Smoke Flight toggle sets FLYING flag and body_modifiers.
        let r_flight = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "smoke_flight".into(),
                enabled: true,
            },
        );
        assert_eq!(r_flight.cast_result, Some(CastResult::Toggled));
        assert!(ctx.form_runtime.has_flag(form_flags::FLYING));
        let mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(mods.reset_above_ground_ticks);
        assert!(mods.passes_through_leaves);
    }
}
