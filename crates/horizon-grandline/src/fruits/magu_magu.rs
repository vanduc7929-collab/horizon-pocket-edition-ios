//! Magu Magu no Mi (Magma Logia) — Grand Line 0.4.0 1:1 port.
//!
//! Mirrors `fruits/magu_magu/{MaguMagu,MaguAbilities,MaguEvents,MaguFx,LavaBody,MagmaCrust,MagmaWalk,Puddles,TouchBurn}.java`
//! and `fruits/magu_magu/ability/{MeigoAbility,InugamiGurenAbility,BakuretsuKazanAbility,
//! VolcanicLaunchAbility,VolcanicLaunch,DaiFunkaAbility,KazanBakuhatsuAbility,RyuseiKazanAbility,MagmaBlast,Meteors}.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastBudget, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::FormRuntime;
use crate::core::fruit::FruitId;
use crate::core::logia::{LogiaInteraction, limbs};
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Canonical Magu-Magu constants (`MaguMagu.java`, `MagmaWalk.java`, `TouchBurn.java`, abilities)
// ---------------------------------------------------------------------------

/// `MaguMagu.java` settings for Magma Walk, puddles, touch burn, and lava energy regen.
pub const MAGMA_WALK_EVERY_TICKS: i64 = 2;
pub const MAGMA_WALK_RADIUS: f64 = 2.5;
pub const MAGMA_WALK_BLOCKS_PER_STEP: usize = 40;
pub const CRUST_SECONDS: f64 = 20.0;
pub const CRUST_TICKS: i32 = 400; // 20.0s * 20

pub const PUDDLE_SECONDS: f64 = 10.0;
pub const PUDDLE_TICKS: i32 = 200;
pub const PUDDLES_PER_CAST: usize = 40;
pub const PUDDLES_PER_PLAYER: usize = 250;
pub const PUDDLE_DAMAGE: f32 = 2.0;
pub const PUDDLE_FIRE_SECONDS: i32 = 4;

pub const DROPLET_DAMAGE: f32 = 1.0;
pub const TOUCH_BURN_DAMAGE: f32 = 2.0;
pub const TOUCH_BURN_FIRE_SECONDS: i32 = 3;
pub const TOUCH_BURN_WEAPON_MELT: i32 = 4;
pub const LAVA_ENERGY_PER_SECOND: f32 = 6.0;

/// `MeigoAbility.java` constants.
pub const MEIGO_ENERGY: f32 = 22.0;
pub const MEIGO_COOLDOWN_TICKS: i32 = 160;
pub const MEIGO_UNLOCK_MASTERY: u32 = 1;
pub const MEIGO_DAMAGE: f32 = 20.0;
pub const MEIGO_RANGE: f32 = 3.5;
pub const MEIGO_KNOCKBACK: f32 = 1.8;
pub const MEIGO_WINDUP_TICKS: i32 = 8;
pub const MEIGO_LUNGE_BLOCKS: f64 = 2.0;
pub const MEIGO_ARMOR_IGNORED: f32 = 0.5;
pub const MEIGO_FIRE_SECONDS: i32 = 6;
pub const MEIGO_BLOBS: i32 = 10;

/// `InugamiGurenAbility.java` constants.
pub const INUGAMI_ENERGY: f32 = 28.0;
pub const INUGAMI_COOLDOWN_TICKS: i32 = 240;
pub const INUGAMI_UNLOCK_MASTERY: u32 = 2;
pub const INUGAMI_DAMAGE: f32 = 14.0;
pub const INUGAMI_RANGE: f32 = 32.0;
pub const INUGAMI_KNOCKBACK: f32 = 1.1;
pub const INUGAMI_SPEED: f64 = 2.0;
pub const INUGAMI_RADIUS: f64 = 3.5;
pub const INUGAMI_FIRE_SECONDS: i32 = 6;
pub const INUGAMI_SIZE: f64 = 3.0;

/// `BakuretsuKazanAbility.java` constants.
pub const BAKURETSU_ENERGY: f32 = 30.0;
pub const BAKURETSU_COOLDOWN_TICKS: i32 = 280;
pub const BAKURETSU_UNLOCK_MASTERY: u32 = 3;
pub const BAKURETSU_DAMAGE: f32 = 12.0;
pub const BAKURETSU_RANGE: f32 = 6.0;
pub const BAKURETSU_KNOCKBACK: f32 = 0.5;
pub const BAKURETSU_RING_TICKS: i32 = 12;
pub const BAKURETSU_LAUNCH: f64 = 0.95;
pub const BAKURETSU_FIRE_SECONDS: i32 = 5;
pub const BAKURETSU_CRATER_RADIUS: f64 = 2.6;
pub const BAKURETSU_CRATER_BLOCKS: usize = 60;

/// `VolcanicLaunchAbility.java` & `VolcanicLaunch.java` constants.
pub const VOLCANIC_LAUNCH_ENERGY: f32 = 18.0;
pub const VOLCANIC_LAUNCH_COOLDOWN_TICKS: i32 = 120;
pub const VOLCANIC_LAUNCH_UNLOCK_MASTERY: u32 = 4;
pub const VOLCANIC_LAUNCH_DAMAGE: f32 = 5.0;
pub const VOLCANIC_LAUNCH_RANGE: f32 = 36.0;
pub const VOLCANIC_LAUNCH_KNOCKBACK: f32 = 0.9;
pub const VOLCANIC_LAUNCH_SINK_TICKS: i32 = 6;
pub const VOLCANIC_LAUNCH_SPEED: f64 = 2.2;
pub const VOLCANIC_LAUNCH_HARD_CAP: f64 = 3.5;
pub const VOLCANIC_LAUNCH_GRAVITY: f64 = 0.085;
pub const VOLCANIC_LAUNCH_MAX_FLIGHT_TICKS: i32 = 60;
pub const VOLCANIC_LAUNCH_LAND_RADIUS: f64 = 2.5;
pub const VOLCANIC_LAUNCH_DIVE_RADIUS: f64 = 4.0;
pub const VOLCANIC_LAUNCH_DIVE_DAMAGE: f32 = 8.0;
pub const VOLCANIC_LAUNCH_TAKEOFF_DAMAGE: f32 = 3.0;
pub const VOLCANIC_LAUNCH_PATH_DAMAGE: f32 = 4.0;

/// `DaiFunkaAbility.java` constants.
pub const DAI_FUNKA_ENERGY: f32 = 40.0;
pub const DAI_FUNKA_COOLDOWN_TICKS: i32 = 400;
pub const DAI_FUNKA_UNLOCK_MASTERY: u32 = 5;
pub const DAI_FUNKA_DAMAGE: f32 = 22.0;
pub const DAI_FUNKA_RANGE: f32 = 40.0;
pub const DAI_FUNKA_KNOCKBACK: f32 = 1.6;
pub const DAI_FUNKA_CHARGE_TICKS: i32 = 24;
pub const DAI_FUNKA_SPEED: f64 = 2.2;
pub const DAI_FUNKA_WIDTH: f64 = 3.5;
pub const DAI_FUNKA_MIN_WIDTH: f64 = 1.6;
pub const DAI_FUNKA_TUNNEL_BLOCKS: usize = 260;
pub const DAI_FUNKA_FIRE_SECONDS: i32 = 8;
pub const DAI_FUNKA_METEORS: usize = 8;
pub const DAI_FUNKA_METEOR_DAMAGE: f32 = 10.0;
pub const DAI_FUNKA_METEOR_RADIUS: f64 = 2.5;

/// `KazanBakuhatsuAbility.java` constants.
pub const KAZAN_ENERGY: f32 = 24.0;
pub const KAZAN_COOLDOWN_TICKS: i32 = 240;
pub const KAZAN_UNLOCK_MASTERY: u32 = 6;
pub const KAZAN_DAMAGE: f32 = 14.0;
pub const KAZAN_RANGE: f32 = 30.0;
pub const KAZAN_KNOCKBACK: f32 = 0.2;
pub const KAZAN_TELEGRAPH_TICKS: i32 = 10;
pub const KAZAN_RADIUS: f64 = 3.0;
pub const KAZAN_HEIGHT: f64 = 6.0;
pub const KAZAN_LAUNCH: f64 = 1.15;
pub const KAZAN_FIRE_SECONDS: i32 = 5;

/// `RyuseiKazanAbility.java` constants.
pub const RYUSEI_ENERGY: f32 = 90.0;
pub const RYUSEI_COOLDOWN_TICKS: i32 = 2400;
pub const RYUSEI_UNLOCK_MASTERY: u32 = 9;
pub const RYUSEI_DAMAGE: f32 = 18.0;
pub const RYUSEI_RANGE: f32 = 40.0;
pub const RYUSEI_KNOCKBACK: f32 = 1.2;
pub const RYUSEI_WINDUP_TICKS: i32 = 20;
pub const RYUSEI_FISTS: usize = 14;
pub const RYUSEI_DELAY_TICKS: i32 = 30;
pub const RYUSEI_BARRAGE_TICKS: i32 = 80;
pub const RYUSEI_RADIUS: f64 = 24.0;
pub const RYUSEI_IMPACT_RADIUS: f64 = 2.5;
pub const RYUSEI_CRATER_BLOCKS: usize = 400;
pub const RYUSEI_FIRE_SECONDS: i32 = 8;

/// Canonical Magu-Magu VFX resource IDs (`MaguFx.java`).
pub mod magu_fx {
    pub const AWAKEN: &str = "devilfruits:magu_magu/awaken";
    pub const LEVEL_UP: &str = "devilfruits:magu_magu/level_up";
    pub const REINCARNATE: &str = "devilfruits:magu_magu/reincarnate";
    pub const WALK: &str = "devilfruits:magu_magu/walk";
    pub const STEAM: &str = "devilfruits:magu_magu/steam";
    pub const SPLASH: &str = "devilfruits:magu_magu/splash";
    pub const TOUCH_BURN: &str = "devilfruits:magu_magu/touch_burn";
    pub const BURN_UP: &str = "devilfruits:magu_magu/burn_up";
    pub const SEAR: &str = "devilfruits:magu_magu/sear";
    pub const MEIGO_WINDUP: &str = "devilfruits:magu_magu/meigo_windup";
    pub const MEIGO: &str = "devilfruits:magu_magu/meigo";
    pub const BAKURETSU: &str = "devilfruits:magu_magu/bakuretsu";
    pub const BAKURETSU_WINDUP: &str = "devilfruits:magu_magu/bakuretsu_windup";
    pub const KAZAN_CRACKS: &str = "devilfruits:magu_magu/kazan_cracks";
    pub const KAZAN_ERUPT: &str = "devilfruits:magu_magu/kazan_erupt";
    pub const LAUNCH_SINK: &str = "devilfruits:magu_magu/launch_sink";
    pub const LAUNCH: &str = "devilfruits:magu_magu/launch";
    pub const LAUNCH_DIVE: &str = "devilfruits:magu_magu/launch_dive";
    pub const LAUNCH_LAND: &str = "devilfruits:magu_magu/launch_land";
    pub const MAGMA_BREAK: &str = "devilfruits:magu_magu/magma_break";
    pub const INUGAMI: &str = "devilfruits:magu_magu/inugami";
    pub const INUGAMI_BITE: &str = "devilfruits:magu_magu/inugami_bite";
    pub const DAI_FUNKA_CHARGE: &str = "devilfruits:magu_magu/dai_funka_charge";
    pub const DAI_FUNKA: &str = "devilfruits:magu_magu/dai_funka";
    pub const DAI_FUNKA_BURST: &str = "devilfruits:magu_magu/dai_funka_burst";
    pub const METEORS: &str = "devilfruits:magu_magu/meteors";
    pub const METEOR_IMPACT: &str = "devilfruits:magu_magu/meteor_impact";
    pub const RYUSEI_WINDUP: &str = "devilfruits:magu_magu/ryusei_windup";
    pub const RYUSEI_LAUNCH: &str = "devilfruits:magu_magu/ryusei_launch";
    pub const RYUSEI_FALL: &str = "devilfruits:magu_magu/ryusei_fall";
    pub const RYUSEI_IMPACT: &str = "devilfruits:magu_magu/ryusei_impact";
}

/// Canonical Magu-Magu animation clip IDs.
pub mod magu_anim {
    pub const MEIGO: &str = "devilfruits:magu_magu/meigo";
    pub const INUGAMI_PUNCH: &str = "devilfruits:magu_magu/inugami_punch";
    pub const BAKURETSU_SLAM: &str = "devilfruits:magu_magu/bakuretsu_slam";
    pub const LAUNCH_SINK: &str = "devilfruits:magu_magu/launch_sink";
    pub const LAUNCH_ERUPT: &str = "devilfruits:magu_magu/launch_erupt";
    pub const LAUNCH_DIVE: &str = "devilfruits:magu_magu/launch_dive";
    pub const DAI_FUNKA_CHARGE: &str = "devilfruits:magu_magu/dai_funka_charge";
    pub const DAI_FUNKA_RELEASE: &str = "devilfruits:magu_magu/dai_funka_release";
    pub const KAZAN_POINT: &str = "devilfruits:magu_magu/kazan_bakuhatsu_point";
    pub const RYUSEI_RAISE: &str = "devilfruits:magu_magu/ryusei_raise";
    pub const RYUSEI_HOLD: &str = "devilfruits:magu_magu/ryusei_hold";
}

/// Magu Magu no Mi Devil Fruit module (`MaguMagu.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct MaguMaguModule;

impl MaguMaguModule {
    pub const fn new() -> Self {
        Self
    }

    /// Evaluates Magma vs Ice Logia superiority (`MaguMagu.registerInteractions`).
    pub fn interaction_with_ice(attacker_is_magma: bool) -> LogiaInteraction {
        if attacker_is_magma {
            LogiaInteraction::evaluate("magma", "ice")
        } else {
            LogiaInteraction::evaluate("ice", "magma")
        }
    }

    /// Exact ballistic arc launch velocity from `VolcanicLaunch.launchVelocity(from, to, cap)`.
    pub fn volcanic_launch_velocity(from: DVec3, to: DVec3, cap: f64) -> DVec3 {
        let dx = to.x - from.x;
        let dz = to.z - from.z;
        let d = (dx * dx + dz * dz).sqrt();
        let dy = to.y - from.y;
        let apex = (d * 0.3).max(3.0) + dy.max(0.0);
        let vy = (0.17 * apex).sqrt();
        let fall = apex - dy;
        let t = vy / VOLCANIC_LAUNCH_GRAVITY
            + (2.0 * fall.max(0.5) / VOLCANIC_LAUNCH_GRAVITY).sqrt();
        let vh = d / t.max(1.0);
        let dir = if d < 0.001 {
            DVec3::ZERO
        } else {
            DVec3::new(dx / d, 0.0, dz / d)
        };
        let v = dir * vh + DVec3::new(0.0, vy, 0.0);
        let len = v.length();
        if len > cap {
            v * (cap / len)
        } else {
            v
        }
    }

    /// Dissolves limbs briefly when casting magma techniques (`LogiaManager.partial`).
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
}

impl DevilFruitModule for MaguMaguModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::MaguMagu
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let allowed = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "meigo"),
                display_name: "Meigo".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: MEIGO_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: MEIGO_COOLDOWN_TICKS,
                    unlock_mastery: MEIGO_UNLOCK_MASTERY,
                    damage: MEIGO_DAMAGE,
                    range: MEIGO_RANGE,
                    knockback: MEIGO_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "inugami_guren"),
                display_name: "Inugami Guren".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: INUGAMI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: INUGAMI_COOLDOWN_TICKS,
                    unlock_mastery: INUGAMI_UNLOCK_MASTERY,
                    damage: INUGAMI_DAMAGE,
                    range: INUGAMI_RANGE,
                    knockback: INUGAMI_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "bakuretsu_kazan"),
                display_name: "Bakuretsu Kazan".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: BAKURETSU_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: BAKURETSU_COOLDOWN_TICKS,
                    unlock_mastery: BAKURETSU_UNLOCK_MASTERY,
                    damage: BAKURETSU_DAMAGE,
                    range: BAKURETSU_RANGE,
                    knockback: BAKURETSU_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "volcanic_launch"),
                display_name: "Volcanic Launch".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: VOLCANIC_LAUNCH_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: VOLCANIC_LAUNCH_COOLDOWN_TICKS,
                    unlock_mastery: VOLCANIC_LAUNCH_UNLOCK_MASTERY,
                    damage: VOLCANIC_LAUNCH_DAMAGE,
                    range: VOLCANIC_LAUNCH_RANGE,
                    knockback: VOLCANIC_LAUNCH_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "dai_funka"),
                display_name: "Dai Funka".into(),
                cast_type: CastType::Charged,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: DAI_FUNKA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: DAI_FUNKA_COOLDOWN_TICKS,
                    unlock_mastery: DAI_FUNKA_UNLOCK_MASTERY,
                    damage: DAI_FUNKA_DAMAGE,
                    range: DAI_FUNKA_RANGE,
                    knockback: DAI_FUNKA_KNOCKBACK,
                    max_charge_ticks: DAI_FUNKA_CHARGE_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "kazan_bakuhatsu"),
                display_name: "Kazan Bakuhatsu".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: KAZAN_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: KAZAN_COOLDOWN_TICKS,
                    unlock_mastery: KAZAN_UNLOCK_MASTERY,
                    damage: KAZAN_DAMAGE,
                    range: KAZAN_RANGE,
                    knockback: KAZAN_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::MaguMagu, "ryusei_kazan"),
                display_name: "Ryusei Kazan".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: RYUSEI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: RYUSEI_COOLDOWN_TICKS,
                    unlock_mastery: RYUSEI_UNLOCK_MASTERY,
                    damage: RYUSEI_DAMAGE,
                    range: RYUSEI_RANGE,
                    knockback: RYUSEI_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: true,
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
            damage_type: Some(GrandLineDamageType::MaguMagma),
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
        let stripped = raw_name
            .split(':')
            .next_back()
            .unwrap_or(raw_name)
            .split('/')
            .next_back()
            .unwrap_or(raw_name);

        let Some(def) = self.ability(stripped) else {
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

        match def.id.name.as_str() {
            "meigo" => {
                res.damage_type = Some(GrandLineDamageType::MaguMeigo);
                res.animation_clip = Some(magu_anim::MEIGO.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 22);

                ctx.world_fx.emit_fx(
                    magu_fx::MEIGO_WINDUP,
                    0,
                    eye,
                    look,
                    0.0,
                    now,
                    MEIGO_WINDUP_TICKS,
                    64.0,
                );

                // Find best target within `range + lunge` inside 28° cone (`MeigoAbility.target`).
                let reach = scaled_range + MEIGO_LUNGE_BLOCKS;
                let target = CombatTargeting::aimed(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    reach,
                    28.0_f64.to_radians(),
                );

                let lunge = if let Some(t) = target {
                    (eye.distance(t.center) - 1.4).clamp(0.0, MEIGO_LUNGE_BLOCKS)
                } else {
                    MEIGO_LUNGE_BLOCKS
                };
                if lunge > 0.1 && horiz_look.length_squared() > 1e-6 {
                    res.caster_velocity_impulse =
                        Some(horiz_look * (lunge * 0.45) + DVec3::new(0.0, 0.05, 0.0));
                }

                if let Some(t) = target {
                    let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.35, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MaguMeigo,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: MEIGO_FIRE_SECONDS,
                    });
                    ctx.world_fx.emit_fx(
                        magu_fx::MEIGO,
                        0,
                        t.center,
                        look,
                        1.0,
                        now,
                        MEIGO_BLOBS,
                        96.0,
                    );
                    ctx.world_fx
                        .emit_fx(magu_fx::SEAR, 0, t.center, DVec3::ZERO, 1.0, now, 50, 64.0);
                } else {
                    let fist = eye + look * 2.0;
                    ctx.world_fx
                        .emit_fx(magu_fx::MEIGO, 0, fist, look, 0.0, now, -1, 96.0);
                }
            }
            "inugami_guren" => {
                res.animation_clip = Some(magu_anim::INUGAMI_PUNCH.into());
                res.spawned_projectiles = 1;
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 22);

                let start = eye + look * 1.2 + DVec3::new(0.0, -0.35, 0.0);
                let vel = look * INUGAMI_SPEED;
                let life = ((scaled_range / INUGAMI_SPEED).ceil() as i32).max(1);
                let half = INUGAMI_SIZE * 0.5;
                let end = start + look * scaled_range;

                ctx.world_fx.emit_fx(
                    magu_fx::INUGAMI,
                    0,
                    start,
                    vel,
                    INUGAMI_SIZE as f32,
                    now,
                    life,
                    128.0,
                );

                // Capsule travel + bite eruption of radius 3.5 (`InugamiGurenAbility.bite`).
                let capsule_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    half * 0.75,
                );
                let bite_center = capsule_hits.first().map(|t| t.center).unwrap_or(end);
                let bite_hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    bite_center,
                    INUGAMI_RADIUS,
                );

                for t in bite_hits {
                    let away = DVec3::new(t.center.x - bite_center.x, 0.0, t.center.z - bite_center.z)
                        .normalize_or_zero();
                    let push = away * (def.stats.knockback as f64) + DVec3::new(0.0, 0.5, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MaguMagma,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: INUGAMI_FIRE_SECONDS,
                    });
                }

                ctx.world_fx.emit_fx(
                    magu_fx::INUGAMI_BITE,
                    0,
                    bite_center,
                    vel,
                    INUGAMI_RADIUS as f32,
                    now,
                    if capsule_hits.is_empty() { 1 } else { 2 },
                    128.0,
                );
                ctx.world_fx.emit_shake(bite_center, 0.7, 20);
                res.screen_shake = Some((0.7, 20));
            }
            "bakuretsu_kazan" => {
                res.damage_type = Some(GrandLineDamageType::MaguEruption);
                res.animation_clip = Some(magu_anim::BAKURETSU_SLAM.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 26);

                let slam_pos = eye + horiz_look * 1.1 + DVec3::new(0.0, -1.5, 0.0);
                ctx.world_fx.emit_fx(
                    magu_fx::BAKURETSU_WINDUP,
                    0,
                    eye,
                    horiz_look,
                    0.0,
                    now,
                    10,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    magu_fx::BAKURETSU,
                    0,
                    slam_pos,
                    horiz_look,
                    scaled_range as f32,
                    now,
                    BAKURETSU_RING_TICKS,
                    128.0,
                );
                ctx.world_fx.emit_shake(slam_pos, 0.9, 24);
                res.screen_shake = Some((0.9, 24));

                let mut crater_budget = CastBudget::new(BAKURETSU_CRATER_BLOCKS, 8);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    slam_pos,
                    BAKURETSU_CRATER_RADIUS,
                    &mut crater_budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );

                // Expanding ring of geysers up to `range` (`BakuretsuKazanAbility.slam`).
                let hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    slam_pos,
                    scaled_range + 1.5,
                );
                for t in hits {
                    let dist = DVec3::new(t.center.x - slam_pos.x, 0.0, t.center.z - slam_pos.z)
                        .length();
                    let falloff = (1.0 - 0.35 * (dist / scaled_range.max(1.0))).clamp(0.4, 1.0) as f32;
                    let away = DVec3::new(t.center.x - slam_pos.x, 0.0, t.center.z - slam_pos.z)
                        .normalize_or_zero();
                    let push =
                        away * (def.stats.knockback as f64) + DVec3::new(0.0, BAKURETSU_LAUNCH, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg * falloff,
                        damage_type: GrandLineDamageType::MaguEruption,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: BAKURETSU_FIRE_SECONDS,
                    });
                }
            }
            "volcanic_launch" => {
                if let AbilityEvent::Toggled { enabled, .. } = event {
                    res.cast_result = Some(CastResult::Toggled);
                    if *enabled {
                        res.damage_type = Some(GrandLineDamageType::MaguEruption);
                        res.animation_clip = Some(magu_anim::LAUNCH_ERUPT.into());
                        Self::dissolve_arms(ctx, limbs::ALL, VOLCANIC_LAUNCH_MAX_FLIGHT_TICKS);

                        let target = ctx
                            .ability_ctx
                            .aim_point
                            .unwrap_or(eye + look * scaled_range);
                        let launch_vel = Self::volcanic_launch_velocity(
                            eye,
                            target,
                            VOLCANIC_LAUNCH_SPEED.min(VOLCANIC_LAUNCH_HARD_CAP),
                        );
                        res.caster_velocity_impulse = Some(launch_vel);

                        ctx.world_fx.emit_fx(
                            magu_fx::LAUNCH_SINK,
                            0,
                            eye,
                            look,
                            0.0,
                            now,
                            VOLCANIC_LAUNCH_SINK_TICKS,
                            64.0,
                        );
                        ctx.world_fx.emit_fx(
                            magu_fx::LAUNCH,
                            0,
                            eye,
                            launch_vel,
                            VOLCANIC_LAUNCH_GRAVITY as f32,
                            now,
                            VOLCANIC_LAUNCH_MAX_FLIGHT_TICKS,
                            128.0,
                        );

                        // Takeoff blast around caster (`takeoffDamage = 3.0`).
                        let takeoff_hits = CombatTargeting::in_radius(
                            ctx.nearby_targets,
                            ctx.player_id,
                            eye,
                            2.2,
                        );
                        for t in takeoff_hits {
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: VOLCANIC_LAUNCH_TAKEOFF_DAMAGE,
                                damage_type: GrandLineDamageType::MaguEruption,
                                knockback_impulse: DVec3::new(0.0, 0.6, 0.0),
                                stun_ticks: 0,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 3,
                            });
                        }
                    } else {
                        res.damage_type = Some(GrandLineDamageType::MaguEruption);
                        ctx.world_fx.emit_fx(
                            magu_fx::LAUNCH_LAND,
                            0,
                            eye,
                            DVec3::ZERO,
                            VOLCANIC_LAUNCH_LAND_RADIUS as f32,
                            now,
                            0,
                            128.0,
                        );
                        ctx.world_fx.emit_shake(eye, 0.6, 18);
                        res.screen_shake = Some((0.6, 18));
                    }
                }
            }
            "dai_funka" => {
                match event {
                    AbilityEvent::ChargeStarted { max_ticks, .. } => {
                        res.cast_result = Some(CastResult::Charging);
                        res.animation_clip = Some(magu_anim::DAI_FUNKA_CHARGE.into());
                        Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 20);
                        ctx.world_fx.emit_fx(
                            magu_fx::DAI_FUNKA_CHARGE,
                            0,
                            eye,
                            look,
                            0.0,
                            now,
                            *max_ticks,
                            64.0,
                        );
                        res.vfx_events = ctx.world_fx.drain_vfx();
                        return res;
                    }
                    AbilityEvent::ChargeCancelled { .. } => {
                        ctx.world_fx.emit_fx(
                            magu_fx::DAI_FUNKA_CHARGE,
                            0,
                            eye,
                            DVec3::ZERO,
                            -1.0,
                            0,
                            0,
                            64.0,
                        );
                        res.vfx_events = ctx.world_fx.drain_vfx();
                        return res;
                    }
                    _ => {}
                }

                let charge = match event {
                    AbilityEvent::ChargeReleased { charge, .. } => *charge,
                    _ => ctx.ability_ctx.charge,
                }
                .clamp(0.0, 1.0);

                res.animation_clip = Some(magu_anim::DAI_FUNKA_RELEASE.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, 19);

                // `width = lerp(charge, 1.6, 3.5)`, `damage = scaled * (0.55 + 0.45 * charge)`
                let width = DAI_FUNKA_MIN_WIDTH
                    + (DAI_FUNKA_WIDTH - DAI_FUNKA_MIN_WIDTH) * (charge as f64);
                let fist_dmg = base_scaled_dmg * (0.55 + 0.45 * charge);
                let speed = DAI_FUNKA_SPEED.min(4.0);
                let vel = look * speed;
                let life = ((scaled_range / speed).ceil() as i32).max(1);
                let start = eye + look * 1.2 + DVec3::new(0.0, -0.35, 0.0);
                let end = start + look * scaled_range;

                // 1 giant magma fist + 8 volcanic rock meteors on burst (`DaiFunkaAbility.burst`).
                res.spawned_projectiles = 1 + DAI_FUNKA_METEORS;

                ctx.world_fx.emit_fx(
                    magu_fx::DAI_FUNKA,
                    0,
                    start,
                    vel,
                    width as f32,
                    now,
                    life,
                    160.0,
                );

                // Melt tunnel blocks and transform underlying water into obsidian crust.
                let mut tunnel_budget = CastBudget::new(DAI_FUNKA_TUNNEL_BLOCKS, 8);
                let broken = ctx.world_fx.carve_spherical_crater(
                    start + look * (width * 0.5),
                    width * 0.5,
                    &mut tunnel_budget,
                    true,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );
                res.destroyed_blocks = broken;

                let mut crust_budget = CastBudget::new(120, 0);
                let crust_pos = [
                    start.x.floor() as i32,
                    (start.y - width * 0.5 - 1.0).floor() as i32,
                    start.z.floor() as i32,
                ];
                if ctx.world_fx.transform_block(
                    crust_pos,
                    "minecraft:water",
                    "minecraft:obsidian",
                    1.0,
                    false,
                    &mut crust_budget,
                    CRUST_TICKS,
                    Some("minecraft:water"),
                    now,
                    ctx.config,
                ) {
                    res.transformed_blocks
                        .push((crust_pos, "minecraft:obsidian".into()));
                }

                // Direct capsule hits along the giant fist tunnel.
                let capsule_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    start,
                    end,
                    width * 0.5,
                );
                let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.45, 0.0);
                for t in capsule_hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: fist_dmg,
                        damage_type: GrandLineDamageType::MaguMagma,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: DAI_FUNKA_FIRE_SECONDS,
                    });
                }

                // End burst + 8 meteor rain (`DaiFunkaAbility.burst` & `Meteors.rain`).
                ctx.world_fx.emit_fx(
                    magu_fx::DAI_FUNKA_BURST,
                    0,
                    end,
                    vel,
                    width as f32,
                    now,
                    0,
                    160.0,
                );
                ctx.world_fx.emit_fx(
                    magu_fx::METEORS,
                    0,
                    end,
                    vel,
                    0.85,
                    now,
                    DAI_FUNKA_METEORS as i32,
                    160.0,
                );
                ctx.world_fx.emit_shake(end, 1.0, 32);
                res.screen_shake = Some((1.0, 32));
            }
            "kazan_bakuhatsu" => {
                res.damage_type = Some(GrandLineDamageType::MaguEruption);
                res.animation_clip = Some(magu_anim::KAZAN_POINT.into());
                Self::dissolve_arms(ctx, limbs::RIGHT_ARM, KAZAN_TELEGRAPH_TICKS + 6);

                let point = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or_else(|| {
                        CombatTargeting::aimed(
                            ctx.nearby_targets,
                            ctx.player_id,
                            eye,
                            look,
                            scaled_range,
                            30.0_f64.to_radians(),
                        )
                        .map(|t| t.center)
                        .unwrap_or(eye + look * scaled_range)
                    });

                ctx.world_fx.emit_fx(
                    magu_fx::KAZAN_CRACKS,
                    0,
                    point,
                    point - eye,
                    0.0,
                    now,
                    KAZAN_TELEGRAPH_TICKS,
                    128.0,
                );
                ctx.world_fx.emit_fx(
                    magu_fx::KAZAN_ERUPT,
                    0,
                    point,
                    DVec3::ZERO,
                    KAZAN_RADIUS as f32,
                    now,
                    KAZAN_HEIGHT as i32,
                    128.0,
                );
                ctx.world_fx.emit_shake(point, 0.7, 20);
                res.screen_shake = Some((0.7, 20));

                let mut budget = CastBudget::new(24, 6);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    point,
                    1.4,
                    &mut budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );

                // Eruption column (`radius = 3.0`, `height = 6.0`, `launch = 1.15`).
                let hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    point,
                    KAZAN_RADIUS + KAZAN_HEIGHT,
                );
                for t in hits {
                    let dy = t.center.y - point.y;
                    let flat = DVec3::new(t.center.x - point.x, 0.0, t.center.z - point.z).length();
                    if flat <= KAZAN_RADIUS + t.width * 0.5 && (-1.0..=KAZAN_HEIGHT).contains(&dy) {
                        let away = DVec3::new(t.center.x - point.x, 0.0, t.center.z - point.z)
                            .normalize_or_zero();
                        let push =
                            away * (def.stats.knockback as f64) + DVec3::new(0.0, KAZAN_LAUNCH, 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: base_scaled_dmg,
                            damage_type: GrandLineDamageType::MaguEruption,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: KAZAN_FIRE_SECONDS,
                        });
                    }
                }
            }
            "ryusei_kazan" => {
                res.damage_type = Some(GrandLineDamageType::MaguEruption);
                res.animation_clip = Some(magu_anim::RYUSEI_RAISE.into());
                res.spawned_projectiles = RYUSEI_FISTS;
                Self::dissolve_arms(ctx, limbs::ARMS, RYUSEI_WINDUP_TICKS + 20);

                let aim = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(eye + look * scaled_range);

                ctx.world_fx.emit_fx(
                    magu_fx::RYUSEI_WINDUP,
                    0,
                    aim,
                    DVec3::ZERO,
                    RYUSEI_RADIUS as f32,
                    now,
                    RYUSEI_WINDUP_TICKS,
                    192.0,
                );
                ctx.world_fx.emit_fx(
                    magu_fx::RYUSEI_LAUNCH,
                    0,
                    eye,
                    aim - eye,
                    RYUSEI_RADIUS as f32,
                    now,
                    RYUSEI_FISTS as i32,
                    192.0,
                );
                ctx.world_fx.emit_fx(
                    magu_fx::RYUSEI_IMPACT,
                    0,
                    aim,
                    DVec3::ZERO,
                    RYUSEI_IMPACT_RADIUS as f32,
                    now,
                    0,
                    192.0,
                );
                ctx.world_fx.emit_shake(aim, 0.9, 30);
                res.screen_shake = Some((0.9, 30));

                // Carve impact craters & crust water around aim point (`RyuseiKazanAbility.land`).
                let mut crater_budget = CastBudget::new(RYUSEI_CRATER_BLOCKS, 20);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    aim,
                    RYUSEI_IMPACT_RADIUS,
                    &mut crater_budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );

                let mut crust_budget = CastBudget::new(60, 0);
                let crust_pos = [
                    aim.x.floor() as i32,
                    aim.y.floor() as i32,
                    aim.z.floor() as i32,
                ];
                if ctx.world_fx.transform_block(
                    crust_pos,
                    "minecraft:water",
                    "minecraft:obsidian",
                    1.0,
                    false,
                    &mut crust_budget,
                    CRUST_TICKS,
                    Some("minecraft:water"),
                    now,
                    ctx.config,
                ) {
                    res.transformed_blocks
                        .push((crust_pos, "minecraft:obsidian".into()));
                }

                let barrage_hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    aim,
                    RYUSEI_RADIUS,
                );
                for t in barrage_hits {
                    let away =
                        DVec3::new(t.center.x - aim.x, 0.0, t.center.z - aim.z).normalize_or_zero();
                    let push = away * 0.9 + DVec3::new(0.0, 0.8, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::MaguEruption,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: RYUSEI_FIRE_SECONDS,
                    });
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

        // 1. `MaguEvents.onPlayerTick`: while in lava and not suppressed (`LavaBody.active`),
        // grant +`lavaEnergyPerSecond` (`6.0 / 20.0 = 0.3` energy/tick).
        if ctx.in_lava && !ctx.suppressed {
            let max_e = ctx.player.max_energy();
            if ctx.player.energy < max_e {
                ctx.player.energy = (ctx.player.energy + LAVA_ENERGY_PER_SECOND / 20.0).min(max_e);
            }
        }

        // 2. `MagmaWalk.tick`: every 2 ticks when not underwater, not suppressed, and
        // "seastone" counter is inactive, crust water into `minecraft:obsidian` (`crustTicks = 400`).
        let seastone_counter = ctx
            .logia
            .as_deref()
            .is_some_and(|l| l.is_counter_active("seastone", now));

        if (now % MAGMA_WALK_EVERY_TICKS) == 0
            && !ctx.submerged_in_water
            && !ctx.suppressed
            && !seastone_counter
        {
            let centre = ctx.ability_ctx.eye_pos + DVec3::new(0.0, -2.2, 0.0);
            let cx = centre.x.floor() as i32;
            let cy = centre.y.floor() as i32;
            let cz = centre.z.floor() as i32;
            let r = MAGMA_WALK_RADIUS.ceil() as i32;
            let r2 = MAGMA_WALK_RADIUS * MAGMA_WALK_RADIUS;
            let mut walk_budget = CastBudget::new(MAGMA_WALK_BLOCKS_PER_STEP, 0);

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
                                    "minecraft:obsidian",
                                    1.0,
                                    false,
                                    &mut walk_budget,
                                    CRUST_TICKS + (r - ring) * 4,
                                    Some("minecraft:water"),
                                    now,
                                    ctx.config,
                                ) {
                                    res.transformed_blocks
                                        .push((pos, "minecraft:obsidian".into()));
                                }
                            }
                        }
                    }
                }
            }

            if !res.transformed_blocks.is_empty() {
                ctx.world_fx.emit_fx(
                    magu_fx::WALK,
                    0,
                    centre + DVec3::new(0.0, 1.0, 0.0),
                    DVec3::ZERO,
                    MAGMA_WALK_RADIUS as f32,
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
        // `LavaBody.java`: fire immunity + lava swimming (`FireProof` & `FluidSwim`).
        mods.fire_immune = true;
        mods.swim_in_lava = true;
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
    use crate::core::combat::TargetCandidate;
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::logia::{DamageCategory, HitRule, LogiaData};
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
    fn magu_magu_abilities_and_logia_profile_match_java() {
        let module = MaguMaguModule::new();
        assert_eq!(module.fruit_id(), FruitId::MaguMagu);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::MaguMagma);

        let profile = module.logia_profile().expect("Magu-Magu is a Logia");
        assert_eq!(profile.element, "magma");
        assert_eq!(profile.elemental_drain_per_second, 4.0);
        assert_eq!(profile.dash_range, 24.0);
        assert_eq!(profile.dash_speed, 3.0);
        assert_eq!(profile.hit_rule(DamageCategory::Fire), HitRule::Pass);

        assert_eq!(
            MaguMaguModule::interaction_with_ice(true),
            LogiaInteraction::Bypass
        );
        assert_eq!(
            MaguMaguModule::interaction_with_ice(false),
            LogiaInteraction::Resisted
        );

        let meigo = module.ability("meigo").unwrap();
        assert_eq!(meigo.stats.energy_cost, 22.0);
        assert_eq!(meigo.stats.cooldown_ticks, 160);
        assert_eq!(meigo.stats.damage, 20.0);
        assert_eq!(meigo.stats.range, 3.5);

        let inugami = module.ability("inugami_guren").unwrap();
        assert_eq!(inugami.stats.energy_cost, 28.0);
        assert_eq!(inugami.stats.cooldown_ticks, 240);
        assert_eq!(inugami.stats.unlock_mastery, 2);
        assert_eq!(inugami.stats.damage, 14.0);
        assert_eq!(inugami.stats.range, 32.0);

        let dai_funka = module.ability("dai_funka").unwrap();
        assert_eq!(dai_funka.cast_type, CastType::Charged);
        assert_eq!(dai_funka.stats.energy_cost, 40.0);
        assert_eq!(dai_funka.stats.cooldown_ticks, 400);
        assert_eq!(dai_funka.stats.unlock_mastery, 5);
        assert_eq!(dai_funka.stats.damage, 22.0);
        assert_eq!(dai_funka.stats.max_charge_ticks, 24);

        let kazan = module.ability("kazan_bakuhatsu").unwrap();
        assert_eq!(kazan.stats.energy_cost, 24.0);
        assert_eq!(kazan.stats.cooldown_ticks, 240);
        assert_eq!(kazan.stats.unlock_mastery, 6);
        assert_eq!(kazan.stats.damage, 14.0);

        let ryusei = module.ability("ryusei_kazan").unwrap();
        assert!(ryusei.is_ultimate);
        assert_eq!(ryusei.stats.energy_cost, 90.0);
        assert_eq!(ryusei.stats.cooldown_ticks, 2400);
        assert_eq!(ryusei.stats.unlock_mastery, 9);
        assert_eq!(ryusei.stats.damage, 18.0);
    }

    #[test]
    fn magu_magu_execute_all_abilities_and_burn_durations() {
        let module = MaguMaguModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::MaguMagu),
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
        let targets = vec![make_target(2, DVec3::new(0.0, 64.0, 3.0))];

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
                fruit: FruitId::MaguMagu,
                ability_name: "meigo".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 100,
                eye_pos: DVec3::new(0.0, 64.0, 0.0),
                look_dir: DVec3::Z,
                aim_point: Some(DVec3::new(0.0, 64.0, 3.0)),
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

        // 1. Meigo lunges and deals MaguMeigo damage with 6s burn.
        let r_meigo = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "meigo".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_meigo.damage_type, Some(GrandLineDamageType::MaguMeigo));
        assert!(r_meigo.caster_velocity_impulse.is_some());
        assert_eq!(r_meigo.hits.len(), 1);
        assert_eq!(r_meigo.hits[0].set_fire_seconds, MEIGO_FIRE_SECONDS);

        // 2. Inugami Guren bites and burns for 6s.
        let r_inugami = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "inugami_guren".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_inugami.spawned_projectiles, 1);
        assert_eq!(r_inugami.hits.len(), 1);
        assert_eq!(r_inugami.hits[0].set_fire_seconds, INUGAMI_FIRE_SECONDS);

        // 3. Dai Funka at full charge launches giant fist + 8 meteors, burns for 8s, and transforms blocks.
        let r_dai = module.execute_ability(
            &mut ctx,
            &AbilityEvent::ChargeReleased {
                ability: "dai_funka".into(),
                charge: 1.0,
                mastery: 10,
            },
        );
        assert_eq!(r_dai.spawned_projectiles, 1 + DAI_FUNKA_METEORS);
        assert_eq!(r_dai.hits.len(), 1);
        assert_eq!(r_dai.hits[0].set_fire_seconds, DAI_FUNKA_FIRE_SECONDS);
        assert!(!r_dai.destroyed_blocks.is_empty());
        assert!(!r_dai.transformed_blocks.is_empty());

        // 4. Kazan Bakuhatsu erupts at aimed point with 5s burn and upward launch.
        let r_kazan = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "kazan_bakuhatsu".into(),
                mastery: 10,
            },
        );
        assert_eq!(
            r_kazan.damage_type,
            Some(GrandLineDamageType::MaguEruption)
        );
        assert_eq!(r_kazan.hits.len(), 1);
        assert_eq!(r_kazan.hits[0].set_fire_seconds, KAZAN_FIRE_SECONDS);
        assert!(r_kazan.hits[0].knockback_impulse.y > 1.0);

        // 5. Ryusei Kazan rains 14 magma fists with 8s burn.
        let r_ryusei = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "ryusei_kazan".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_ryusei.spawned_projectiles, RYUSEI_FISTS);
        assert_eq!(r_ryusei.hits.len(), 1);
        assert_eq!(r_ryusei.hits[0].set_fire_seconds, RYUSEI_FIRE_SECONDS);
    }

    #[test]
    fn magu_magu_lava_swim_fire_immunity_and_magma_walk() {
        let module = MaguMaguModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::MaguMagu),
            mastery: 5,
            energy: 50.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();

        // Check body modifiers: fire_immune & swim_in_lava when unsuppressed.
        let mods = module.body_modifiers(&player, &form_rt, false, 0.0);
        assert!(mods.fire_immune);
        assert!(mods.swim_in_lava);

        let sup_mods = module.body_modifiers(&player, &form_rt, true, 1.5);
        assert!(!sup_mods.fire_immune);
        assert!(!sup_mods.swim_in_lava);

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
                fruit: FruitId::MaguMagu,
                ability_name: String::new(),
                mastery: 5,
                charge: 0.0,
                combo_step: 0,
                channel_ticks: 0,
                tick: 40, // even tick triggers MagmaWalk
                eye_pos: DVec3::new(0.0, 65.0, 0.0),
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
            in_lava: true,
        };

        let tick_res = module.on_tick(&mut ctx);
        // Lava energy bonus: 50.0 + 6.0 / 20.0 = 50.3
        assert!((ctx.player.energy - 50.3).abs() < 1e-4);
        assert!(!tick_res.transformed_blocks.is_empty());
        assert!(
            tick_res
                .transformed_blocks
                .iter()
                .all(|(_, block)| block == "minecraft:obsidian")
        );
    }
}
