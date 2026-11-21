//! Ability execution engine, charge/channel/toggle/combo state machines, aim
//! sanitization, cast budgets, and technique heat.
//!
//! Mirrors `core/ability/{Ability,AbilityStats,AbilityManager,CastType,CastResult,AimHelper,AbilityContext}.java`,
//! `core/form/AbilityModifiers.java`, `core/world/CastBudget.java`, and `core/world/TechniqueHeat.java`.

use std::collections::BTreeMap;

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::rules::{CoreConfig, Mastery, PlayerFruitData, PlayerUuid};

/// Maximum extra ticks a player may keep holding a fully-charged ability (`AbilityManager.MAX_HOLD_EXTRA`).
pub const MAX_HOLD_EXTRA_TICKS: i32 = 60;

/// Movement speed attribute modifier while charging (`AbilityManager.CHARGE_SLOW`).
pub const CHARGE_SLOW_MODIFIER: f64 = -0.45;

/// Cooldown pruning cadence (`AbilityManager.PRUNE_INTERVAL`).
pub const COOLDOWN_PRUNE_INTERVAL_TICKS: i64 = 100;

/// Aim sanitization constants (`AimHelper.java`).
pub const AIM_MAX_RANGE: f64 = 160.0;
pub const AIM_MAX_ANGLE_DEGREES: f64 = 35.0;
pub const AIM_STALE_TICKS: i64 = 25;
pub const AIM_MIN_DISTANCE: f64 = 1.0;

/// Ability cast archetype (`com.devilfruits.core.ability.CastType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CastType {
    Instant,
    Charged,
    Held,
    Toggle,
    Combo,
    FormSwitch,
}

/// Ability activation outcome (`com.devilfruits.core.ability.CastResult`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CastResult {
    Success,
    Charging,
    Channelling,
    Toggled,
    ComboStep,
    NoFruit,
    NotInForm,
    Suppressed,
    Disabled,
    Locked,
    Cooldown,
    NoEnergy,
    Requirement,
    Busy,
}

impl CastResult {
    pub const fn is_ok(self) -> bool {
        matches!(
            self,
            Self::Success | Self::Charging | Self::Channelling | Self::Toggled | Self::ComboStep
        )
    }
}

/// Qualified ability identifier (`fruit_id` + `ability_name`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbilityId {
    pub fruit: FruitId,
    pub name: String,
}

impl AbilityId {
    pub fn new(fruit: FruitId, name: impl Into<String>) -> Self {
        Self {
            fruit,
            name: name.into(),
        }
    }

    pub fn full_key(&self) -> String {
        format!("{}:{}", self.fruit.path(), self.name)
    }
}

/// Tunable stat bundle for an ability (`com.devilfruits.core.ability.AbilityStats`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilityStats {
    pub enabled: bool,
    pub energy_cost: f32,
    pub energy_per_second: f32,
    pub cooldown_ticks: i32,
    pub unlock_mastery: u32,
    pub damage: f32,
    pub range: f32,
    pub knockback: f32,
    pub max_charge_ticks: i32,
    pub max_channel_ticks: i32,
    pub min_channel_ticks: i32,
    pub max_combo_steps: u32,
    pub combo_window_ticks: i32,
    pub mastery_xp_override: Option<f32>,
}

impl Default for AbilityStats {
    fn default() -> Self {
        Self {
            enabled: true,
            energy_cost: 20.0,
            energy_per_second: 0.0,
            cooldown_ticks: 40,
            unlock_mastery: 1,
            damage: 6.0,
            range: 8.0,
            knockback: 0.5,
            max_charge_ticks: 40,
            max_channel_ticks: 100,
            min_channel_ticks: 0,
            max_combo_steps: 3,
            combo_window_ticks: 16,
            mastery_xp_override: None,
        }
    }
}

impl AbilityStats {
    /// Default mastery XP grant matching `Ability.masteryXp()`: `3.0 + energy_cost * 0.2`.
    pub fn mastery_xp(&self) -> f32 {
        self.mastery_xp_override
            .unwrap_or_else(|| 3.0 + self.energy_cost * 0.2)
    }

    /// Scaled damage using `Mastery.damage_multiplier` + form modifier.
    pub fn scaled_damage(&self, mastery: u32, cfg: &CoreConfig, mods: &AbilityModifiers) -> f32 {
        self.damage * Mastery::damage_multiplier(mastery, cfg) * mods.damage_multiplier
    }

    /// Scaled range using `Mastery.range_multiplier` + form modifier.
    pub fn scaled_range(&self, mastery: u32, cfg: &CoreConfig, mods: &AbilityModifiers) -> f32 {
        self.range * Mastery::range_multiplier(mastery, cfg) * mods.range_multiplier
    }

    /// Scaled cooldown using `Mastery.scale_cooldown` + form modifier.
    pub fn scaled_cooldown(&self, mastery: u32, cfg: &CoreConfig, mods: &AbilityModifiers) -> i32 {
        let base = ((self.cooldown_ticks as f32) * mods.cooldown_multiplier).round() as i32;
        Mastery::scale_cooldown(base, mastery, cfg)
    }

    /// Scaled energy cost with form modifier and free-ability flag.
    pub fn scaled_energy_cost(&self, mods: &AbilityModifiers) -> f32 {
        if mods.free_abilities {
            0.0
        } else {
            (self.energy_cost * mods.energy_multiplier).max(0.0)
        }
    }

    /// Scaled energy per tick for Held/Toggle abilities.
    pub fn energy_per_tick(&self, mods: &AbilityModifiers) -> f32 {
        if mods.free_abilities {
            0.0
        } else {
            (self.energy_per_second * mods.energy_multiplier / 20.0).max(0.0)
        }
    }

    /// Charge-scaled energy cost (`energy_cost * (0.5 + 0.5 * charge)` by default, customizable via closure).
    pub fn charge_energy_cost(&self, charge: f32, mods: &AbilityModifiers) -> f32 {
        let c = charge.clamp(0.0, 1.0);
        self.scaled_energy_cost(mods) * (0.5 + 0.5 * c)
    }
}

/// Form-supplied ability stat multipliers (`com.devilfruits.core.form.AbilityModifiers`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilityModifiers {
    pub damage_multiplier: f32,
    pub range_multiplier: f32,
    pub cooldown_multiplier: f32,
    pub energy_multiplier: f32,
    pub free_abilities: bool,
}

impl Default for AbilityModifiers {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl AbilityModifiers {
    pub const IDENTITY: Self = Self {
        damage_multiplier: 1.0,
        range_multiplier: 1.0,
        cooldown_multiplier: 1.0,
        energy_multiplier: 1.0,
        free_abilities: false,
    };
}

/// Static declaration of an ability (`Ability.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilityDef {
    pub id: AbilityId,
    pub display_name: String,
    pub cast_type: CastType,
    pub allowed_forms: Vec<String>,
    pub stats: AbilityStats,
    pub is_ultimate: bool,
    pub stops_movement_while_charging: bool,
    pub allows_cancel_on_hurt: bool,
}

impl AbilityDef {
    pub fn allowed_in_form(&self, form: &str) -> bool {
        self.allowed_forms.is_empty() || self.allowed_forms.iter().any(|f| f == form)
    }
}

/// Runtime context passed when executing an ability (`AbilityContext.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityContext {
    pub caster: PlayerUuid,
    pub fruit: FruitId,
    pub ability_name: String,
    pub mastery: u32,
    pub charge: f32,
    pub combo_step: u32,
    pub channel_ticks: i32,
    pub tick: i64,
    pub eye_pos: DVec3,
    pub look_dir: DVec3,
    pub aim_point: Option<DVec3>,
    pub modifiers: AbilityModifiers,
}

/// Client-reported crosshair aim point with staleness & cone validation (`AimHelper.java`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoredAim {
    pub point: DVec3,
    pub received_tick: i64,
}

/// Server-side aim validator (`com.devilfruits.core.ability.AimHelper`).
#[derive(Clone, Debug, Default)]
pub struct AimHelper {
    points: BTreeMap<PlayerUuid, StoredAim>,
}

impl AimHelper {
    pub fn record(&mut self, player: PlayerUuid, point: DVec3, now: i64) {
        self.points.insert(
            player,
            StoredAim {
                point,
                received_tick: now,
            },
        );
    }

    pub fn clear(&mut self, player: PlayerUuid) {
        self.points.remove(&player);
    }

    /// Validates stored aim against eye position and look vector matching `AimHelper.resolve`:
    /// - Rejects if older than `AIM_STALE_TICKS` (25)
    /// - Rejects if distance `< AIM_MIN_DISTANCE` (1.0) or `> AIM_MAX_RANGE` (160.0)
    /// - Rejects if angular deviation from `look_dir` exceeds `AIM_MAX_ANGLE_DEGREES` (35°).
    pub fn resolve(
        &self,
        player: PlayerUuid,
        eye_pos: DVec3,
        look_dir: DVec3,
        now: i64,
    ) -> Option<DVec3> {
        let stored = self.points.get(&player)?;
        if now.saturating_sub(stored.received_tick) > AIM_STALE_TICKS {
            return None;
        }
        let delta = stored.point - eye_pos;
        let dist = delta.length();
        if !dist.is_finite() || dist < AIM_MIN_DISTANCE || dist > AIM_MAX_RANGE {
            return None;
        }
        let look = look_dir.normalize_or_zero();
        if look.length_squared() < 1e-8 {
            return None;
        }
        let dir = delta / dist;
        let min_cos = AIM_MAX_ANGLE_DEGREES.to_radians().cos();
        if dir.dot(look) < min_cos {
            return None;
        }
        Some(stored.point)
    }

    /// Returns a normalized aim direction (using validated aim point if available, else `look_dir`).
    pub fn direction(&self, player: PlayerUuid, eye_pos: DVec3, look_dir: DVec3, now: i64) -> DVec3 {
        if let Some(pt) = self.resolve(player, eye_pos, look_dir, now) {
            (pt - eye_pos).normalize_or_zero()
        } else {
            look_dir.normalize_or_zero()
        }
    }
}

/// Active charging state for a player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveCharge {
    pub ability_name: String,
    pub started_tick: i64,
    pub max_charge_ticks: i32,
}

impl ActiveCharge {
    pub fn elapsed(&self, now: i64) -> i32 {
        (now - self.started_tick).max(0) as i32
    }

    pub fn progress(&self, now: i64) -> f32 {
        if self.max_charge_ticks <= 0 {
            return 1.0;
        }
        (self.elapsed(now) as f32 / self.max_charge_ticks as f32).clamp(0.0, 1.0)
    }

    pub fn is_overheld(&self, now: i64) -> bool {
        self.elapsed(now) >= self.max_charge_ticks + MAX_HOLD_EXTRA_TICKS
    }
}

/// Active channeled (Held) ability state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveChannel {
    pub ability_name: String,
    pub started_tick: i64,
    pub max_channel_ticks: i32,
    pub min_channel_ticks: i32,
}

impl ActiveChannel {
    pub fn elapsed(&self, now: i64) -> i32 {
        (now - self.started_tick).max(0) as i32
    }
}

/// Active combo chain tracker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComboTracker {
    pub ability_name: String,
    pub step: u32,
    pub expires_tick: i64,
}

/// Per-player mutable ability runtime (`AbilityManager.java`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilityRuntime {
    pub charging: Option<ActiveCharge>,
    pub channelling: Option<ActiveChannel>,
    pub active_toggles: Vec<String>,
    pub combo: Option<ComboTracker>,
    pub busy_until_tick: i64,
}

/// Events produced by `AbilityManager` for fruit modules and networking.
#[derive(Clone, Debug, PartialEq)]
pub enum AbilityEvent {
    CastInstant {
        ability: String,
        mastery: u32,
    },
    ChargeStarted {
        ability: String,
        max_ticks: i32,
    },
    ChargeReleased {
        ability: String,
        charge: f32,
        mastery: u32,
    },
    ChargeCancelled {
        ability: String,
    },
    ChannelStarted {
        ability: String,
    },
    ChannelTick {
        ability: String,
        elapsed_ticks: i32,
    },
    ChannelEnded {
        ability: String,
        elapsed_ticks: i32,
    },
    Toggled {
        ability: String,
        enabled: bool,
    },
    ToggleTick {
        ability: String,
    },
    ComboStep {
        ability: String,
        step: u32,
        is_finisher: bool,
    },
    FormSwitchRequested {
        ability: String,
    },
}

/// High-level ability controller (`com.devilfruits.core.ability.AbilityManager`).
pub struct AbilityManager;

impl AbilityManager {
    /// Exact 10-iteration binary search for highest affordable charge fraction in `0.0..=charge`
    /// (`AbilityManager.affordableCharge`).
    pub fn affordable_charge<F>(
        available_energy: f32,
        target_charge: f32,
        creative_free: bool,
        cost_fn: F,
    ) -> f32
    where
        F: Fn(f32) -> f32,
    {
        let target = target_charge.clamp(0.0, 1.0);
        if creative_free || cost_fn(target) <= available_energy + 1e-4 {
            return target;
        }
        if cost_fn(0.0) > available_energy + 1e-4 {
            return -1.0;
        }
        let mut lo = 0.0f32;
        let mut hi = target;
        for _ in 0..10 {
            let mid = 0.5 * (lo + hi);
            if cost_fn(mid) <= available_energy + 1e-4 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// Attempts to begin/execute an ability press (`AbilityInputPayload` press).
    #[allow(clippy::too_many_arguments)]
    pub fn on_press(
        player: &mut PlayerFruitData,
        runtime: &mut AbilityRuntime,
        def: &AbilityDef,
        now: i64,
        suppressed: bool,
        blocked: bool,
        creative: bool,
        cfg: &CoreConfig,
        mods: &AbilityModifiers,
    ) -> (CastResult, Vec<AbilityEvent>) {
        let mut events = Vec::new();
        let Some(fruit) = player.fruit_id else {
            return (CastResult::NoFruit, events);
        };
        if fruit != def.id.fruit {
            return (CastResult::NoFruit, events);
        }
        if !def.stats.enabled {
            return (CastResult::Disabled, events);
        }
        if suppressed {
            return (CastResult::Suppressed, events);
        }
        if blocked || now < runtime.busy_until_tick || runtime.charging.is_some() || runtime.channelling.is_some() {
            return (CastResult::Busy, events);
        }
        if !def.allowed_in_form(&player.active_form) {
            return (CastResult::NotInForm, events);
        }
        if player.mastery < def.stats.unlock_mastery {
            return (CastResult::Locked, events);
        }

        // Toggle off bypasses cooldown and energy cost.
        if def.cast_type == CastType::Toggle {
            if let Some(idx) = runtime
                .active_toggles
                .iter()
                .position(|n| n == &def.id.name)
            {
                runtime.active_toggles.remove(idx);
                let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                player.set_cooldown(&def.id.name, now, cd);
                events.push(AbilityEvent::Toggled {
                    ability: def.id.name.clone(),
                    enabled: false,
                });
                return (CastResult::Toggled, events);
            }
        }

        if player.cooldown_remaining(&def.id.name, now) > 0 {
            return (CastResult::Cooldown, events);
        }

        match def.cast_type {
            CastType::Instant => {
                let cost = def.stats.scaled_energy_cost(mods);
                if !player.spend_energy(cost, creative, cfg) {
                    return (CastResult::NoEnergy, events);
                }
                let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                player.set_cooldown(&def.id.name, now, cd);
                player.mark_combat(now);
                Mastery::award_xp(player, def.stats.mastery_xp(), cfg);
                events.push(AbilityEvent::CastInstant {
                    ability: def.id.name.clone(),
                    mastery: player.mastery,
                });
                (CastResult::Success, events)
            }
            CastType::Charged => {
                let min_cost = def.stats.charge_energy_cost(0.0, mods);
                if !(creative && cfg.creative_free_energy) && player.energy + 1e-4 < min_cost {
                    return (CastResult::NoEnergy, events);
                }
                runtime.charging = Some(ActiveCharge {
                    ability_name: def.id.name.clone(),
                    started_tick: now,
                    max_charge_ticks: def.stats.max_charge_ticks.max(1),
                });
                events.push(AbilityEvent::ChargeStarted {
                    ability: def.id.name.clone(),
                    max_ticks: def.stats.max_charge_ticks.max(1),
                });
                (CastResult::Charging, events)
            }
            CastType::Held => {
                let cost = def.stats.scaled_energy_cost(mods);
                if !player.spend_energy(cost, creative, cfg) {
                    return (CastResult::NoEnergy, events);
                }
                runtime.channelling = Some(ActiveChannel {
                    ability_name: def.id.name.clone(),
                    started_tick: now,
                    max_channel_ticks: def.stats.max_channel_ticks.max(1),
                    min_channel_ticks: def.stats.min_channel_ticks.max(0),
                });
                player.mark_combat(now);
                events.push(AbilityEvent::ChannelStarted {
                    ability: def.id.name.clone(),
                });
                (CastResult::Channelling, events)
            }
            CastType::Toggle => {
                let cost = def.stats.scaled_energy_cost(mods);
                if !player.spend_energy(cost, creative, cfg) {
                    return (CastResult::NoEnergy, events);
                }
                runtime.active_toggles.push(def.id.name.clone());
                player.mark_combat(now);
                events.push(AbilityEvent::Toggled {
                    ability: def.id.name.clone(),
                    enabled: true,
                });
                (CastResult::Toggled, events)
            }
            CastType::Combo => {
                let cost = def.stats.scaled_energy_cost(mods);
                if !player.spend_energy(cost, creative, cfg) {
                    return (CastResult::NoEnergy, events);
                }
                let next_step = match &runtime.combo {
                    Some(c) if c.ability_name == def.id.name && now <= c.expires_tick => c.step + 1,
                    _ => 1,
                };
                let max_steps = def.stats.max_combo_steps.max(1);
                let is_finisher = next_step >= max_steps;
                if is_finisher {
                    runtime.combo = None;
                    let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                    player.set_cooldown(&def.id.name, now, cd);
                } else {
                    runtime.combo = Some(ComboTracker {
                        ability_name: def.id.name.clone(),
                        step: next_step,
                        expires_tick: now + def.stats.combo_window_ticks.max(1) as i64,
                    });
                }
                player.mark_combat(now);
                Mastery::award_xp(player, def.stats.mastery_xp() * 0.5, cfg);
                events.push(AbilityEvent::ComboStep {
                    ability: def.id.name.clone(),
                    step: next_step,
                    is_finisher,
                });
                (CastResult::ComboStep, events)
            }
            CastType::FormSwitch => {
                events.push(AbilityEvent::FormSwitchRequested {
                    ability: def.id.name.clone(),
                });
                (CastResult::Success, events)
            }
        }
    }

    /// Handles button release for Charged and Held abilities.
    #[allow(clippy::too_many_arguments)]
    pub fn on_release(
        player: &mut PlayerFruitData,
        runtime: &mut AbilityRuntime,
        def: &AbilityDef,
        now: i64,
        creative: bool,
        cfg: &CoreConfig,
        mods: &AbilityModifiers,
    ) -> (CastResult, Vec<AbilityEvent>) {
        let mut events = Vec::new();
        if let Some(charge) = runtime.charging.take() {
            if charge.ability_name != def.id.name {
                runtime.charging = Some(charge);
                return (CastResult::Busy, events);
            }
            let raw = charge.progress(now);
            let affordable = Self::affordable_charge(
                player.energy,
                raw,
                creative && cfg.creative_free_energy,
                |c| def.stats.charge_energy_cost(c, mods),
            );
            if affordable < 0.0 {
                events.push(AbilityEvent::ChargeCancelled {
                    ability: def.id.name.clone(),
                });
                return (CastResult::NoEnergy, events);
            }
            let cost = def.stats.charge_energy_cost(affordable, mods);
            player.spend_energy(cost, creative, cfg);
            let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
            player.set_cooldown(&def.id.name, now, cd);
            player.mark_combat(now);
            Mastery::award_xp(player, def.stats.mastery_xp() * (0.5 + 0.5 * affordable), cfg);
            events.push(AbilityEvent::ChargeReleased {
                ability: def.id.name.clone(),
                charge: affordable,
                mastery: player.mastery,
            });
            return (CastResult::Success, events);
        }

        if let Some(chan) = runtime.channelling.clone() {
            if chan.ability_name == def.id.name {
                let elapsed = chan.elapsed(now);
                if elapsed < chan.min_channel_ticks {
                    return (CastResult::Channelling, events);
                }
                runtime.channelling = None;
                let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                player.set_cooldown(&def.id.name, now, cd);
                Mastery::award_xp(player, def.stats.mastery_xp(), cfg);
                events.push(AbilityEvent::ChannelEnded {
                    ability: def.id.name.clone(),
                    elapsed_ticks: elapsed,
                });
                return (CastResult::Success, events);
            }
        }

        (CastResult::Busy, events)
    }

    /// Advances active charging, channelling, toggles, combo windows, and cooldown pruning.
    #[allow(clippy::too_many_arguments)]
    pub fn tick<F>(
        player: &mut PlayerFruitData,
        runtime: &mut AbilityRuntime,
        now: i64,
        suppressed: bool,
        creative: bool,
        cfg: &CoreConfig,
        mods: &AbilityModifiers,
        lookup: F,
    ) -> Vec<AbilityEvent>
    where
        F: Fn(&str) -> Option<AbilityDef>,
    {
        let mut events = Vec::new();
        if (now % COOLDOWN_PRUNE_INTERVAL_TICKS) == 0 {
            player.prune_cooldowns(now);
        }

        if suppressed {
            Self::interrupt(player, runtime, now, cfg, mods, &lookup, &mut events);
            return events;
        }

        // Auto-release overheld charge after max_charge + MAX_HOLD_EXTRA_TICKS (60).
        if let Some(charge) = &runtime.charging {
            if charge.is_overheld(now) {
                if let Some(def) = lookup(&charge.ability_name) {
                    let (_, mut rel) = Self::on_release(player, runtime, &def, now, creative, cfg, mods);
                    events.append(&mut rel);
                } else {
                    runtime.charging = None;
                }
            }
        }

        // Advance active channel.
        if let Some(chan) = runtime.channelling.clone() {
            let elapsed = chan.elapsed(now);
            let mut end_now = elapsed >= chan.max_channel_ticks;
            if let Some(def) = lookup(&chan.ability_name) {
                let drain = def.stats.energy_per_tick(mods);
                if drain > 0.0 && !player.spend_energy(drain, creative, cfg) {
                    end_now = true;
                }
                if end_now {
                    runtime.channelling = None;
                    let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                    player.set_cooldown(&def.id.name, now, cd);
                    Mastery::award_xp(player, def.stats.mastery_xp(), cfg);
                    events.push(AbilityEvent::ChannelEnded {
                        ability: def.id.name,
                        elapsed_ticks: elapsed,
                    });
                } else {
                    events.push(AbilityEvent::ChannelTick {
                        ability: def.id.name,
                        elapsed_ticks: elapsed,
                    });
                }
            } else {
                runtime.channelling = None;
            }
        }

        // Advance active toggles.
        let mut kept = Vec::with_capacity(runtime.active_toggles.len());
        let toggles = std::mem::take(&mut runtime.active_toggles);
        for name in toggles {
            if let Some(def) = lookup(&name) {
                let drain = def.stats.energy_per_tick(mods);
                if drain > 0.0 && !player.spend_energy(drain, creative, cfg) {
                    let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                    player.set_cooldown(&def.id.name, now, cd);
                    events.push(AbilityEvent::Toggled {
                        ability: name,
                        enabled: false,
                    });
                } else {
                    events.push(AbilityEvent::ToggleTick {
                        ability: name.clone(),
                    });
                    kept.push(name);
                }
            }
        }
        runtime.active_toggles = kept;

        // Expire stale combo window and apply cooldown on timeout.
        if let Some(combo) = &runtime.combo {
            if now > combo.expires_tick {
                let name = combo.ability_name.clone();
                runtime.combo = None;
                if let Some(def) = lookup(&name) {
                    let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                    player.set_cooldown(&name, now, cd);
                }
            }
        }

        events
    }

    /// Cancels charging, channelling, and active toggles (e.g. stun, seastone, death).
    pub fn interrupt<F>(
        player: &mut PlayerFruitData,
        runtime: &mut AbilityRuntime,
        now: i64,
        cfg: &CoreConfig,
        mods: &AbilityModifiers,
        lookup: &F,
        events: &mut Vec<AbilityEvent>,
    ) where
        F: Fn(&str) -> Option<AbilityDef>,
    {
        if let Some(charge) = runtime.charging.take() {
            events.push(AbilityEvent::ChargeCancelled {
                ability: charge.ability_name,
            });
        }
        if let Some(chan) = runtime.channelling.take() {
            let elapsed = chan.elapsed(now);
            if let Some(def) = lookup(&chan.ability_name) {
                let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                player.set_cooldown(&def.id.name, now, cd);
            }
            events.push(AbilityEvent::ChannelEnded {
                ability: chan.ability_name,
                elapsed_ticks: elapsed,
            });
        }
        for name in runtime.active_toggles.drain(..) {
            if let Some(def) = lookup(&name) {
                let cd = def.stats.scaled_cooldown(player.mastery, cfg, mods);
                player.set_cooldown(&def.id.name, now, cd);
            }
            events.push(AbilityEvent::Toggled {
                ability: name,
                enabled: false,
            });
        }
        runtime.combo = None;
    }
}

/// Per-cast terrain destruction & debris budget (`com.devilfruits.core.world.CastBudget`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastBudget {
    pub blocks_remaining: usize,
    pub debris_remaining: usize,
}

impl CastBudget {
    pub fn from_config(cfg: &CoreConfig) -> Self {
        Self {
            blocks_remaining: cfg.max_blocks_per_cast,
            debris_remaining: cfg.max_debris_entities_per_cast,
        }
    }

    pub const fn new(max_blocks: usize, max_debris: usize) -> Self {
        Self {
            blocks_remaining: max_blocks,
            debris_remaining: max_debris,
        }
    }

    pub fn try_consume_block(&mut self) -> bool {
        if self.blocks_remaining == 0 {
            false
        } else {
            self.blocks_remaining -= 1;
            true
        }
    }

    pub fn try_consume_debris(&mut self) -> bool {
        if self.debris_remaining == 0 {
            false
        } else {
            self.debris_remaining -= 1;
            true
        }
    }
}

/// Per-player technique heat / spam-damping tracker (`com.devilfruits.core.world.TechniqueHeat`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TechniqueHeat {
    pub entries: BTreeMap<String, (f32, i64)>,
}

impl TechniqueHeat {
    /// Decay per second (default 0.25 heat/sec => 0.0125/tick).
    pub const DECAY_PER_TICK: f32 = 0.0125;

    /// Returns current decayed heat in `0.0..=1.0`.
    pub fn current(&self, key: &str, now: i64) -> f32 {
        let Some(&(val, last_tick)) = self.entries.get(key) else {
            return 0.0;
        };
        let dt = (now - last_tick).max(0) as f32;
        (val - dt * Self::DECAY_PER_TICK).max(0.0)
    }

    /// Adds heat for a cast and returns the updated value clamped to `0.0..=1.0`.
    pub fn record(&mut self, key: impl Into<String>, delta: f32, now: i64) -> f32 {
        let k = key.into();
        let next = (self.current(&k, now) + delta).clamp(0.0, 1.0);
        self.entries.insert(k, (next, now));
        next
    }

    /// Destruction scale factor `[0.25..=1.0]` so rapid repeated casts taper terrain damage cleanly.
    pub fn destruction_scale(&self, key: &str, now: i64) -> f32 {
        (1.0 - 0.75 * self.current(key, now)).clamp(0.25, 1.0)
    }
}
