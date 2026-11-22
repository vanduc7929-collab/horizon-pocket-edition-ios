//! Transformation forms, form stats, synced bitflags, collision hooks, upkeep,
//! charged jump physics, form dash, and safe landing.
//!
//! Mirrors `core/form/{FruitForm,FormStats,FormFlags,FormCollision,FormManager,FormUpkeep,ChargedJump,FormDash,SafeLanding}.java`.

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::ability::AbilityModifiers;
use crate::core::fruit::FruitId;
use crate::core::rules::{CoreConfig, PlayerFruitData};

/// Synced transformation state bitflags (`com.devilfruits.core.form.FormFlags`).
pub mod form_flags {
    pub const BOOSTING: u32 = 1;
    pub const FLYING: u32 = 2;
    pub const DYING: u32 = 128;

    /// Fruit-private synced flag bit `n` (`0..=15` -> `1 << (8 + n)`).
    pub const fn fruit_bit(n: u32) -> u32 {
        1u32 << (8 + (n & 15))
    }
}

/// Charged jump constants (`com.devilfruits.core.form.ChargedJump`).
pub const CHARGED_JUMP_CHARGE_TICKS: i32 = 30;
pub const CHARGED_JUMP_MIN_CHARGE: f32 = 0.2;
pub const CHARGED_JUMP_MIN_HEIGHT: f64 = 3.0;
pub const CHARGED_JUMP_COOLDOWN_TICKS: i32 = 10;

/// Safe landing constants (`com.devilfruits.core.form.SafeLanding`).
pub const SAFE_LANDING_MAX_TICKS: i32 = 1200;
pub const SAFE_LANDING_SLOW_FALLING_TICKS: i32 = 60;

/// Qualified transformation form identifier (`fruit` + `name`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FormId {
    pub fruit: FruitId,
    pub name: String,
}

impl FormId {
    pub fn new(fruit: FruitId, name: impl Into<String>) -> Self {
        Self {
            fruit,
            name: name.into(),
        }
    }
}

/// Entity attribute deltas applied while in a form (`FormStats` / `FruitForm`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FormAttributeBonuses {
    pub scale_add: f64,
    pub max_health_add: f64,
    pub attack_damage_add: f64,
    pub attack_speed_add: f64,
    pub armor_add: f64,
    pub armor_toughness_add: f64,
    pub knockback_resistance_add: f64,
    pub movement_speed_mult: f64,
    pub jump_strength_mult: f64,
    pub safe_fall_distance_add: f64,
    pub absorption_amount: f32,
}

/// Hitbox & camera collision profile (`FormCollision.java`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FormCollision {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub eye_height: Option<f32>,
    pub pass_through_leaves: bool,
    pub fire_immune: bool,
    pub camera_distance: Option<f32>,
}

/// Energy, cooldown, timing, and movement tuning for a form (`com.devilfruits.core.form.FormStats`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormStats {
    pub unlock_mastery: u32,
    pub enter_cost: f32,
    pub drain_per_second: f32,
    pub cooldown_ticks: i32,
    pub cast_ticks: i32,
    pub energy_regen_multiplier: f32,
    pub time_limit_seconds: Option<f32>,
    pub flight: bool,
    pub boostable: bool,
    pub charged_jump_max_height: Option<f64>,
    pub ability_modifiers: AbilityModifiers,
}

impl Default for FormStats {
    fn default() -> Self {
        Self {
            unlock_mastery: 1,
            enter_cost: 0.0,
            drain_per_second: 0.0,
            cooldown_ticks: 0,
            cast_ticks: 0,
            energy_regen_multiplier: 1.0,
            time_limit_seconds: None,
            flight: false,
            boostable: false,
            charged_jump_max_height: None,
            ability_modifiers: AbilityModifiers::IDENTITY,
        }
    }
}

/// Complete definition of a fruit transformation form (`FruitForm.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FruitFormDef {
    pub id: FormId,
    pub is_base: bool,
    pub stats: FormStats,
    pub collision: FormCollision,
    pub attributes: FormAttributeBonuses,
    pub abilities: Vec<String>,
    pub ultimate_ability: Option<String>,
}

impl FruitFormDef {
    /// Returns the built-in canonical forms extracted directly from Grand Line 0.4.0 Java sources.
    pub fn builtin_forms(fruit: FruitId) -> Vec<Self> {
        let meta = fruit.meta();
        let base_form = |name: &str, abilities: &[&str], ult: Option<&str>| Self {
            id: FormId::new(fruit, name),
            is_base: true,
            stats: FormStats::default(),
            collision: FormCollision::default(),
            attributes: FormAttributeBonuses::default(),
            abilities: abilities.iter().map(|s| (*s).to_string()).collect(),
            ultimate_ability: ult.map(str::to_string),
        };

        match fruit {
            FruitId::UoUoSeiryu => vec![
                base_form("human", meta.base_abilities, meta.ultimate_ability),
                Self {
                    id: FormId::new(fruit, "hybrid"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 3,
                        enter_cost: 20.0,
                        drain_per_second: 0.0,
                        cooldown_ticks: 60,
                        cast_ticks: 30,
                        energy_regen_multiplier: 0.35,
                        time_limit_seconds: None,
                        flight: false,
                        boostable: false,
                        charged_jump_max_height: Some(30.0),
                        ability_modifiers: AbilityModifiers::IDENTITY,
                    },
                    collision: FormCollision {
                        fire_immune: true,
                        ..FormCollision::default()
                    },
                    attributes: FormAttributeBonuses {
                        scale_add: 1.0,
                        max_health_add: 20.0,
                        attack_damage_add: 4.0,
                        armor_add: 6.0,
                        armor_toughness_add: 3.0,
                        knockback_resistance_add: 0.4,
                        ..FormAttributeBonuses::default()
                    },
                    abilities: vec![
                        "hybrid_form".into(),
                        "dragon_form".into(),
                        "bolo_breath".into(),
                        "tatsumaki".into(),
                        "kaifu".into(),
                    ],
                    ultimate_ability: Some("dragon_form".into()),
                },
                Self {
                    id: FormId::new(fruit, "dragon"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 6,
                        enter_cost: 0.0,
                        drain_per_second: 0.0,
                        cooldown_ticks: 100,
                        cast_ticks: 50,
                        energy_regen_multiplier: 0.0,
                        time_limit_seconds: Some(60.0),
                        flight: true,
                        boostable: true,
                        charged_jump_max_height: None,
                        ability_modifiers: AbilityModifiers {
                            free_abilities: true,
                            ..AbilityModifiers::IDENTITY
                        },
                    },
                    collision: FormCollision {
                        width: Some(2.4),
                        height: Some(2.2),
                        eye_height: Some(1.3),
                        pass_through_leaves: true,
                        fire_immune: true,
                        camera_distance: Some(14.0),
                    },
                    attributes: FormAttributeBonuses {
                        max_health_add: 60.0,
                        attack_damage_add: 8.0,
                        armor_add: 10.0,
                        armor_toughness_add: 4.0,
                        knockback_resistance_add: 1.0,
                        ..FormAttributeBonuses::default()
                    },
                    abilities: vec![
                        "dragon_form".into(),
                        "bolo_breath".into(),
                        "tatsumaki".into(),
                        "kaifu".into(),
                        "flaming_dragon".into(),
                    ],
                    ultimate_ability: Some("flaming_dragon".into()),
                },
            ],
            FruitId::DoruDoru => vec![
                base_form("base", meta.base_abilities, meta.ultimate_ability),
                Self {
                    id: FormId::new(fruit, "champion"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 10,
                        enter_cost: 60.0,
                        drain_per_second: 1.0,
                        cooldown_ticks: 600,
                        cast_ticks: 30,
                        energy_regen_multiplier: 0.5,
                        time_limit_seconds: None,
                        flight: false,
                        boostable: false,
                        charged_jump_max_height: None,
                        ability_modifiers: AbilityModifiers::IDENTITY,
                    },
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses {
                        scale_add: 1.78,
                        attack_damage_add: 7.0,
                        armor_add: 8.0,
                        armor_toughness_add: 2.0,
                        knockback_resistance_add: 0.6,
                        movement_speed_mult: 0.15,
                        absorption_amount: 60.0,
                        ..FormAttributeBonuses::default()
                    },
                    abilities: vec!["champion".into(), "lance".into(), "wall".into()],
                    ultimate_ability: Some("champion".into()),
                },
            ],
            FruitId::GomuGomu => vec![
                base_form("base", meta.base_abilities, meta.ultimate_ability),
                Self {
                    id: FormId::new(fruit, "gear_2"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 5,
                        enter_cost: 15.0,
                        drain_per_second: 1.0,
                        cooldown_ticks: 40,
                        cast_ticks: 19,
                        energy_regen_multiplier: 1.0,
                        time_limit_seconds: None,
                        flight: false,
                        boostable: false,
                        charged_jump_max_height: None,
                        ability_modifiers: AbilityModifiers::IDENTITY,
                    },
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses {
                        movement_speed_mult: 0.5,
                        jump_strength_mult: 0.35,
                        attack_speed_add: 0.15,
                        ..FormAttributeBonuses::default()
                    },
                    abilities: vec![
                        "jet_pistol".into(),
                        "jet_bazooka".into(),
                        "jet_rocket".into(),
                        "jet_axe".into(),
                        "jet_rifle".into(),
                        "jet_gatling".into(),
                    ],
                    ultimate_ability: Some("jet_gatling".into()),
                },
            ],
            FruitId::YomiYomi => vec![
                base_form("base", meta.base_abilities, meta.ultimate_ability),
                base_form("human", meta.base_abilities, meta.ultimate_ability),
                base_form("corpse", meta.base_abilities, meta.ultimate_ability),
                Self {
                    id: FormId::new(fruit, "skeleton"),
                    is_base: true,
                    stats: FormStats::default(),
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses {
                        movement_speed_mult: 0.2,
                        safe_fall_distance_add: 2.0,
                        ..FormAttributeBonuses::default()
                    },
                    abilities: meta.base_abilities.iter().map(|s| (*s).to_string()).collect(),
                    ultimate_ability: meta.ultimate_ability.map(str::to_string),
                },
                Self {
                    id: FormId::new(fruit, "soul"),
                    is_base: false,
                    stats: FormStats {
                        flight: true,
                        ..FormStats::default()
                    },
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses::default(),
                    abilities: vec!["soul_projection".into()],
                    ultimate_ability: None,
                },
                Self {
                    id: FormId::new(fruit, "soul_king"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 9,
                        enter_cost: 0.0,
                        drain_per_second: 0.0,
                        cooldown_ticks: 1800,
                        cast_ticks: 30,
                        energy_regen_multiplier: 0.0,
                        time_limit_seconds: Some(60.0),
                        flight: false,
                        boostable: false,
                        charged_jump_max_height: None,
                        ability_modifiers: AbilityModifiers {
                            free_abilities: true,
                            ..AbilityModifiers::IDENTITY
                        },
                    },
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses::default(),
                    abilities: meta.base_abilities.iter().map(|s| (*s).to_string()).collect(),
                    ultimate_ability: Some("soul_king".into()),
                },
            ],
            FruitId::HoroHoro => vec![
                base_form("base", meta.base_abilities, meta.ultimate_ability),
                Self {
                    id: FormId::new(fruit, "ghost"),
                    is_base: false,
                    stats: FormStats {
                        unlock_mastery: 1,
                        enter_cost: 0.0,
                        drain_per_second: 0.0,
                        cooldown_ticks: 0,
                        flight: true,
                        ..FormStats::default()
                    },
                    collision: FormCollision::default(),
                    attributes: FormAttributeBonuses::default(),
                    abilities: vec!["negative_hollow".into(), "mini_hollow".into(), "special_hollow".into()],
                    ultimate_ability: Some("special_hollow".into()),
                },
            ],
            _ => vec![base_form(meta.default_form, meta.base_abilities, meta.ultimate_ability)],
        }
    }
}

/// Charged jump controller (`com.devilfruits.core.form.ChargedJump`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ChargedJumpState {
    pub charging_since: Option<i64>,
    pub cooldown_until: i64,
}

impl ChargedJumpState {
    pub fn start(&mut self, now: i64) -> bool {
        if now < self.cooldown_until || self.charging_since.is_some() {
            return false;
        }
        self.charging_since = Some(now);
        true
    }

    pub fn progress(&self, now: i64) -> f32 {
        let Some(start) = self.charging_since else {
            return 0.0;
        };
        let elapsed = (now - start).max(0) as f32;
        (elapsed / CHARGED_JUMP_CHARGE_TICKS as f32).clamp(0.0, 1.0)
    }

    /// Releases the charged jump and returns launch velocity `(vx, vy, vz)` if above `MIN_CHARGE` (0.2).
    pub fn release(
        &mut self,
        now: i64,
        max_height: f64,
        gravity: f64,
        look_horizontal: DVec3,
    ) -> Option<DVec3> {
        let charge = self.progress(now);
        self.charging_since = None;
        if charge < CHARGED_JUMP_MIN_CHARGE {
            return None;
        }
        self.cooldown_until = now + CHARGED_JUMP_COOLDOWN_TICKS as i64;
        let height = CHARGED_JUMP_MIN_HEIGHT + (max_height - CHARGED_JUMP_MIN_HEIGHT).max(0.0) * (charge as f64);
        let vy = Self::takeoff_speed(height, gravity);
        let horiz_speed = 0.6 + 2.5 * (charge as f64);
        let flat = DVec3::new(look_horizontal.x, 0.0, look_horizontal.z).normalize_or_zero();
        Some(DVec3::new(flat.x * horiz_speed, vy, flat.z * horiz_speed))
    }

    /// Simulates exact vanilla vertical integration `y += v; v = (v - gravity) * 0.98` to peak height.
    pub fn peak_height(mut v: f64, gravity: f64) -> f64 {
        let g = gravity.max(1e-4);
        let mut y = 0.0;
        for _ in 0..400 {
            if v <= 0.0 {
                break;
            }
            y += v;
            v = (v - g) * 0.98;
        }
        y
    }

    /// Binary search matching `ChargedJump.takeoffSpeed(height, gravity)`.
    pub fn takeoff_speed(target_height: f64, gravity: f64) -> f64 {
        let mut lo = 0.0f64;
        let mut hi = 6.0f64;
        while Self::peak_height(hi, gravity) < target_height && hi < 64.0 {
            hi *= 2.0;
        }
        for _ in 0..24 {
            let mid = 0.5 * (lo + hi);
            if Self::peak_height(mid, gravity) < target_height {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi
    }
}

/// Form dash controller (`com.devilfruits.core.form.FormDash`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FormDashState {
    pub cooldown_until: i64,
    pub active_ticks: i32,
}

impl FormDashState {
    pub fn trigger(&mut self, now: i64, cooldown_ticks: i32, duration_ticks: i32) -> bool {
        if now < self.cooldown_until {
            return false;
        }
        self.cooldown_until = now + cooldown_ticks.max(0) as i64;
        self.active_ticks = duration_ticks.max(1);
        true
    }

    pub fn tick(&mut self) -> bool {
        if self.active_ticks > 0 {
            self.active_ticks -= 1;
            true
        } else {
            false
        }
    }
}

/// Safe landing protection after exiting flying/airborne forms (`com.devilfruits.core.form.SafeLanding`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeLandingState {
    pub ticks_remaining: i32,
}

impl SafeLandingState {
    pub fn arm(&mut self) {
        self.ticks_remaining = SAFE_LANDING_MAX_TICKS;
    }

    /// Ticks safe landing; returns `Some(60)` (Slow Falling duration) while airborne & armed.
    pub fn tick(&mut self, on_ground: bool, in_water: bool) -> Option<i32> {
        if self.ticks_remaining <= 0 {
            return None;
        }
        if on_ground || in_water {
            self.ticks_remaining = 0;
            return None;
        }
        self.ticks_remaining -= 1;
        Some(SAFE_LANDING_SLOW_FALLING_TICKS)
    }
}

/// Pending transformation windup (`FormManager.Windup`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormWindup {
    pub target_form: String,
    pub started_tick: i64,
    pub finish_tick: i64,
}

/// Outcome of attempting to switch forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormSwitchResult {
    Entered,
    WindupStarted,
    Reverted,
    NoFruit,
    UnknownForm,
    Suppressed,
    Locked,
    Cooldown,
    NoEnergy,
    AlreadyInForm,
}

/// Per-player transformation runtime (`FormManager` + `FormUpkeep`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FormRuntime {
    pub flags: u32,
    pub entered_tick: i64,
    pub time_limit_end_tick: Option<i64>,
    pub windup: Option<FormWindup>,
    pub charged_jump: ChargedJumpState,
    pub dash: FormDashState,
    pub safe_landing: SafeLandingState,
}

impl FormRuntime {
    pub fn set_flag(&mut self, mask: u32, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }

    pub fn has_flag(&self, mask: u32) -> bool {
        (self.flags & mask) != 0
    }
}

/// Form lifecycle & upkeep manager (`FormManager.java` + `FormUpkeep.java`).
pub struct FormManager;

impl FormManager {
    pub fn lookup_form(fruit: FruitId, name: &str) -> Option<FruitFormDef> {
        FruitFormDef::builtin_forms(fruit)
            .into_iter()
            .find(|f| f.id.name == name)
    }

    /// Requests entering or toggling a form.
    #[allow(clippy::too_many_arguments)]
    pub fn request_form(
        player: &mut PlayerFruitData,
        runtime: &mut FormRuntime,
        target_name: &str,
        now: i64,
        suppressed: bool,
        creative: bool,
        cfg: &CoreConfig,
    ) -> FormSwitchResult {
        let Some(fruit) = player.fruit_id else {
            return FormSwitchResult::NoFruit;
        };
        let default_form = fruit.meta().default_form;
        if target_name == default_form || player.active_form == target_name {
            if player.active_form != default_form {
                Self::revert(player, runtime, now, cfg);
                return FormSwitchResult::Reverted;
            }
            return FormSwitchResult::AlreadyInForm;
        }
        if suppressed {
            return FormSwitchResult::Suppressed;
        }
        let Some(def) = Self::lookup_form(fruit, target_name) else {
            return FormSwitchResult::UnknownForm;
        };
        if player.mastery < def.stats.unlock_mastery {
            return FormSwitchResult::Locked;
        }
        let cd_key = format!("form:{target_name}");
        if player.cooldown_remaining(&cd_key, now) > 0 {
            return FormSwitchResult::Cooldown;
        }
        if !player.spend_energy(def.stats.enter_cost, creative, cfg) {
            return FormSwitchResult::NoEnergy;
        }
        if def.stats.cast_ticks > 0 {
            runtime.windup = Some(FormWindup {
                target_form: target_name.to_string(),
                started_tick: now,
                finish_tick: now + def.stats.cast_ticks as i64,
            });
            FormSwitchResult::WindupStarted
        } else {
            Self::complete_enter(player, runtime, &def, now);
            FormSwitchResult::Entered
        }
    }

    fn complete_enter(
        player: &mut PlayerFruitData,
        runtime: &mut FormRuntime,
        def: &FruitFormDef,
        now: i64,
    ) {
        player.active_form = def.id.name.clone();
        runtime.entered_tick = now;
        runtime.time_limit_end_tick = def
            .stats
            .time_limit_seconds
            .map(|secs| now + (secs * 20.0).round() as i64);
        runtime.set_flag(form_flags::FLYING, def.stats.flight);
        runtime.set_flag(form_flags::BOOSTING, false);
        runtime.windup = None;
    }

    /// Reverts the player back to their fruit's base form and sets form cooldown + safe landing if flying.
    pub fn revert(
        player: &mut PlayerFruitData,
        runtime: &mut FormRuntime,
        now: i64,
        cfg: &CoreConfig,
    ) -> bool {
        let Some(fruit) = player.fruit_id else {
            return false;
        };
        let default_form = fruit.meta().default_form;
        runtime.windup = None;
        if player.active_form == default_form {
            return false;
        }
        let prev = player.active_form.clone();
        if let Some(def) = Self::lookup_form(fruit, &prev) {
            if def.stats.flight {
                runtime.safe_landing.arm();
            }
            if def.stats.cooldown_ticks > 0 {
                let cd = crate::core::rules::Mastery::scale_cooldown(
                    def.stats.cooldown_ticks,
                    player.mastery,
                    cfg,
                );
                player.set_cooldown(format!("form:{prev}"), now, cd);
            }
        }
        player.active_form = default_form.to_string();
        runtime.time_limit_end_tick = None;
        runtime.set_flag(form_flags::FLYING, false);
        runtime.set_flag(form_flags::BOOSTING, false);
        true
    }

    /// Ticks windups, energy upkeep, time limits, water/seastone reversion, and safe landing.
    /// Returns the active form's energy regen multiplier for this tick.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        player: &mut PlayerFruitData,
        runtime: &mut FormRuntime,
        now: i64,
        submerged: bool,
        suppressed: bool,
        on_ground: bool,
        creative: bool,
        cfg: &CoreConfig,
    ) -> (f32, Option<i32>) {
        let slow_fall = runtime.safe_landing.tick(on_ground, submerged);
        runtime.dash.tick();

        let Some(fruit) = player.fruit_id else {
            return (1.0, slow_fall);
        };

        // Cancel windup if suppressed; finish windup when timer expires.
        if let Some(windup) = runtime.windup.clone() {
            if suppressed {
                runtime.windup = None;
            } else if now >= windup.finish_tick {
                if let Some(def) = Self::lookup_form(fruit, &windup.target_form) {
                    Self::complete_enter(player, runtime, &def, now);
                } else {
                    runtime.windup = None;
                }
            }
        }

        let Some(active) = Self::lookup_form(fruit, &player.active_form) else {
            return (1.0, slow_fall);
        };
        if active.is_base {
            return (active.stats.energy_regen_multiplier, slow_fall);
        }

        // Forced reversions: water, seastone, time limit, or energy depletion.
        if (submerged && cfg.revert_in_water) || suppressed {
            Self::revert(player, runtime, now, cfg);
            return (1.0, slow_fall);
        }
        if let Some(end_tick) = runtime.time_limit_end_tick {
            if now >= end_tick {
                Self::revert(player, runtime, now, cfg);
                return (1.0, slow_fall);
            }
        }
        if active.stats.drain_per_second > 0.0 {
            let drain = active.stats.drain_per_second / 20.0;
            if !player.spend_energy(drain, creative, cfg) && cfg.revert_on_depletion {
                Self::revert(player, runtime, now, cfg);
                return (1.0, slow_fall);
            }
        }

        (active.stats.energy_regen_multiplier, slow_fall)
    }
}
