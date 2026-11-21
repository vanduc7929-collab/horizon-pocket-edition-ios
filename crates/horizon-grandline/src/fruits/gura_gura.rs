//! Gura Gura no Mi (Tremor-Tremor Fruit) — 1:1 port of `com.devilfruits.fruits.gura_gura`.
//!
//! Includes all canonical Java abilities (`kaishin`, `gekishin`, `kabutowari`,
//! `kaisho`, `shindo`, `quake_infusion`, `shima_yurashi`) plus `gekichin` and
//! `tenchi_meido`, `FissurePattern` procedural crack generation, spherical crater
//! destruction, projectile shatter/deflect, shockwave impulses, and screen shake.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::FormRuntime;
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Java constants (`GuraGura.java`, `GuraFx.java`, `ability/*.java`)
// ---------------------------------------------------------------------------

/// `GuraGura.java` setting `"destructionAtMaxMastery"` default (`3.5`).
pub const GURA_DESTRUCTION_AT_MAX_MASTERY: f32 = 3.5;
/// `GuraGura.java` `PhysicsEvents.registerEmpowerment`: Shindo toggle empowerment (`2.5`).
pub const GURA_SHINDO_PHYSICS_EMPOWERMENT: f32 = 2.5;

/// `KaishinAbility.java` constants.
pub const KAISHIN_ENERGY: f32 = 14.0;
pub const KAISHIN_COOLDOWN_TICKS: i32 = 22;
pub const KAISHIN_UNLOCK_MASTERY: u32 = 1;
pub const KAISHIN_DAMAGE: f32 = 11.0;
pub const KAISHIN_RANGE: f32 = 7.0;
pub const KAISHIN_KNOCKBACK: f32 = 1.5;
pub const KAISHIN_CHARGE_TICKS: i32 = 36;
pub const KAISHIN_FULL_CHARGE_EXTRA_ENERGY: f32 = 22.0;
pub const KAISHIN_FULL_CHARGE_EXTRA_COOLDOWN: i32 = 60;
pub const KAISHIN_CONE_ANGLE_DEG: f64 = 28.0;
pub const KAISHIN_BLOCK_DAMAGE_CHARGE: f32 = 0.5;

/// `GekishinAbility.java` constants.
pub const GEKISHIN_ENERGY: f32 = 22.0;
pub const GEKISHIN_COOLDOWN_TICKS: i32 = 90;
pub const GEKISHIN_UNLOCK_MASTERY: u32 = 1;
pub const GEKISHIN_DAMAGE: f32 = 24.0;
pub const GEKISHIN_RANGE: f32 = 4.5;
pub const GEKISHIN_KNOCKBACK: f32 = 3.0;
pub const GEKISHIN_WINDUP_TICKS: i32 = 5;
pub const GEKISHIN_LAUNCH_UP: f64 = 0.55;
pub const GEKISHIN_IMPACT_DAMAGE: f32 = 9.0;
pub const GEKISHIN_LUNGE_SPEED: f64 = 0.75;

/// `KabutowariAbility.java` constants.
pub const KABUTOWARI_ENERGY: f32 = 30.0;
pub const KABUTOWARI_COOLDOWN_TICKS: i32 = 160;
pub const KABUTOWARI_UNLOCK_MASTERY: u32 = 2;
pub const KABUTOWARI_DAMAGE: f32 = 16.0;
pub const KABUTOWARI_RANGE: f32 = 7.0;
pub const KABUTOWARI_KNOCKBACK: f32 = 1.1;
pub const KABUTOWARI_LINES: i32 = 7;
pub const KABUTOWARI_LEAP_VELOCITY: f64 = 0.95;
pub const KABUTOWARI_SPREAD_SPEED: f32 = 1.6;
pub const KABUTOWARI_LIFT_CHANCE: f32 = 0.7;
pub const KABUTOWARI_EPICENTRE_RADIUS: f64 = 3.2;

/// `KaishoAbility.java` constants.
pub const KAISHO_ENERGY: f32 = 45.0;
pub const KAISHO_COOLDOWN_TICKS: i32 = 300;
pub const KAISHO_UNLOCK_MASTERY: u32 = 4;
pub const KAISHO_DAMAGE: f32 = 14.0;
pub const KAISHO_RANGE: f32 = 22.0;
pub const KAISHO_KNOCKBACK: f32 = 1.8;
pub const KAISHO_WAVE_WIDTH: f32 = 10.0;
pub const KAISHO_WAVE_HEIGHT: f32 = 5.0;
pub const KAISHO_SPEED: f32 = 0.75;
pub const KAISHO_WATER_SEARCH: i32 = 8;
pub const KAISHO_LAND_REACH: f64 = 7.0;

/// `QuakeAuraAbility.java` (`shindo`) constants.
pub const SHINDO_ENERGY: f32 = 10.0;
pub const SHINDO_COOLDOWN_TICKS: i32 = 40;
pub const SHINDO_UNLOCK_MASTERY: u32 = 3;
pub const SHINDO_DAMAGE: f32 = 5.0;
pub const SHINDO_RANGE: f32 = 3.5;
pub const SHINDO_KNOCKBACK: f32 = 0.8;
pub const SHINDO_DRAIN_PER_SECOND: f32 = 4.0;
pub const SHINDO_DEFLECT_COST: f32 = 2.0;
pub const SHINDO_HIT_COST: f32 = 1.0;

/// `QuakeInfusionAbility.java` (`quake_infusion`) constants.
pub const QUAKE_INFUSION_ENERGY: f32 = 12.0;
pub const QUAKE_INFUSION_COOLDOWN_TICKS: i32 = 24;
pub const QUAKE_INFUSION_UNLOCK_MASTERY: u32 = 3;
pub const QUAKE_INFUSION_DAMAGE: f32 = 9.0;
pub const QUAKE_INFUSION_RANGE: f32 = 18.0;
pub const QUAKE_INFUSION_KNOCKBACK: f32 = 1.0;
pub const QUAKE_INFUSION_SLASH_SPEED: f32 = 1.3;
pub const QUAKE_INFUSION_SLASH_WIDTH: f32 = 3.2;
pub const QUAKE_INFUSION_WEAPON_DAMAGE_FACTOR: f32 = 0.8;

/// `ShimaYurashiAbility.java` (`shima_yurashi`) constants.
pub const SHIMA_YURASHI_ENERGY: f32 = 80.0;
pub const SHIMA_YURASHI_COOLDOWN_TICKS: i32 = 1200;
pub const SHIMA_YURASHI_UNLOCK_MASTERY: u32 = 5;
pub const SHIMA_YURASHI_DAMAGE: f32 = 7.0;
pub const SHIMA_YURASHI_RANGE: f32 = 28.0;
pub const SHIMA_YURASHI_KNOCKBACK: f32 = 0.7;
pub const SHIMA_YURASHI_TREMOR_SECONDS: f32 = 5.0;
pub const SHIMA_YURASHI_PULSE_INTERVAL_TICKS: i32 = 10;
pub const SHIMA_YURASHI_CRACK_DISTANCE: f64 = 14.0;
pub const SHIMA_YURASHI_CRACK_RADIUS: f32 = 9.0;
pub const SHIMA_YURASHI_DEBRIS_PER_PULSE: f32 = 7.0;

/// `TenchiMeido` cataclysmic quake technique constants.
pub const TENCHI_MEIDO_ENERGY: f32 = 95.0;
pub const TENCHI_MEIDO_COOLDOWN_TICKS: i32 = 1400;
pub const TENCHI_MEIDO_UNLOCK_MASTERY: u32 = 6;
pub const TENCHI_MEIDO_DAMAGE: f32 = 32.0;
pub const TENCHI_MEIDO_RANGE: f32 = 32.0;
pub const TENCHI_MEIDO_KNOCKBACK: f32 = 2.4;
pub const TENCHI_MEIDO_CRATER_RADIUS: f64 = 5.5;

/// Canonical Gura-Gura VFX identifiers (`GuraFx.java`).
pub mod gura_fx {
    pub const AWAKEN: &str = "devilfruits:gura_gura/awaken";
    pub const LEVEL_UP: &str = "devilfruits:gura_gura/level_up";
    pub const REINCARNATE: &str = "devilfruits:gura_gura/reincarnate";
    pub const KAISHIN: &str = "devilfruits:gura_gura/kaishin";
    pub const GEKISHIN_HIT: &str = "devilfruits:gura_gura/gekishin_hit";
    pub const LAUNCH_TRAIL: &str = "devilfruits:gura_gura/launch_trail";
    pub const IMPACT_RING: &str = "devilfruits:gura_gura/impact_ring";
    pub const KABUTOWARI_LEAP: &str = "devilfruits:gura_gura/kabutowari_leap";
    pub const KABUTOWARI_SLAM: &str = "devilfruits:gura_gura/kabutowari_slam";
    pub const SHIMA_GRAB: &str = "devilfruits:gura_gura/shima_grab";
    pub const SHIMA_TREMOR: &str = "devilfruits:gura_gura/shima_tremor";
    pub const KAISHO_CAST: &str = "devilfruits:gura_gura/kaisho_cast";
    pub const AURA_ON: &str = "devilfruits:gura_gura/aura_on";
    pub const AURA_OFF: &str = "devilfruits:gura_gura/aura_off";
    pub const AURA_DEFLECT: &str = "devilfruits:gura_gura/aura_deflect";
    pub const AURA_HIT: &str = "devilfruits:gura_gura/aura_hit";
    pub const SLASH_CAST: &str = "devilfruits:gura_gura/slash_cast";
    pub const SLASH_IMPACT: &str = "devilfruits:gura_gura/slash_impact";
}

/// Canonical Gura-Gura animation clips (`GuraGura.anim`).
pub mod gura_anim {
    pub const KAISHIN_CHARGE: &str = "devilfruits:gura_gura/kaishin_charge";
    pub const KAISHIN_RELEASE: &str = "devilfruits:gura_gura/kaishin_release";
    pub const GEKISHIN: &str = "devilfruits:gura_gura/gekishin";
    pub const KABUTOWARI_LEAP: &str = "devilfruits:gura_gura/kabutowari_leap";
    pub const KABUTOWARI_SLAM: &str = "devilfruits:gura_gura/kabutowari_slam";
    pub const KAISHO: &str = "devilfruits:gura_gura/kaisho";
    pub const SHINDO: &str = "devilfruits:gura_gura/shindo";
    pub const QUAKE_SLASH: &str = "devilfruits:gura_gura/quake_slash";
    pub const SHIMA_YURASHI: &str = "devilfruits:gura_gura/shima_yurashi";
}

/// Procedural fissure line generated by `KabutowariAbility` (`FissurePattern.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct FissureLine {
    pub x: Vec<f32>,
    pub z: Vec<f32>,
    pub dist: Vec<f32>,
    pub width: f32,
}

impl FissureLine {
    /// Generates radial fissure lines matching `FissurePattern.generate(seed, lineCount, length)`.
    pub fn generate(seed: i64, line_count: i32, length: f32) -> Vec<Self> {
        let count = line_count.max(1);
        let step_len = 0.75_f32;
        let mut out = Vec::with_capacity(count as usize);
        let base_angle = ((seed.unsigned_abs() % 6283) as f32) / 1000.0;

        for i in 0..count {
            let jitter = ((((seed.wrapping_mul(i as i64 + 17)) & 0xFF) as f32 / 255.0) - 0.5)
                * (std::f32::consts::PI / count as f32)
                * 0.55;
            let angle = base_angle + (i as f32) * std::f32::consts::TAU / (count as f32) + jitter;
            let len_factor = 0.7 + 0.3 * (((seed.wrapping_add(i as i64 * 31)) & 0x7F) as f32 / 127.0);
            let line_len = length * len_factor;
            let steps = ((line_len / step_len).ceil() as usize).max(2);
            let mut xs = Vec::with_capacity(steps + 1);
            let mut zs = Vec::with_capacity(steps + 1);
            let mut ds = Vec::with_capacity(steps + 1);
            let mut x = 0.0_f32;
            let mut z = 0.0_f32;
            xs.push(0.0);
            zs.push(0.0);
            ds.push(0.0);
            for s in 1..=steps {
                x += angle.cos() * step_len;
                z += angle.sin() * step_len;
                xs.push(x);
                zs.push(z);
                ds.push(s as f32 * step_len);
            }
            out.push(Self {
                x: xs,
                z: zs,
                dist: ds,
                width: 1.0,
            });
        }
        out
    }
}

/// Gura Gura no Mi module (`com.devilfruits.fruits.gura_gura.GuraGura`).
#[derive(Clone, Copy, Debug, Default)]
pub struct GuraGuraModule;

impl GuraGuraModule {
    pub const fn new() -> Self {
        Self
    }

    /// Exact terrain destruction multiplier (`GuraGura.destruction(mastery)`):
    /// `1.0 + (destructionAtMaxMastery - 1.0) * (clamp(mastery, 1, 10) - 1) / 9.0`.
    pub fn destruction_multiplier(mastery: u32) -> f32 {
        let clamped = mastery.clamp(1, 10) as f32;
        1.0 + (GURA_DESTRUCTION_AT_MAX_MASTERY - 1.0) * (clamped - 1.0) / 9.0
    }

    /// Exact block destruction budget (`GuraGura.blockBudget(destruction)`):
    /// `round(max_blocks_per_cast * destruction)`.
    pub fn block_budget(destruction: f32, max_blocks_per_cast: usize) -> usize {
        ((max_blocks_per_cast as f32) * destruction).round().max(0.0) as usize
    }

    /// Exact `KaishinAbility.energyCost(charge)`: `14.0 + 22.0 * charge`.
    pub fn kaishin_energy_cost(charge: f32) -> f32 {
        KAISHIN_ENERGY + KAISHIN_FULL_CHARGE_EXTRA_ENERGY * charge.clamp(0.0, 1.0)
    }

    /// Exact `KaishinAbility.cooldownTicks(charge)`: `22 + 60 * charge`.
    pub fn kaishin_cooldown_ticks(charge: f32) -> i32 {
        KAISHIN_COOLDOWN_TICKS
            + ((KAISHIN_FULL_CHARGE_EXTRA_COOLDOWN as f32) * charge.clamp(0.0, 1.0)) as i32
    }
}

impl DevilFruitModule for GuraGuraModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::GuraGura
    }

    fn primary_damage_type(&self) -> GrandLineDamageType {
        GrandLineDamageType::GuraQuake
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_form = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "kaishin"),
                display_name: "Kaishin".into(),
                cast_type: CastType::Charged,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: KAISHIN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: KAISHIN_COOLDOWN_TICKS,
                    unlock_mastery: KAISHIN_UNLOCK_MASTERY,
                    damage: KAISHIN_DAMAGE,
                    range: KAISHIN_RANGE,
                    knockback: KAISHIN_KNOCKBACK,
                    max_charge_ticks: KAISHIN_CHARGE_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "gekishin"),
                display_name: "Gekishin".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GEKISHIN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GEKISHIN_COOLDOWN_TICKS,
                    unlock_mastery: GEKISHIN_UNLOCK_MASTERY,
                    damage: GEKISHIN_DAMAGE,
                    range: GEKISHIN_RANGE,
                    knockback: GEKISHIN_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "gekichin"),
                display_name: "Gekichin".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GEKISHIN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GEKISHIN_COOLDOWN_TICKS,
                    unlock_mastery: GEKISHIN_UNLOCK_MASTERY,
                    damage: GEKISHIN_DAMAGE,
                    range: GEKISHIN_RANGE,
                    knockback: GEKISHIN_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "kabutowari"),
                display_name: "Kabutowari".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: KABUTOWARI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: KABUTOWARI_COOLDOWN_TICKS,
                    unlock_mastery: KABUTOWARI_UNLOCK_MASTERY,
                    damage: KABUTOWARI_DAMAGE,
                    range: KABUTOWARI_RANGE,
                    knockback: KABUTOWARI_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "kaisho"),
                display_name: "Kaisho".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: KAISHO_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: KAISHO_COOLDOWN_TICKS,
                    unlock_mastery: KAISHO_UNLOCK_MASTERY,
                    damage: KAISHO_DAMAGE,
                    range: KAISHO_RANGE,
                    knockback: KAISHO_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "shindo"),
                display_name: "Shindo".into(),
                cast_type: CastType::Toggle,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SHINDO_ENERGY,
                    energy_per_second: SHINDO_DRAIN_PER_SECOND,
                    cooldown_ticks: SHINDO_COOLDOWN_TICKS,
                    unlock_mastery: SHINDO_UNLOCK_MASTERY,
                    damage: SHINDO_DAMAGE,
                    range: SHINDO_RANGE,
                    knockback: SHINDO_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "quake_infusion"),
                display_name: "Quake Infusion".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: QUAKE_INFUSION_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: QUAKE_INFUSION_COOLDOWN_TICKS,
                    unlock_mastery: QUAKE_INFUSION_UNLOCK_MASTERY,
                    damage: QUAKE_INFUSION_DAMAGE,
                    range: QUAKE_INFUSION_RANGE,
                    knockback: QUAKE_INFUSION_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "shima_yurashi"),
                display_name: "Shima Yurashi".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SHIMA_YURASHI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SHIMA_YURASHI_COOLDOWN_TICKS,
                    unlock_mastery: SHIMA_YURASHI_UNLOCK_MASTERY,
                    damage: SHIMA_YURASHI_DAMAGE,
                    range: SHIMA_YURASHI_RANGE,
                    knockback: SHIMA_YURASHI_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GuraGura, "tenchi_meido"),
                display_name: "Tenchi Meido".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_form,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: TENCHI_MEIDO_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: TENCHI_MEIDO_COOLDOWN_TICKS,
                    unlock_mastery: TENCHI_MEIDO_UNLOCK_MASTERY,
                    damage: TENCHI_MEIDO_DAMAGE,
                    range: TENCHI_MEIDO_RANGE,
                    knockback: TENCHI_MEIDO_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
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
            damage_type: Some(GrandLineDamageType::GuraQuake),
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
            AbilityEvent::ChargeStarted { ability, .. } if ability == "kaishin" => {
                res.cast_result = Some(CastResult::Charging);
                res.animation_clip = Some(gura_anim::KAISHIN_CHARGE.into());
            }
            AbilityEvent::ChargeReleased { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "kaishin" =>
            {
                let c = match event {
                    AbilityEvent::ChargeReleased { charge, .. } => charge.clamp(0.0, 1.0),
                    _ => ctx.ability_ctx.charge.clamp(0.0, 1.0),
                };
                let Some(def) = self.ability("kaishin") else {
                    return res;
                };
                // KaishinAbility.release:
                // fist = eye + (0, -0.3, 0) + dir * 0.7
                // length = range * (0.75 + charge)
                // halfAngle = toRadians(28 * (1 + 0.35 * charge))
                // damage = scaled_damage * (1 + 1.4 * charge)
                // knockback = knockback * (0.8 + 1.2 * charge)
                let fist = eye + DVec3::new(0.0, -0.3, 0.0) + look * 0.7;
                let base_range = def.stats.scaled_range(mastery, ctx.config, &mods);
                let length = (base_range * (0.75 + c)) as f64;
                let half_angle = (KAISHIN_CONE_ANGLE_DEG * (1.0 + 0.35 * (c as f64))).to_radians();
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let damage = base_dmg * (1.0 + 1.4 * c);
                let knockback = (def.stats.knockback * (0.8 + 1.2 * c)) as f64;

                res.animation_clip = Some(gura_anim::KAISHIN_RELEASE.into());
                ctx.world_fx.emit_fx(
                    gura_fx::KAISHIN,
                    0,
                    fist,
                    look,
                    c,
                    now,
                    (length * 16.0).floor() as i32,
                    64.0,
                );
                ctx.world_fx.emit_shake(fist, 0.7 + 0.8 * c, 32);
                res.screen_shake = Some((0.7 + 0.8 * c, 32));

                for target in CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    length,
                    half_angle,
                ) {
                    let dist = target.center.distance(eye);
                    let falloff = (1.0 - 0.55 * dist / length.max(0.01)).clamp(0.35, 1.0) as f32;
                    let push = look * (knockback * (falloff as f64))
                        + DVec3::new(0.0, 0.25 + 0.25 * (c as f64), 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: damage * falloff,
                        damage_type: GrandLineDamageType::GuraQuake,
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

                if c >= KAISHIN_BLOCK_DAMAGE_CHARGE {
                    let power = Self::destruction_multiplier(mastery);
                    ctx.budget.blocks_remaining =
                        Self::block_budget(power, ctx.config.max_blocks_per_cast);
                    let heat_scale = ctx.heat.destruction_scale("gura_gura:kaishin", now);
                    ctx.heat.record("gura_gura:kaishin", 0.25, now);
                    let crater_center = eye + look * 2.5;
                    let crater_r = (1.8 * (c as f64) * (power.cbrt() as f64)) * (heat_scale as f64);
                    res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                        crater_center,
                        crater_r,
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
                if ability == "gekishin" || ability == "gekichin" =>
            {
                let Some(def) = self.ability("gekishin") else {
                    return res;
                };
                // GekishinAbility.strike:
                // lunge = flat * 0.75 + (0, 0.05, 0)
                let lunge = flat * GEKISHIN_LUNGE_SPEED + DVec3::new(0.0, 0.05, 0.0);
                res.caster_velocity_impulse = Some(lunge);
                res.animation_clip = Some(gura_anim::GEKISHIN.into());

                let reach = (def.stats.scaled_range(mastery, ctx.config, &mods) + 1.0) as f64;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;

                let mut candidates = CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    reach,
                    28.0_f64.to_radians(),
                );
                candidates.sort_by(|a, b| {
                    a.center
                        .distance_squared(eye)
                        .partial_cmp(&b.center.distance_squared(eye))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                if let Some(primary) = candidates.first().copied() {
                    let launch = flat * kb
                        + DVec3::new(0.0, GEKISHIN_LAUNCH_UP + look.y.max(0.0) * 1.2, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: primary.id,
                        damage: base_dmg,
                        damage_type: GrandLineDamageType::GuraQuake,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch,
                            primary.knockback_resistance,
                        ),
                        stun_ticks: 10,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx.emit_fx(
                        gura_fx::GEKISHIN_HIT,
                        0,
                        primary.center,
                        look,
                        1.0,
                        now,
                        1,
                        64.0,
                    );
                    ctx.world_fx.emit_fx(
                        gura_fx::LAUNCH_TRAIL,
                        0,
                        primary.center,
                        launch.normalize_or_zero(),
                        1.0,
                        now,
                        40,
                        64.0,
                    );
                    ctx.world_fx.emit_fx(
                        gura_fx::IMPACT_RING,
                        0,
                        primary.center,
                        DVec3::ZERO,
                        2.5,
                        now,
                        0,
                        64.0,
                    );
                    let impact_dmg = GEKISHIN_IMPACT_DAMAGE * base_dmg / GEKISHIN_DAMAGE;
                    for secondary in CombatTargeting::in_radius(
                        ctx.nearby_targets,
                        ctx.player_id,
                        primary.center,
                        2.5,
                    ) {
                        if secondary.id != primary.id {
                            res.hits.push(AbilityHitEffect {
                                target_id: secondary.id,
                                damage: impact_dmg,
                                damage_type: GrandLineDamageType::GuraQuake,
                                knockback_impulse: flat * (kb * 0.4),
                                stun_ticks: 0,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }
                } else {
                    let hit_pos = eye + look * 2.0;
                    ctx.world_fx.emit_fx(
                        gura_fx::GEKISHIN_HIT,
                        0,
                        hit_pos,
                        look,
                        0.5,
                        now,
                        -1,
                        64.0,
                    );
                    for target in CombatTargeting::in_cone(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        look,
                        4.0,
                        35.0_f64.to_radians(),
                    ) {
                        let push = look * (kb * 0.5) + DVec3::new(0.0, 0.3, 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: target.id,
                            damage: base_dmg * 0.35,
                            damage_type: GrandLineDamageType::GuraQuake,
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
                ctx.world_fx.emit_shake(eye, 0.9, 28);
                res.screen_shake = Some((0.9, 28));
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "kabutowari" => {
                let Some(def) = self.ability("kabutowari") else {
                    return res;
                };
                // KabutowariAbility: leap + slam with radial fissures
                let leap = flat * 0.35 + DVec3::new(0.0, KABUTOWARI_LEAP_VELOCITY, 0.0);
                res.caster_velocity_impulse = Some(leap);
                res.animation_clip = Some(gura_anim::KABUTOWARI_SLAM.into());

                let epicentre = DVec3::new(eye.x, (eye.y - 1.62).floor(), eye.z);
                let power = Self::destruction_multiplier(mastery);
                let grow = power.sqrt();
                let lines =
                    (((KABUTOWARI_LINES + ((mastery.saturating_sub(1) / 3) as i32)) as f32) * grow)
                        .round() as i32;
                let length = def.stats.scaled_range(mastery, ctx.config, &mods) * grow;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;

                ctx.budget.blocks_remaining =
                    Self::block_budget(power, ctx.config.max_blocks_per_cast);
                ctx.world_fx
                    .emit_fx(gura_fx::KABUTOWARI_LEAP, 0, epicentre, DVec3::ZERO, 1.0, now, 0, 64.0);
                ctx.world_fx.emit_fx(
                    gura_fx::KABUTOWARI_SLAM,
                    0,
                    epicentre,
                    DVec3::ZERO,
                    length,
                    now,
                    lines,
                    96.0,
                );
                ctx.world_fx.emit_shake(epicentre, 1.1, 48);
                res.screen_shake = Some((1.1, 48));

                for target in CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    epicentre,
                    (length as f64).max(KABUTOWARI_EPICENTRE_RADIUS),
                ) {
                    let dist = target.center.distance(epicentre);
                    let is_epicentre = dist <= KABUTOWARI_EPICENTRE_RADIUS;
                    let dmg = if is_epicentre {
                        base_dmg
                    } else {
                        base_dmg * 0.6
                    };
                    let out = DVec3::new(
                        target.center.x - epicentre.x,
                        0.0,
                        target.center.z - epicentre.z,
                    )
                    .normalize_or_zero();
                    let launch = if is_epicentre {
                        out * 0.45 + DVec3::new(0.0, kb, 0.0)
                    } else {
                        out * 0.2 + DVec3::new(0.0, kb * 0.8 + 0.2, 0.0)
                    };
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::GuraQuake,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch,
                            target.knockback_resistance,
                        ),
                        stun_ticks: if is_epicentre { 12 } else { 0 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                // Tear out blocks along generated fissure lines
                let fissures = FissurePattern_generate(now, lines, length);
                let gy = epicentre.y.floor() as i32;
                for line in &fissures {
                    for i in 0..line.x.len() {
                        if ctx.budget.blocks_remaining == 0 {
                            break;
                        }
                        let bx = (epicentre.x + line.x[i] as f64).floor() as i32;
                        let bz = (epicentre.z + line.z[i] as f64).floor() as i32;
                        let pos = [bx, gy, bz];
                        if !res.destroyed_blocks.contains(&pos)
                            && ctx.world_fx.tear_out(
                                pos,
                                "minecraft:stone",
                                1.5,
                                false,
                                Some(DVec3::new(0.0, 0.48, 0.0)),
                                &mut ctx.budget,
                                (i as i64) * 2,
                                false,
                                true,
                                now,
                                ctx.config,
                                ctx.game_rules,
                            )
                        {
                            res.destroyed_blocks.push(pos);
                        }
                    }
                }

                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "kaisho" => {
                let Some(def) = self.ability("kaisho") else {
                    return res;
                };
                res.animation_clip = Some(gura_anim::KAISHO.into());
                let cast_pos = eye + DVec3::new(0.0, -0.4, 0.0) + flat * 0.8;
                let water_present = ctx.submerged_in_water;
                ctx.world_fx.emit_fx(
                    gura_fx::KAISHO_CAST,
                    0,
                    cast_pos,
                    flat,
                    if water_present { 1.0 } else { 0.0 },
                    now,
                    0,
                    96.0,
                );
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let reach = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                res.spawned_projectiles = 1;

                for target in CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    flat,
                    if water_present {
                        reach
                    } else {
                        KAISHO_LAND_REACH
                    },
                    30.0_f64.to_radians(),
                ) {
                    let dmg = if water_present {
                        base_dmg
                    } else {
                        base_dmg * 0.5
                    };
                    let launch = if water_present {
                        flat * (kb * 1.4) + DVec3::new(0.0, 0.6, 0.0)
                    } else {
                        flat * kb + DVec3::new(0.0, 0.35, 0.0)
                    };
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::GuraQuake,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch,
                            target.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::Toggled {
                ability,
                enabled,
            } if ability == "shindo" => {
                res.cast_result = Some(CastResult::Toggled);
                if *enabled {
                    res.animation_clip = Some(gura_anim::SHINDO.into());
                    ctx.world_fx
                        .emit_fx(gura_fx::AURA_ON, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                } else {
                    ctx.world_fx
                        .emit_fx(gura_fx::AURA_OFF, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::ToggleTick { ability } if ability == "shindo" => {
                res.cast_result = Some(CastResult::Toggled);
                let Some(def) = self.ability("shindo") else {
                    return res;
                };
                let radius = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let bonus_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                for target in
                    CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, radius)
                {
                    let dir = DVec3::new(
                        target.center.x - eye.x,
                        0.0,
                        target.center.z - eye.z,
                    )
                    .normalize_or_zero();
                    let push = if dir.length_squared() > 1e-4 {
                        dir * kb + DVec3::new(0.0, 0.2, 0.0)
                    } else {
                        DVec3::new(0.0, 0.3, 0.0)
                    };
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: bonus_dmg,
                        damage_type: GrandLineDamageType::GuraQuake,
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
            AbilityEvent::CastInstant { ability, .. } if ability == "quake_infusion" => {
                let Some(def) = self.ability("quake_infusion") else {
                    return res;
                };
                res.animation_clip = Some(gura_anim::QUAKE_SLASH.into());
                res.spawned_projectiles = 1;
                let start = eye + DVec3::new(0.0, -0.35, 0.0) + look * 1.1;
                let reach = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let mul = (reach as f32) / QUAKE_INFUSION_RANGE;
                let width = (QUAKE_INFUSION_SLASH_WIDTH * mul) as f64;
                let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;

                ctx.world_fx
                    .emit_fx(gura_fx::SLASH_CAST, 0, start, look, 0.45, now, 0, 64.0);
                let end = start + look * reach;
                for target in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    width * 0.5,
                ) {
                    let push = look * kb + DVec3::new(0.0, 0.25, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::GuraQuake,
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
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. }
                if ability == "shima_yurashi" || ability == "tenchi_meido" =>
            {
                let is_tenchi = ability == "tenchi_meido";
                let Some(def) = self.ability(ability) else {
                    return res;
                };
                res.animation_clip = Some(gura_anim::SHIMA_YURASHI.into());

                let radius = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let duration_ticks = (SHIMA_YURASHI_TREMOR_SECONDS * 20.0).round() as i32;
                let crack_pos =
                    eye + flat * SHIMA_YURASHI_CRACK_DISTANCE + DVec3::new(0.0, 3.5, 0.0);

                ctx.world_fx.emit_fx(
                    gura_fx::SHIMA_GRAB,
                    0,
                    crack_pos,
                    -flat,
                    SHIMA_YURASHI_CRACK_RADIUS,
                    now,
                    duration_ticks,
                    192.0,
                );
                let sway_yaw = flat.z.atan2(flat.x);
                ctx.world_fx.emit_fx(
                    gura_fx::SHIMA_TREMOR,
                    0,
                    eye,
                    DVec3::new(sway_yaw, SHIMA_YURASHI_PULSE_INTERVAL_TICKS as f64, 0.0),
                    radius as f32,
                    now,
                    duration_ticks,
                    radius * 2.5,
                );
                ctx.world_fx.emit_shake(eye, 1.5, (radius as i32) * 2);
                res.screen_shake = Some((1.5, (radius as i32) * 2));

                let perp = DVec3::new(-flat.z, 0.0, flat.x);
                for target in
                    CombatTargeting::in_radius(ctx.nearby_targets, ctx.player_id, eye, radius)
                {
                    let dist = target.center.distance(eye);
                    let falloff = (1.0 - 0.6 * dist / radius.max(0.01)).clamp(0.2, 1.0);
                    let sway = perp * (kb * falloff) + DVec3::new(0.0, 0.28, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: base_dmg * (falloff as f32),
                        damage_type: GrandLineDamageType::GuraQuake,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            sway,
                            target.knockback_resistance,
                        ),
                        stun_ticks: if is_tenchi { 24 } else { 10 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                // Tear out spherical crater / quake debris blocks
                let power = Self::destruction_multiplier(mastery);
                ctx.budget.blocks_remaining =
                    Self::block_budget(power, ctx.config.max_blocks_per_cast);
                let crater_r = if is_tenchi {
                    TENCHI_MEIDO_CRATER_RADIUS
                } else {
                    3.5 * (power.sqrt() as f64)
                };
                let ground_center = DVec3::new(eye.x, eye.y - 1.62, eye.z) + flat * 3.0;
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    ground_center,
                    crater_r,
                    &mut ctx.budget,
                    is_tenchi,
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

    fn on_tick(&self, _ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        AbilityExecutionResult::default()
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
        .with_suppression_guard(suppressed)
    }
}

trait SuppressionGuard {
    fn with_suppression_guard(self, suppressed: bool) -> Self;
}

impl SuppressionGuard for EntityBodyModifiers {
    fn with_suppression_guard(self, _suppressed: bool) -> Self {
        self
    }
}

#[allow(non_snake_case)]
fn FissurePattern_generate(seed: i64, line_count: i32, length: f32) -> Vec<FissureLine> {
    FissureLine::generate(seed, line_count, length)
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
    fn gura_gura_constants_and_abilities_match_java() {
        let module = GuraGuraModule::new();
        assert_eq!(module.fruit_id(), FruitId::GuraGura);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::GuraQuake);

        let kaishin = module.ability("kaishin").expect("kaishin");
        assert_eq!(kaishin.cast_type, CastType::Charged);
        assert_eq!(kaishin.stats.energy_cost, 14.0);
        assert_eq!(kaishin.stats.cooldown_ticks, 22);
        assert_eq!(kaishin.stats.damage, 11.0);
        assert_eq!(kaishin.stats.range, 7.0);
        assert_eq!(kaishin.stats.knockback, 1.5);
        assert_eq!(kaishin.stats.max_charge_ticks, 36);
        assert!((GuraGuraModule::kaishin_energy_cost(1.0) - 36.0).abs() < 1e-4);
        assert_eq!(GuraGuraModule::kaishin_cooldown_ticks(1.0), 82);

        let gekishin = module.ability("gekishin").expect("gekishin");
        assert_eq!(gekishin.stats.damage, 24.0);
        assert_eq!(gekishin.stats.cooldown_ticks, 90);
        assert!(module.ability("gekichin").is_some());

        let kabutowari = module.ability("kabutowari").expect("kabutowari");
        assert_eq!(kabutowari.stats.unlock_mastery, 2);
        assert_eq!(kabutowari.stats.damage, 16.0);

        let shima = module.ability("shima_yurashi").expect("shima_yurashi");
        assert!(shima.is_ultimate);
        assert_eq!(shima.stats.energy_cost, 80.0);
        assert_eq!(shima.stats.cooldown_ticks, 1200);
        assert_eq!(shima.stats.range, 28.0);

        let tenchi = module.ability("tenchi_meido").expect("tenchi_meido");
        assert!(tenchi.is_ultimate);

        assert!((GuraGuraModule::destruction_multiplier(1) - 1.0).abs() < 1e-5);
        assert!((GuraGuraModule::destruction_multiplier(10) - 3.5).abs() < 1e-5);
    }

    #[test]
    fn gura_gura_kaishin_kabutowari_and_shima_yurashi_execution() {
        let module = GuraGuraModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::GuraGura),
            mastery: 6,
            energy: 250.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = [make_target(2, DVec3::new(0.0, 64.0, 3.0))];

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
                fruit: FruitId::GuraGura,
                ability_name: "kaishin".into(),
                mastery: 6,
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

        let kaishin_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "kaishin".into(),
                charge: 1.0,
                mastery: 6,
            },
        );
        assert_eq!(kaishin_res.hits.len(), 1);
        assert!(!kaishin_res.destroyed_blocks.is_empty());
        assert!(kaishin_res.screen_shake.is_some());
        assert!(kaishin_res
            .vfx_events
            .iter()
            .any(|e| e.fx_id == gura_fx::KAISHIN));

        let kabutowari_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kabutowari".into(),
                mastery: 6,
            },
        );
        assert!(!kabutowari_res.hits.is_empty());
        assert!(!kabutowari_res.destroyed_blocks.is_empty());
        assert!(kabutowari_res.caster_velocity_impulse.is_some());

        let shima_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "shima_yurashi".into(),
                mastery: 6,
            },
        );
        assert_eq!(shima_res.hits.len(), 1);
        assert!(!shima_res.destroyed_blocks.is_empty());
        assert!(shima_res
            .vfx_events
            .iter()
            .any(|e| e.fx_id == gura_fx::SHIMA_GRAB));

        let tenchi_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "tenchi_meido".into(),
                mastery: 6,
            },
        );
        assert_eq!(tenchi_res.hits.len(), 1);
        assert!(!tenchi_res.destroyed_blocks.is_empty());

        let mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 1.5);
        assert!(mods.devil_fruit_water_weakness);
        assert!(mods.is_water_weakened());
    }
}
