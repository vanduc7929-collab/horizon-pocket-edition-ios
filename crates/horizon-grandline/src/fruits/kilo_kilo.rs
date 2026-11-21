//! Kilo Kilo no Mi (Kilo-Kilo Fruit) — 1:1 port of `com.devilfruits.fruits.kilo_kilo`.
//!
//! Includes `kilo_press_10000` (`press` / `ten_thousand_kilo_press`),
//! `light_feather` (`one_kilo`), `kilo_slingshot`, `heavy_stomp`,
//! `WeightCurves` dynamic `1.0 kg..=10,000.0 kg` logarithmic physics curves,
//! kinetic energy block smashing, terminal landing shockwave in `on_tick`,
//! and `EntityBodyModifiers` mass/immovability hooks in `body_modifiers`.

use glam::DVec3;
use horizon_world::grandline_hooks::{
    ENTITY_MASS_DEFAULT_KG, ENTITY_MASS_HEAVY_TARGET_KG, ENTITY_MASS_IMMOVABLE_KG,
    ENTITY_MASS_LIGHT_TARGET_KG, EntityBodyModifiers, EntityMassTier,
    is_mass_immovable, mass_tier_from_kg,
};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::{Mastery, PlayerFruitData};
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Java constants (`KiloKilo.java`, `WeightCurves.java`, `WeightManager.java`)
// ---------------------------------------------------------------------------

pub const KILO_MIN_KG: f64 = ENTITY_MASS_LIGHT_TARGET_KG; // 1.0
pub const KILO_NORMAL_KG: f64 = ENTITY_MASS_DEFAULT_KG; // 70.0
pub const KILO_MAX_KG: f64 = ENTITY_MASS_HEAVY_TARGET_KG; // 10000.0
pub const KILO_IMMOVABLE_KG: f64 = ENTITY_MASS_IMMOVABLE_KG; // 3000.0

pub const KILO_RAMP_SECONDS_FULL_RANGE: f64 = 3.0;
pub const KILO_LIGHT_JUMP_BLOCKS: f64 = 50.0;
pub const KILO_LIGHT_TERMINAL_SPEED: f64 = 0.1;
pub const KILO_GLIDE_FALL_SPEED: f64 = 0.05;
pub const KILO_AIR_CONTROL: f64 = 1.0;
pub const KILO_WIND_DRIFT: f64 = 0.012;
pub const KILO_HEAVY_GRAVITY_MULTIPLIER: f64 = 6.0;
pub const KILO_MAX_FALL_SPEED: f64 = 3.0;
pub const KILO_LIGHT_SPEED_BONUS: f64 = 0.2;
pub const KILO_HEAVY_SPEED_MULTIPLIER: f64 = 0.45;
pub const KILO_LIGHT_STEP_HEIGHT: f64 = 1.0;
pub const KILO_MAX_KNOCKBACK_MULTIPLIER: f64 = 8.0;
pub const KILO_LIGHT_DRAIN_PER_SECOND: f64 = 3.0;
pub const KILO_HEAVY_DRAIN_PER_SECOND: f64 = 4.0;
pub const KILO_DRAIN_DEAD_ZONE: f64 = 0.08;
pub const KILO_EXHAUSTED_RETURN_SECONDS: f64 = 2.0;
pub const KILO_FOOTSTEP_MIN_KG: f64 = 1000.0;
pub const KILO_BLOCK_ENERGY_KJ: f64 = 120.0;
pub const KILO_MAX_SMASH_DEPTH: f64 = 10.0;
pub const KILO_THIN_FLOOR_MAX_BLOCKS: i32 = 1;
pub const KILO_THIN_FLOOR_MIN_KG: f64 = 5000.0;
pub const KILO_THIN_FLOOR_SECONDS: f64 = 0.4;
pub const KILO_IMPACT_MIN_FALL: f64 = 2.5;
pub const KILO_IMPACT_MIN_ENERGY_KJ: f64 = 250.0;
pub const KILO_CRUSH_DAMAGE_CAP: f64 = 60.0;
pub const KILO_CONTACT_MIN_ENERGY_KJ: f64 = 40.0;
pub const KILO_CONTACT_COOLDOWN_TICKS: i64 = 10;
pub const KILO_PRESS_BURST_RADIUS: f64 = 3.0;
pub const KILO_HEAVY_IGNORES_PUSHING: bool = true;

/// `OneKiloAbility.java` (`one_kilo` / `light_feather`) constants.
pub const ONE_KILO_ENERGY: f32 = 4.0;
pub const ONE_KILO_COOLDOWN_TICKS: i32 = 10;
pub const ONE_KILO_UNLOCK_MASTERY: u32 = 1;

/// `KiloPressAbility.java` (`press` / `ten_thousand_kilo_press` / `kilo_press_10000`) constants.
pub const PRESS_ENERGY: f32 = 6.0;
pub const PRESS_COOLDOWN_TICKS: i32 = 10;
pub const PRESS_UNLOCK_MASTERY: u32 = 1;
pub const PRESS_MIN_HEIGHT: f64 = 6.0;
pub const PRESS_SWITCH_SECONDS: f64 = 0.35;
pub const PRESS_PLUNGE_EXTRA_ENERGY: f32 = 12.0;
pub const PRESS_PLUNGE_COOLDOWN_TICKS: i32 = 60;
pub const PRESS_MAX_BLOCKS_PER_TICK: usize = 24;
pub const PRESS_MAX_BLOCKS_PER_CAST: usize = 160;

/// `kilo_slingshot` & `heavy_stomp` technique constants.
pub const KILO_SLINGSHOT_ENERGY: f32 = 22.0;
pub const KILO_SLINGSHOT_COOLDOWN_TICKS: i32 = 110;
pub const KILO_SLINGSHOT_UNLOCK_MASTERY: u32 = 3;
pub const KILO_SLINGSHOT_DAMAGE: f32 = 24.0;
pub const KILO_SLINGSHOT_RANGE: f32 = 14.0;
pub const KILO_SLINGSHOT_KNOCKBACK: f32 = 1.8;

pub const HEAVY_STOMP_ENERGY: f32 = 26.0;
pub const HEAVY_STOMP_COOLDOWN_TICKS: i32 = 140;
pub const HEAVY_STOMP_UNLOCK_MASTERY: u32 = 4;
pub const HEAVY_STOMP_DAMAGE: f32 = 28.0;
pub const HEAVY_STOMP_RANGE: f32 = 7.5;
pub const HEAVY_STOMP_KNOCKBACK: f32 = 2.0;

/// Synced form flags used to track Kilo-Kilo weight mode & airborne plunge state.
pub const KILO_FLAG_LIGHT: u32 = form_flags::fruit_bit(0);
pub const KILO_FLAG_HEAVY: u32 = form_flags::fruit_bit(1);
pub const KILO_FLAG_PLUNGING: u32 = form_flags::fruit_bit(2);

/// Canonical Kilo-Kilo VFX identifiers (`KiloFx.java`).
pub mod kilo_fx {
    pub const AWAKEN: &str = "devilfruits:kilo_kilo/awaken";
    pub const RAMP_START: &str = "devilfruits:kilo_kilo/ramp_start";
    pub const RAMP_DONE: &str = "devilfruits:kilo_kilo/ramp_done";
    pub const LIGHT_JUMP: &str = "devilfruits:kilo_kilo/light_jump";
    pub const FOOTSTEP: &str = "devilfruits:kilo_kilo/footstep";
    pub const PRESS_START: &str = "devilfruits:kilo_kilo/press_start";
    pub const IMPACT: &str = "devilfruits:kilo_kilo/impact";
    pub const PUNCH: &str = "devilfruits:kilo_kilo/punch";
}

/// 1:1 port of `com.devilfruits.fruits.kilo_kilo.weight.WeightCurves`.
pub struct WeightCurves;

impl WeightCurves {
    pub const GRAVITY: f64 = 0.08;
    pub const JUMP: f64 = 0.42;
    pub const DRAG: f64 = 0.98;
    pub const STEP: f64 = 0.6;
    pub const SAFE_FALL: f64 = 3.0;

    pub fn clamp_kg(kg: f64) -> f64 {
        if kg.is_nan() {
            KILO_NORMAL_KG
        } else {
            kg.clamp(KILO_MIN_KG, KILO_MAX_KG)
        }
    }

    /// Normalized log weight scale in `-1.0..=1.0` (`0.0` at `70 kg`, `-1.0` at `1 kg`, `+1.0` at `10,000 kg`).
    pub fn scale(kg: f64) -> f64 {
        let ln_normal = KILO_NORMAL_KG.ln();
        let l = Self::clamp_kg(kg).ln() - ln_normal;
        if l < 0.0 {
            l / ln_normal
        } else {
            l / (KILO_MAX_KG / KILO_NORMAL_KG).ln()
        }
    }

    pub fn kg_at(scale: f64) -> f64 {
        let s = scale.clamp(-1.0, 1.0);
        let ln_normal = KILO_NORMAL_KG.ln();
        let span = if s < 0.0 {
            ln_normal
        } else {
            (KILO_MAX_KG / KILO_NORMAL_KG).ln()
        };
        (ln_normal + s * span).exp()
    }

    pub fn lightness(kg: f64) -> f64 {
        (-Self::scale(kg)).max(0.0)
    }

    pub fn heaviness(kg: f64) -> f64 {
        Self::scale(kg).max(0.0)
    }

    pub fn rate(seconds: f64) -> f64 {
        KILO_MAX_KG.ln() / (seconds * 20.0).max(1.0)
    }

    pub fn rate_between(from: f64, to: f64, seconds: f64) -> f64 {
        (Self::clamp_kg(to).ln() - Self::clamp_kg(from).ln())
            .abs()
            .max(1e-6)
            / (seconds * 20.0).max(1.0)
    }

    pub fn step(kg: f64, target: f64, rate: f64) -> f64 {
        let a = Self::clamp_kg(kg).ln();
        let b = Self::clamp_kg(target).ln();
        if (b - a).abs() <= rate {
            Self::clamp_kg(target)
        } else {
            (a + rate.copysign(b - a)).exp()
        }
    }

    pub fn gravity(kg: f64) -> f64 {
        let s = Self::scale(kg);
        if s < 0.0 {
            let light = KILO_LIGHT_TERMINAL_SPEED * (1.0 - Self::DRAG) / Self::DRAG;
            Self::GRAVITY * (light / Self::GRAVITY).powf(-s)
        } else {
            Self::GRAVITY * KILO_HEAVY_GRAVITY_MULTIPLIER.powf(s)
        }
    }

    pub fn terminal(gravity: f64) -> f64 {
        gravity * Self::DRAG / (1.0 - Self::DRAG)
    }

    pub fn fall_cap(kg: f64) -> f64 {
        let h = Self::heaviness(kg);
        let x = (h * 4.0).min(1.0);
        let smooth = x * x * (3.0 - 2.0 * x);
        let vanilla_term = Self::terminal(Self::GRAVITY).max(KILO_MAX_FALL_SPEED);
        vanilla_term + (KILO_MAX_FALL_SPEED - vanilla_term) * smooth
    }

    pub fn speed_multiplier(kg: f64) -> f64 {
        let s = Self::scale(kg);
        if s < 0.0 {
            1.0 + KILO_LIGHT_SPEED_BONUS * (-s)
        } else {
            1.0 - (1.0 - KILO_HEAVY_SPEED_MULTIPLIER) * s.powf(1.2)
        }
    }

    pub fn step_height(kg: f64) -> f64 {
        let s = Self::scale(kg);
        if s < 0.0 {
            0.6 + (KILO_LIGHT_STEP_HEIGHT - 0.6) * (-s)
        } else {
            0.6 * (1.0 - s * s)
        }
    }

    pub fn knockback_resistance(kg: f64) -> f64 {
        let h = Self::heaviness(kg);
        if h <= 0.0 {
            0.0
        } else {
            h.powf(0.75)
        }
    }

    pub fn knockback_multiplier(kg: f64) -> f64 {
        (KILO_NORMAL_KG / Self::clamp_kg(kg))
            .max(1.0)
            .min(KILO_MAX_KNOCKBACK_MULTIPLIER)
    }

    pub fn safe_fall_distance(kg: f64) -> f64 {
        3.0 + 200.0 * Self::lightness(kg)
    }

    pub fn drain_per_second(kg: f64) -> f64 {
        let s = Self::scale(kg);
        let d = s.abs();
        if d <= KILO_DRAIN_DEAD_ZONE {
            return 0.0;
        }
        let x = (d - KILO_DRAIN_DEAD_ZONE) / (1.0 - KILO_DRAIN_DEAD_ZONE);
        let max_drain = if s < 0.0 {
            KILO_LIGHT_DRAIN_PER_SECOND
        } else {
            KILO_HEAVY_DRAIN_PER_SECOND
        };
        max_drain * x.powf(1.3)
    }

    /// Kinetic energy in Joules (`0.5 * kg * (blocks_per_tick * 20)^2`).
    pub fn kinetic_energy(kg: f64, blocks_per_tick: f64) -> f64 {
        let v = blocks_per_tick * 20.0;
        0.5 * kg * v * v
    }

    pub fn speed_for(kg: f64, joules: f64) -> f64 {
        if joules <= 0.0 {
            0.0
        } else {
            (2.0 * joules / Self::clamp_kg(kg)).sqrt() / 20.0
        }
    }

    pub fn impact_power(joules: f64) -> f64 {
        (1.0 + joules.max(0.0) / 50_000.0).log10()
    }

    pub fn impact_radius(joules: f64) -> f64 {
        1.5 + 2.2 * Self::impact_power(joules)
    }

    pub fn impact_damage(joules: f64) -> f64 {
        let p = Self::impact_power(joules);
        4.0 * p * p
    }

    pub fn crush_damage(joules: f64, cap: f64) -> f64 {
        let p = Self::impact_power(joules);
        (12.0 * p * p).min(cap)
    }
}

/// Kilo Kilo no Mi module (`com.devilfruits.fruits.kilo_kilo.KiloKilo`).
#[derive(Clone, Copy, Debug, Default)]
pub struct KiloKiloModule;

impl KiloKiloModule {
    pub const fn new() -> Self {
        Self
    }

    /// Resolves effective body mass in kilograms from active Kilo-Kilo form flags.
    pub fn active_mass_kg(form_runtime: &FormRuntime) -> f64 {
        if form_runtime.has_flag(KILO_FLAG_HEAVY) || form_runtime.has_flag(KILO_FLAG_PLUNGING) {
            KILO_MAX_KG
        } else if form_runtime.has_flag(KILO_FLAG_LIGHT) {
            KILO_MIN_KG
        } else {
            KILO_NORMAL_KG
        }
    }
}

impl DevilFruitModule for KiloKiloModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::KiloKilo
    }

    fn primary_damage_type(&self) -> GrandLineDamageType {
        GrandLineDamageType::KiloPress
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_form = vec!["base".to_string()];

        let make_light = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::KiloKilo, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: ONE_KILO_ENERGY,
                energy_per_second: KILO_LIGHT_DRAIN_PER_SECOND as f32,
                cooldown_ticks: ONE_KILO_COOLDOWN_TICKS,
                unlock_mastery: ONE_KILO_UNLOCK_MASTERY,
                damage: 0.0,
                range: 1.0,
                knockback: 0.0,
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_press = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::KiloKilo, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: PRESS_ENERGY,
                energy_per_second: KILO_HEAVY_DRAIN_PER_SECOND as f32,
                cooldown_ticks: PRESS_COOLDOWN_TICKS,
                unlock_mastery: PRESS_UNLOCK_MASTERY,
                damage: 28.0,
                range: KILO_PRESS_BURST_RADIUS as f32,
                knockback: 1.4,
                ..AbilityStats::default()
            },
            is_ultimate: true,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        vec![
            make_light("one_kilo", "1 Kilo"),
            make_light("light_feather", "Light Feather (1 Kilo)"),
            make_press("press", "10,000 Kilo Press"),
            make_press("ten_thousand_kilo_press", "10,000 Kilo Press"),
            make_press("kilo_press_10000", "10,000 Kilo Press"),
            AbilityDef {
                id: AbilityId::new(FruitId::KiloKilo, "kilo_slingshot"),
                display_name: "Kilo Slingshot".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: KILO_SLINGSHOT_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: KILO_SLINGSHOT_COOLDOWN_TICKS,
                    unlock_mastery: KILO_SLINGSHOT_UNLOCK_MASTERY,
                    damage: KILO_SLINGSHOT_DAMAGE,
                    range: KILO_SLINGSHOT_RANGE,
                    knockback: KILO_SLINGSHOT_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::KiloKilo, "heavy_stomp"),
                display_name: "10,000 Kilo Heavy Stomp".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: HEAVY_STOMP_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: HEAVY_STOMP_COOLDOWN_TICKS,
                    unlock_mastery: HEAVY_STOMP_UNLOCK_MASTERY,
                    damage: HEAVY_STOMP_DAMAGE,
                    range: HEAVY_STOMP_RANGE,
                    knockback: HEAVY_STOMP_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
        ]
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::KiloPress),
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
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "one_kilo" || ability == "light_feather" =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(KILO_FLAG_LIGHT),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(KILO_FLAG_LIGHT, active);
                if active {
                    ctx.form_runtime.set_flag(KILO_FLAG_HEAVY, false);
                    ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, false);
                    res.animation_clip = Some("devilfruits:kilo_kilo/light_cast".into());
                    ctx.world_fx.emit_fx(
                        kilo_fx::RAMP_START,
                        0,
                        eye,
                        DVec3::ZERO,
                        -1.0,
                        now,
                        0,
                        64.0,
                    );
                } else {
                    ctx.world_fx.emit_fx(
                        kilo_fx::RAMP_DONE,
                        0,
                        eye,
                        DVec3::ZERO,
                        0.0,
                        now,
                        0,
                        64.0,
                    );
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if matches!(
                    ability.as_str(),
                    "press" | "ten_thousand_kilo_press" | "kilo_press_10000"
                ) =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(KILO_FLAG_HEAVY),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(KILO_FLAG_HEAVY, active);
                if active {
                    ctx.form_runtime.set_flag(KILO_FLAG_LIGHT, false);
                    let airborne = !ctx.on_ground && !ctx.submerged_in_water;
                    if airborne && ctx.player.cooldown_remaining("press_combo", now) == 0 {
                        let _ = ctx.player.spend_energy(
                            PRESS_PLUNGE_EXTRA_ENERGY,
                            ctx.creative,
                            ctx.config,
                        );
                        ctx.player.set_cooldown(
                            "press_combo",
                            now,
                            PRESS_PLUNGE_COOLDOWN_TICKS,
                        );
                        ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, true);
                        res.caster_velocity_impulse =
                            Some(DVec3::new(0.0, -KILO_MAX_FALL_SPEED, 0.0));
                        res.animation_clip = Some("devilfruits:kilo_kilo/press_dive".into());
                        ctx.world_fx.emit_fx(
                            kilo_fx::PRESS_START,
                            0,
                            eye,
                            DVec3::ZERO,
                            1.0,
                            now,
                            0,
                            64.0,
                        );
                    } else {
                        res.animation_clip = Some("devilfruits:kilo_kilo/heavy_cast".into());
                    }
                    ctx.world_fx.emit_fx(
                        kilo_fx::RAMP_START,
                        0,
                        eye,
                        DVec3::ZERO,
                        1.0,
                        now,
                        if airborne { 1 } else { 0 },
                        64.0,
                    );
                } else {
                    ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, false);
                    ctx.world_fx.emit_fx(
                        kilo_fx::RAMP_DONE,
                        0,
                        eye,
                        DVec3::ZERO,
                        0.0,
                        now,
                        0,
                        64.0,
                    );
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "kilo_slingshot" => {
                let Some(def) = self.ability("kilo_slingshot") else {
                    return res;
                };
                // Slingshot: high-speed forward + upward launch at 1 kg followed by 10,000 kg drop strike
                let range = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                res.caster_velocity_impulse = Some(flat * 1.9 + DVec3::new(0.0, 1.35, 0.0));
                res.animation_clip = Some("devilfruits:kilo_kilo/press_dive".into());
                ctx.world_fx.emit_fx(
                    kilo_fx::LIGHT_JUMP,
                    0,
                    eye,
                    DVec3::ZERO,
                    KILO_LIGHT_JUMP_BLOCKS as f32,
                    now,
                    0,
                    64.0,
                );

                let end = eye + look * range;
                for target in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    end,
                    2.2,
                ) {
                    let away = (target.center - eye).normalize_or_zero();
                    let push = away * kb + DVec3::new(0.0, 0.45, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::KiloPress,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            target.knockback_resistance,
                        ),
                        stun_ticks: 10,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "heavy_stomp" => {
                let Some(def) = self.ability("heavy_stomp") else {
                    return res;
                };
                ctx.form_runtime.set_flag(KILO_FLAG_HEAVY, true);
                ctx.form_runtime.set_flag(KILO_FLAG_LIGHT, false);
                let energy_joules = WeightCurves::kinetic_energy(KILO_MAX_KG, KILO_MAX_FALL_SPEED);
                let power = WeightCurves::impact_power(energy_joules);
                let radius = WeightCurves::impact_radius(energy_joules)
                    .max(def.stats.scaled_range(mastery, ctx.config, &mods) as f64);
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;

                res.animation_clip = Some("devilfruits:kilo_kilo/press_slam".into());
                ctx.world_fx.emit_fx(
                    kilo_fx::IMPACT,
                    0,
                    eye,
                    DVec3::Y,
                    power as f32,
                    now,
                    (radius * 16.0).round() as i32,
                    96.0,
                );
                ctx.world_fx.emit_shake(eye, 1.3, (radius as i32) * 3);
                res.screen_shake = Some((1.3, (radius as i32) * 3));

                for target in
                    CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, radius)
                {
                    let dist = target.center.distance(eye);
                    let falloff = (1.0 - dist / radius.max(0.01)).clamp(0.0, 1.0);
                    let away = DVec3::new(
                        target.center.x - eye.x,
                        0.0,
                        target.center.z - eye.z,
                    )
                    .normalize_or_zero();
                    let away = if away.length_squared() < 1e-4 {
                        flat
                    } else {
                        away
                    };
                    let push = away * (kb * falloff) + DVec3::new(0.0, 0.25 + 0.3 * falloff, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: base_dmg * (0.3 + 0.7 * (falloff as f32)),
                        damage_type: GrandLineDamageType::KiloPress,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            target.knockback_resistance,
                        ),
                        stun_ticks: 14,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.budget.blocks_remaining =
                    ctx.budget.blocks_remaining.min(PRESS_MAX_BLOCKS_PER_TICK);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    DVec3::new(eye.x, eye.y - 1.62, eye.z),
                    2.6,
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
            ctx.form_runtime.set_flag(KILO_FLAG_LIGHT, false);
            ctx.form_runtime.set_flag(KILO_FLAG_HEAVY, false);
            ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, false);
            return res;
        }

        let kg = Self::active_mass_kg(ctx.form_runtime);
        let drain_per_sec = WeightCurves::drain_per_second(kg) as f32;
        if drain_per_sec > 0.0 && !ctx.player.spend_energy(drain_per_sec / 20.0, ctx.creative, ctx.config) {
            ctx.form_runtime.set_flag(KILO_FLAG_LIGHT, false);
            ctx.form_runtime.set_flag(KILO_FLAG_HEAVY, false);
            ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, false);
            return res;
        }

        // Terminal landing shockwave (`HeavyBody.onLand` / `HeavyBody.impact`) when a plunging or airborne 10,000 kg user touches down
        if ctx.form_runtime.has_flag(KILO_FLAG_PLUNGING) {
            if !ctx.on_ground && !ctx.submerged_in_water {
                res.caster_velocity_impulse = Some(DVec3::new(0.0, -KILO_MAX_FALL_SPEED, 0.0));
            } else if ctx.on_ground {
                ctx.form_runtime.set_flag(KILO_FLAG_PLUNGING, false);
                let energy_joules = WeightCurves::kinetic_energy(KILO_MAX_KG, KILO_MAX_FALL_SPEED);
                let power = WeightCurves::impact_power(energy_joules);
                let radius = WeightCurves::impact_radius(energy_joules).max(KILO_PRESS_BURST_RADIUS);
                let mastery_mult = Mastery::damage_multiplier(ctx.player.mastery, ctx.config);
                let centre_dmg = (WeightCurves::impact_damage(energy_joules) as f32) * mastery_mult;
                let crush_dmg = (WeightCurves::crush_damage(energy_joules, KILO_CRUSH_DAMAGE_CAP)
                    as f32)
                    * mastery_mult;
                let at = ctx.ability_ctx.eye_pos;
                let now = ctx.ability_ctx.tick;

                res.damage_type = Some(GrandLineDamageType::KiloPress);
                res.animation_clip = Some("devilfruits:kilo_kilo/press_slam".into());
                ctx.world_fx.emit_fx(
                    kilo_fx::IMPACT,
                    0,
                    at,
                    DVec3::Y,
                    power as f32,
                    now,
                    (radius * 16.0).round() as i32,
                    96.0,
                );
                ctx.world_fx.emit_shake(at, 1.4, (radius as i32) * 3);
                res.screen_shake = Some((1.4, (radius as i32) * 3));

                for target in
                    CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, at, radius)
                {
                    let dist = target.center.distance(at);
                    let falloff = (1.0 - dist / radius.max(0.01)).clamp(0.0, 1.0);
                    let crushed = dist < 1.1;
                    let dmg = if crushed {
                        crush_dmg
                    } else {
                        centre_dmg * (0.3 + 0.7 * (falloff as f32))
                    };
                    let away = DVec3::new(
                        target.center.x - at.x,
                        0.0,
                        target.center.z - at.z,
                    )
                    .normalize_or_zero();
                    let away = if away.length_squared() < 1e-4 {
                        DVec3::X
                    } else {
                        away
                    };
                    let kb = (0.2 + 0.4 * power) * falloff;
                    let launch = if crushed {
                        DVec3::ZERO
                    } else {
                        away * kb + DVec3::new(0.0, 0.18 + 0.2 * power * falloff, 0.0)
                    };
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::KiloPress,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch,
                            target.knockback_resistance,
                        ),
                        stun_ticks: if crushed { 20 } else { 8 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.budget.blocks_remaining =
                    ctx.budget.blocks_remaining.min(PRESS_MAX_BLOCKS_PER_TICK);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    DVec3::new(at.x, at.y - 1.62, at.z),
                    2.4,
                    &mut ctx.budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_pos| ("minecraft:stone".to_string(), 1.5, false),
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
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
        let kg = Self::active_mass_kg(form_runtime);
        mods.mass_kg_override = Some(kg);
        mods.mass_tier = mass_tier_from_kg(kg, KILO_HEAVY_IGNORES_PUSHING);
        mods.immovable = is_mass_immovable(kg, KILO_HEAVY_IGNORES_PUSHING);
        if WeightCurves::lightness(kg) > 0.1 {
            mods.reset_above_ground_ticks = true;
        } else if WeightCurves::heaviness(kg) > 0.3 {
            // Heavy body sinks faster in water
            mods.sink_speed_override = Some(0.24);
        }
        let _ = EntityMassTier::Normal;
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
    fn kilo_kilo_weight_curves_and_abilities_match_java() {
        let module = KiloKiloModule::new();
        assert_eq!(module.fruit_id(), FruitId::KiloKilo);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::KiloPress);

        assert!(module.ability("kilo_press_10000").is_some());
        assert!(module.ability("light_feather").is_some());
        assert!(module.ability("kilo_slingshot").is_some());
        assert!(module.ability("heavy_stomp").is_some());
        assert!(module.ability("one_kilo").is_some());
        assert!(module.ability("press").is_some());

        // WeightCurves exact checks
        assert!((WeightCurves::scale(70.0) - 0.0).abs() < 1e-9);
        assert!((WeightCurves::scale(1.0) - (-1.0)).abs() < 1e-9);
        assert!((WeightCurves::scale(10_000.0) - 1.0).abs() < 1e-9);
        assert!((WeightCurves::gravity(70.0) - 0.08).abs() < 1e-9);
        assert!((WeightCurves::gravity(10_000.0) - 0.48).abs() < 1e-9);
        assert!((WeightCurves::speed_multiplier(1.0) - 1.2).abs() < 1e-9);
        assert!((WeightCurves::speed_multiplier(10_000.0) - 0.45).abs() < 1e-9);
        assert!((WeightCurves::knockback_resistance(10_000.0) - 1.0).abs() < 1e-9);
        assert!((WeightCurves::drain_per_second(70.0) - 0.0).abs() < 1e-9);
        assert!((WeightCurves::drain_per_second(1.0) - 3.0).abs() < 1e-9);
        assert!((WeightCurves::drain_per_second(10_000.0) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn kilo_kilo_dynamic_weight_body_modifiers_and_terminal_landing_shockwave() {
        let module = KiloKiloModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::KiloKilo),
            mastery: 5,
            energy: 200.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = [make_target(2, DVec3::new(0.0, 64.0, 0.8))];

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
                fruit: FruitId::KiloKilo,
                ability_name: "light_feather".into(),
                mastery: 5,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 120,
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
            on_ground: false,
            in_lava: false,
        };

        // 1. Toggle Light Feather (1 kg)
        let _ = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "light_feather".into(),
                enabled: true,
            },
        );
        let light_mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert_eq!(light_mods.mass_kg_override, Some(1.0));
        assert_eq!(light_mods.mass_tier, EntityMassTier::Light);
        assert!(!light_mods.is_immovable());
        assert!(light_mods.reset_above_ground_ticks);

        // 2. Toggle 10,000 Kilo Press while airborne -> plunges downward at terminal speed
        let press_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "kilo_press_10000".into(),
                enabled: true,
            },
        );
        assert!(ctx.form_runtime.has_flag(KILO_FLAG_HEAVY));
        assert!(ctx.form_runtime.has_flag(KILO_FLAG_PLUNGING));
        assert_eq!(
            press_res.caster_velocity_impulse,
            Some(DVec3::new(0.0, -3.0, 0.0))
        );

        let heavy_mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert_eq!(heavy_mods.mass_kg_override, Some(10_000.0));
        assert_eq!(heavy_mods.mass_tier, EntityMassTier::Immovable);
        assert!(heavy_mods.is_immovable());

        // 3. Touch ground -> on_tick triggers terminal landing shockwave
        ctx.on_ground = true;
        let land_res = module.on_tick(&mut ctx);
        assert_eq!(land_res.hits.len(), 1);
        assert!(land_res.hits[0].damage > 30.0);
        assert!(!land_res.destroyed_blocks.is_empty());
        assert!(land_res.screen_shake.is_some());
        assert!(!ctx.form_runtime.has_flag(KILO_FLAG_PLUNGING));
    }
}
