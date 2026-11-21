//! Suke Suke no Mi (Clear-Clear Fruit) — 1:1 port of `com.devilfruits.fruits.suke_suke`.
//!
//! Includes `skatting` (`invisibility`), `stealth_ambush` (`hidden_weapon`),
//! `blind_touch` (`group_invisibility`), `clear_world` (`ship_invisibility`),
//! `Cloak` solo & shared energy drain scaling (`60s..=300s`), `ShipCloak` duration
//! scaling (`30s..=60s`), and full stealth flags (`StealthFlags` +
//! `clear_invisible`, `hide_hand`, `suppress_footstep_vibrations`,
//! `suppress_sprint_particles`, `suppress_hurt_tint`) in `body_modifiers`.

use glam::DVec3;
use horizon_world::grandline_hooks::{
    CLEAR_FLAG_HIDDEN_ITEMS, CLEAR_FLAG_INVISIBLE, CLEAR_FLAG_SHIP, CLEAR_NOBODY_UUID,
    EntityBodyModifiers, SUKE_DEFAULT_GROUP_RADIUS, SUKE_DEFAULT_MOB_VISIBILITY,
    SUKE_DEFAULT_MUFFLE_STEPS, SUKE_DEFAULT_SHIP_SOUND_FACTOR, SUKE_DEFAULT_SPECTATORS_SEE,
    StealthFlags,
};

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Java constants (`SukeSuke.java`, `Cloak.java`, `ShipCloak.java`, `Clear.java`)
// ---------------------------------------------------------------------------

pub const SUKE_SOLO_SECONDS_MASTERY_1: f64 = 60.0;
pub const SUKE_SOLO_SECONDS_MASTERY_10: f64 = 300.0;
pub const SUKE_MOB_VISIBILITY: f64 = SUKE_DEFAULT_MOB_VISIBILITY; // 0.07
pub const SUKE_MUFFLE_STEPS: f32 = SUKE_DEFAULT_MUFFLE_STEPS; // 0.5
pub const SUKE_GROUP_RADIUS: f64 = SUKE_DEFAULT_GROUP_RADIUS; // 3.0
pub const SUKE_SHIP_SECONDS_MASTERY_8: f64 = 30.0;
pub const SUKE_SHIP_SECONDS_MASTERY_10: f64 = 60.0;
pub const SUKE_SHIP_REACH: f64 = 16.0;
pub const SUKE_SHIP_SOUND_FACTOR: f32 = SUKE_DEFAULT_SHIP_SOUND_FACTOR; // 0.35
pub const SUKE_HIDE_NAME_TAG: bool = true;
pub const SUKE_BREAK_ON_ATTACK: bool = false;
pub const SUKE_SPECTATORS_SEE: bool = SUKE_DEFAULT_SPECTATORS_SEE; // true

/// `CloakAbility` (`invisibility` / `skatting`) constants.
pub const INVISIBILITY_ENERGY: f32 = 0.0;
pub const INVISIBILITY_COOLDOWN_TICKS: i32 = 40;
pub const INVISIBILITY_UNLOCK_MASTERY: u32 = 1;
pub const INVISIBILITY_MASTERY_XP: f32 = 3.0;

/// `HiddenWeaponAbility` (`hidden_weapon` / `stealth_ambush`) constants.
pub const HIDDEN_WEAPON_ENERGY: f32 = 0.0;
pub const HIDDEN_WEAPON_DRAIN_PER_SECOND: f32 = 0.4;
pub const HIDDEN_WEAPON_COOLDOWN_TICKS: i32 = 20;
pub const HIDDEN_WEAPON_UNLOCK_MASTERY: u32 = 2;
pub const STEALTH_AMBUSH_DAMAGE: f32 = 14.0;
pub const STEALTH_AMBUSH_RANGE: f32 = 5.0;
pub const STEALTH_AMBUSH_KNOCKBACK: f32 = 0.9;

/// `CloakAbility` (`group_invisibility` / `blind_touch`) constants.
pub const GROUP_INVISIBILITY_ENERGY: f32 = 0.0;
pub const GROUP_INVISIBILITY_COOLDOWN_TICKS: i32 = 100;
pub const GROUP_INVISIBILITY_UNLOCK_MASTERY: u32 = 4;
pub const GROUP_INVISIBILITY_MASTERY_XP: f32 = 6.0;

/// `ShipInvisibilityAbility` (`ship_invisibility` / `clear_world`) constants.
pub const SHIP_INVISIBILITY_ENERGY: f32 = 120.0;
pub const SHIP_INVISIBILITY_COOLDOWN_TICKS: i32 = 3600;
pub const SHIP_INVISIBILITY_UNLOCK_MASTERY: u32 = 8;
pub const SHIP_INVISIBILITY_RANGE: f32 = 16.0;

/// Synced form flags used to track Suke-Suke clear state (`Clear.INVISIBLE`, `Clear.HIDDEN_ITEMS`, `Clear.SHIP`).
pub const SUKE_FLAG_INVISIBLE: u32 = form_flags::fruit_bit(0);
pub const SUKE_FLAG_HIDDEN_ITEMS: u32 = form_flags::fruit_bit(1);
pub const SUKE_FLAG_GROUP_CLOAK: u32 = form_flags::fruit_bit(2);
pub const SUKE_FLAG_SHIP_CLOAK: u32 = form_flags::fruit_bit(3);

/// Canonical Suke-Suke VFX identifiers (`SukeFx.java`).
pub mod suke_fx {
    pub const VANISH: &str = "devilfruits:suke_suke/vanish";
    pub const APPEAR: &str = "devilfruits:suke_suke/appear";
    pub const GROUP_CAST: &str = "devilfruits:suke_suke/group_cast";
    pub const SHIP_CAST: &str = "devilfruits:suke_suke/ship_cast";
}

/// Complete Suke-Suke stealth descriptor exposing all render, audio, vibration, and particle suppression flags.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SukeStealthProfile {
    pub clear_invisible: bool,
    pub hide_hand: bool,
    pub suppress_footstep_vibrations: bool,
    pub suppress_sprint_particles: bool,
    pub suppress_hurt_tint: bool,
    pub ship_cloaked: bool,
    pub group_cloaked: bool,
    pub clear_bits: u32,
}

impl SukeStealthProfile {
    pub fn from_runtime(form_runtime: &FormRuntime, suppressed: bool) -> Self {
        if suppressed {
            return Self::default();
        }
        let invisible = form_runtime.has_flag(SUKE_FLAG_INVISIBLE)
            || form_runtime.has_flag(SUKE_FLAG_GROUP_CLOAK)
            || form_runtime.has_flag(SUKE_FLAG_SHIP_CLOAK);
        let hidden_items = form_runtime.has_flag(SUKE_FLAG_HIDDEN_ITEMS) || invisible;
        let ship_cloaked = form_runtime.has_flag(SUKE_FLAG_SHIP_CLOAK);
        let group_cloaked = form_runtime.has_flag(SUKE_FLAG_GROUP_CLOAK);
        let clear_bits = (if invisible { CLEAR_FLAG_INVISIBLE } else { 0 })
            | (if hidden_items {
                CLEAR_FLAG_HIDDEN_ITEMS
            } else {
                0
            })
            | (if ship_cloaked { CLEAR_FLAG_SHIP } else { 0 });

        Self {
            clear_invisible: invisible,
            hide_hand: hidden_items,
            suppress_footstep_vibrations: invisible,
            suppress_sprint_particles: invisible,
            suppress_hurt_tint: invisible,
            ship_cloaked,
            group_cloaked,
            clear_bits,
        }
    }
}

/// Suke Suke no Mi module (`com.devilfruits.fruits.suke_suke.SukeSuke`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SukeSukeModule;

impl SukeSukeModule {
    pub const fn new() -> Self {
        Self
    }

    /// Exact solo invisibility duration in seconds at `mastery` (`Cloak.soloSeconds`):
    /// linearly interpolates from `60.0s` at mastery 1 to `300.0s` at mastery 10.
    pub fn solo_seconds(mastery: u32) -> f64 {
        let t = (mastery.clamp(1, 10) - 1) as f64 / 9.0;
        SUKE_SOLO_SECONDS_MASTERY_1
            + (SUKE_SOLO_SECONDS_MASTERY_10 - SUKE_SOLO_SECONDS_MASTERY_1) * t
    }

    /// Exact energy drained per server tick while cloaked (`Cloak.drainPerTick`):
    /// `max_energy / (solo_seconds(mastery) * 20.0) * max(1, headcount)`.
    pub fn cloak_drain_per_tick(max_energy: f32, mastery: u32, headcount: usize) -> f32 {
        let secs = Self::solo_seconds(mastery);
        ((max_energy as f64) / (secs * 20.0) * (headcount.max(1) as f64)) as f32
    }

    /// Exact ship invisibility duration in seconds at `mastery` (`ShipCloak.seconds`):
    /// linearly interpolates from `30.0s` at mastery 8 to `60.0s` at mastery 10.
    pub fn ship_seconds(mastery: u32) -> f64 {
        let clamped = mastery.clamp(8, 10) - 8;
        let t = (clamped as f64) / 2.0;
        SUKE_SHIP_SECONDS_MASTERY_8
            + (SUKE_SHIP_SECONDS_MASTERY_10 - SUKE_SHIP_SECONDS_MASTERY_8) * t
    }

    /// Returns the [`SukeStealthProfile`] for a player.
    pub fn stealth_profile(form_runtime: &FormRuntime, suppressed: bool) -> SukeStealthProfile {
        SukeStealthProfile::from_runtime(form_runtime, suppressed)
    }
}

impl DevilFruitModule for SukeSukeModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::SukeSuke
    }

    fn primary_damage_type(&self) -> GrandLineDamageType {
        GrandLineDamageType::DevilFruit
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_form = vec!["base".to_string()];

        let make_invisibility = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::SukeSuke, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: INVISIBILITY_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: INVISIBILITY_COOLDOWN_TICKS,
                unlock_mastery: INVISIBILITY_UNLOCK_MASTERY,
                damage: 0.0,
                range: 1.0,
                knockback: 0.0,
                mastery_xp_override: Some(INVISIBILITY_MASTERY_XP),
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_hidden_weapon = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::SukeSuke, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: HIDDEN_WEAPON_ENERGY,
                energy_per_second: HIDDEN_WEAPON_DRAIN_PER_SECOND,
                cooldown_ticks: HIDDEN_WEAPON_COOLDOWN_TICKS,
                unlock_mastery: HIDDEN_WEAPON_UNLOCK_MASTERY,
                damage: STEALTH_AMBUSH_DAMAGE,
                range: STEALTH_AMBUSH_RANGE,
                knockback: STEALTH_AMBUSH_KNOCKBACK,
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_group = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::SukeSuke, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: GROUP_INVISIBILITY_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: GROUP_INVISIBILITY_COOLDOWN_TICKS,
                unlock_mastery: GROUP_INVISIBILITY_UNLOCK_MASTERY,
                damage: 0.0,
                range: SUKE_GROUP_RADIUS as f32,
                knockback: 0.0,
                mastery_xp_override: Some(GROUP_INVISIBILITY_MASTERY_XP),
                ..AbilityStats::default()
            },
            is_ultimate: false,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        let make_ship = |name: &str, display: &str| AbilityDef {
            id: AbilityId::new(FruitId::SukeSuke, name),
            display_name: display.into(),
            cast_type: CastType::Toggle,
            allowed_forms: base_form.clone(),
            stats: AbilityStats {
                enabled: true,
                energy_cost: SHIP_INVISIBILITY_ENERGY,
                energy_per_second: 0.0,
                cooldown_ticks: SHIP_INVISIBILITY_COOLDOWN_TICKS,
                unlock_mastery: SHIP_INVISIBILITY_UNLOCK_MASTERY,
                damage: 0.0,
                range: SHIP_INVISIBILITY_RANGE,
                knockback: 0.0,
                ..AbilityStats::default()
            },
            is_ultimate: true,
            stops_movement_while_charging: false,
            allows_cancel_on_hurt: false,
        };

        vec![
            make_invisibility("invisibility", "Invisibility (Skatting)"),
            make_invisibility("skatting", "Skatting"),
            make_hidden_weapon("hidden_weapon", "Hidden Weapon"),
            make_hidden_weapon("stealth_ambush", "Stealth Ambush"),
            make_group("group_invisibility", "Shared Invisibility"),
            make_group("blind_touch", "Blind Touch"),
            make_ship("ship_invisibility", "Ship Invisibility"),
            make_ship("clear_world", "Clear World"),
        ]
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::DevilFruit),
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
        let now = ctx.ability_ctx.tick;

        match event {
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "invisibility" || ability == "skatting" =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(SUKE_FLAG_INVISIBLE),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(SUKE_FLAG_INVISIBLE, active);
                if active {
                    ctx.form_runtime.set_flag(SUKE_FLAG_GROUP_CLOAK, false);
                    res.animation_clip = Some("devilfruits:suke_suke/vanish".into());
                    ctx.world_fx
                        .emit_fx(suke_fx::VANISH, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                } else {
                    res.animation_clip = Some("devilfruits:suke_suke/reappear".into());
                    ctx.world_fx
                        .emit_fx(suke_fx::APPEAR, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "hidden_weapon" || ability == "stealth_ambush" =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(SUKE_FLAG_HIDDEN_ITEMS),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(SUKE_FLAG_HIDDEN_ITEMS, active);

                // If `stealth_ambush` is triggered with targets in front, strike with an invisible weapon ambush!
                if ability == "stealth_ambush" && active {
                    if let Some(def) = self.ability("stealth_ambush") {
                        let reach = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                        let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                        let kb = def.stats.knockback as f64;
                        for target in CombatTargeting::in_cone(
                            ctx.nearby_targets,
                            ctx.player_id,
                            eye,
                            look,
                            reach,
                            35.0_f64.to_radians(),
                        ) {
                            let push = look * kb + DVec3::new(0.0, 0.2, 0.0);
                            res.hits.push(AbilityHitEffect {
                                target_id: target.id,
                                damage: dmg,
                                damage_type: GrandLineDamageType::DevilFruit,
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
                    }
                }
            }
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "group_invisibility" || ability == "blind_touch" =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(SUKE_FLAG_GROUP_CLOAK),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(SUKE_FLAG_GROUP_CLOAK, active);
                ctx.form_runtime.set_flag(SUKE_FLAG_INVISIBLE, active);
                if active {
                    res.animation_clip = Some("devilfruits:suke_suke/group_cast".into());
                    ctx.world_fx.emit_fx(
                        suke_fx::GROUP_CAST,
                        0,
                        eye,
                        DVec3::ZERO,
                        SUKE_GROUP_RADIUS as f32,
                        now,
                        0,
                        64.0,
                    );
                    ctx.world_fx
                        .emit_fx(suke_fx::VANISH, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                } else {
                    res.animation_clip = Some("devilfruits:suke_suke/reappear".into());
                    ctx.world_fx
                        .emit_fx(suke_fx::APPEAR, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "ship_invisibility" || ability == "clear_world" =>
            {
                let active = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => !ctx.form_runtime.has_flag(SUKE_FLAG_SHIP_CLOAK),
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(SUKE_FLAG_SHIP_CLOAK, active);
                ctx.form_runtime.set_flag(SUKE_FLAG_INVISIBLE, active);
                if active {
                    let duration_ticks = (Self::ship_seconds(mastery) * 20.0).round() as i32;
                    res.animation_clip = Some("devilfruits:suke_suke/ship_cast".into());
                    ctx.world_fx.emit_fx(
                        suke_fx::SHIP_CAST,
                        0,
                        eye,
                        DVec3::ZERO,
                        duration_ticks as f32,
                        now,
                        0,
                        128.0,
                    );
                } else {
                    ctx.world_fx
                        .emit_fx(suke_fx::APPEAR, 0, eye, DVec3::ZERO, 1.0, now, 0, 128.0);
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        if ctx.suppressed {
            let was_invisible = ctx.form_runtime.has_flag(SUKE_FLAG_INVISIBLE)
                || ctx.form_runtime.has_flag(SUKE_FLAG_GROUP_CLOAK)
                || ctx.form_runtime.has_flag(SUKE_FLAG_SHIP_CLOAK);
            ctx.form_runtime.set_flag(SUKE_FLAG_INVISIBLE, false);
            ctx.form_runtime.set_flag(SUKE_FLAG_HIDDEN_ITEMS, false);
            ctx.form_runtime.set_flag(SUKE_FLAG_GROUP_CLOAK, false);
            ctx.form_runtime.set_flag(SUKE_FLAG_SHIP_CLOAK, false);
            if was_invisible {
                ctx.world_fx.emit_fx(
                    suke_fx::APPEAR,
                    0,
                    ctx.ability_ctx.eye_pos,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            return res;
        }

        // Cloak.tick: drain energy proportional to max_energy / (solo_seconds * 20) * headcount
        let is_group = ctx.form_runtime.has_flag(SUKE_FLAG_GROUP_CLOAK);
        let is_solo = ctx.form_runtime.has_flag(SUKE_FLAG_INVISIBLE);
        if is_solo || is_group {
            let allies_in_radius = if is_group {
                CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    ctx.ability_ctx.eye_pos,
                    SUKE_GROUP_RADIUS,
                )
                .len()
            } else {
                0
            };
            let headcount = 1 + allies_in_radius;
            let drain = Self::cloak_drain_per_tick(
                ctx.player.max_energy(),
                ctx.player.mastery,
                headcount,
            );
            if !ctx.player.spend_energy(drain, ctx.creative, ctx.config) {
                ctx.form_runtime.set_flag(SUKE_FLAG_INVISIBLE, false);
                ctx.form_runtime.set_flag(SUKE_FLAG_GROUP_CLOAK, false);
                ctx.world_fx.emit_fx(
                    suke_fx::APPEAR,
                    0,
                    ctx.ability_ctx.eye_pos,
                    DVec3::ZERO,
                    1.0,
                    ctx.ability_ctx.tick,
                    0,
                    64.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
        }

        if ctx.form_runtime.has_flag(SUKE_FLAG_HIDDEN_ITEMS)
            && !ctx.player.spend_energy(
                HIDDEN_WEAPON_DRAIN_PER_SECOND / 20.0,
                ctx.creative,
                ctx.config,
            )
        {
            ctx.form_runtime.set_flag(SUKE_FLAG_HIDDEN_ITEMS, false);
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
        let profile = Self::stealth_profile(form_runtime, suppressed);
        if profile.clear_invisible {
            mods.stealth_flags = StealthFlags::suke_clear_hidden(
                CLEAR_NOBODY_UUID,
                profile.hide_hand,
                profile.ship_cloaked,
            );
        } else if profile.hide_hand {
            mods.stealth_flags = StealthFlags {
                hidden_items: true,
                hide_eating_particles: true,
                ..StealthFlags::NONE
            };
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
    fn suke_suke_constants_and_abilities_match_java() {
        let module = SukeSukeModule::new();
        assert_eq!(module.fruit_id(), FruitId::SukeSuke);

        assert!(module.ability("skatting").is_some());
        assert!(module.ability("stealth_ambush").is_some());
        assert!(module.ability("blind_touch").is_some());
        assert!(module.ability("clear_world").is_some());
        assert!(module.ability("invisibility").is_some());
        assert!(module.ability("hidden_weapon").is_some());
        assert!(module.ability("group_invisibility").is_some());
        assert!(module.ability("ship_invisibility").is_some());

        let ship = module.ability("ship_invisibility").expect("ship_invisibility");
        assert!(ship.is_ultimate);
        assert_eq!(ship.stats.energy_cost, 120.0);
        assert_eq!(ship.stats.cooldown_ticks, 3600);
        assert_eq!(ship.stats.unlock_mastery, 8);

        assert!((SukeSukeModule::solo_seconds(1) - 60.0).abs() < 1e-6);
        assert!((SukeSukeModule::solo_seconds(10) - 300.0).abs() < 1e-6);
        assert!((SukeSukeModule::ship_seconds(8) - 30.0).abs() < 1e-6);
        assert!((SukeSukeModule::ship_seconds(10) - 60.0).abs() < 1e-6);
    }

    #[test]
    fn suke_suke_stealth_flags_and_tick_behavior() {
        let module = SukeSukeModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::SukeSuke),
            mastery: 8,
            energy: 200.0,
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
                fruit: FruitId::SukeSuke,
                ability_name: "skatting".into(),
                mastery: 8,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 60,
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

        // 1. Activate Skatting (solo invisibility)
        let skat_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "skatting".into(),
                enabled: true,
            },
        );
        assert!(skat_res
            .vfx_events
            .iter()
            .any(|e| e.fx_id == suke_fx::VANISH));

        // 2. Activate Stealth Ambush (hidden weapon + ambush hit)
        let ambush_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "stealth_ambush".into(),
                mastery: 8,
            },
        );
        assert_eq!(ambush_res.hits.len(), 1);

        // 3. Verify all stealth flags in body_modifiers and stealth_profile
        let profile = SukeSukeModule::stealth_profile(ctx.form_runtime, false);
        assert!(profile.clear_invisible);
        assert!(profile.hide_hand);
        assert!(profile.suppress_footstep_vibrations);
        assert!(profile.suppress_sprint_particles);
        assert!(profile.suppress_hurt_tint);

        let mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(mods.stealth_flags.invisible);
        assert!(mods.stealth_flags.hidden_items);
        assert!(mods.stealth_flags.hide_sprint_particles);
        assert!(mods.stealth_flags.hide_fire);
        assert!(mods.stealth_flags.hide_glow);
        assert!(mods.stealth_flags.hook_footsteps);
        assert!((mods.stealth_flags.footstep_volume_scale - 0.5).abs() < 1e-5);

        // 4. Activate Clear World (Ship Invisibility)
        let clear_world_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "clear_world".into(),
                enabled: true,
            },
        );
        assert!(clear_world_res
            .vfx_events
            .iter()
            .any(|e| e.fx_id == suke_fx::SHIP_CAST));
        let ship_mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(ship_mods.stealth_flags.ship_cloaked);

        // 5. Passive tick drains energy; suppression clears stealth immediately
        let energy_before = ctx.player.energy;
        let _ = module.on_tick(&mut ctx);
        assert!(ctx.player.energy < energy_before);

        ctx.suppressed = true;
        let sup_res = module.on_tick(&mut ctx);
        assert!(sup_res
            .vfx_events
            .iter()
            .any(|e| e.fx_id == suke_fx::APPEAR));
        let sup_mods = module.body_modifiers(ctx.player, ctx.form_runtime, true, 0.0);
        assert!(!sup_mods.stealth_flags.invisible);
    }
}
