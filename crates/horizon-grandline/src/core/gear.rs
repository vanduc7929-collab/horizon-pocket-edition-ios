//! Gear system & Gomu-Gomu Gear 2 stamina, adaptation, and overexertion state machine.
//!
//! Mirrors `core/gear/{GearDefinition,GearRegistry,GearState,GearSwitch}.java`
//! and `fruits/gomu_gomu/gear/{Gear2,Gear2Adaptation}.java`.

use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;

/// Gear 2 exact constants extracted from `Gear2.java` and `Gear2Adaptation.java`.
pub const GEAR2_ENTER_TICKS: i32 = 19;
pub const GEAR2_MAX_ACTIVE_SECONDS: f32 = 60.0;
pub const GEAR2_MAX_ACTIVE_SECONDS_ADAPTED: f32 = 90.0;
pub const GEAR2_WARNING_SECONDS: f32 = 10.0;
pub const GEAR2_REST_SECONDS: f32 = 20.0;
pub const GEAR2_EXHAUSTION_SECONDS: f32 = 3.0;
pub const GEAR2_LOCKOUT_SECONDS: f32 = 10.0;
pub const GEAR2_HUNGER_PER_SECOND: f32 = 0.5;
pub const GEAR2_LOW_FOOD: f32 = 6.0;
pub const GEAR2_LOW_FOOD_FACTOR: f32 = 0.5;
pub const GEAR2_JET_DRAIN_PER_SECOND: f32 = 3.0;
pub const GEAR2_QUICK_ENTER_ADAPTATION: f32 = 0.6;
pub const GEAR2_ENTER_SLOW: f32 = 0.9;
pub const GEAR2_ADAPTATION_FULL_SECONDS: f32 = 1800.0;
pub const GEAR2_ADAPTATION_PER_JET_USE: f32 = 1.0;

/// Canonical gear identifier (`GearDefinition.id`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GearId(pub String);

impl GearId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Sub-form variant under a Gear (e.g. Gear 4 Bounce Man / Snake Man).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubGearDef {
    pub id: String,
    pub display_name: String,
    pub index: u32,
    pub available: bool,
}

/// Static declaration of a Gear wheel entry (`GearDefinition.java`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GearDefinition {
    pub id: GearId,
    pub display_name: String,
    pub order: u32,
    pub form_name: Option<String>,
    pub unlock_mastery: u32,
    pub available: bool,
    pub progress_key: Option<String>,
    pub sub_gears: Vec<SubGearDef>,
}

/// Registry of Gears per Devil Fruit (`GearRegistry.java`).
pub struct GearRegistry;

impl GearRegistry {
    /// Returns all Gears defined for a fruit (currently Gomu-Gomu Gears 1..=5).
    pub fn for_fruit(fruit: FruitId) -> Vec<GearDefinition> {
        if fruit != FruitId::GomuGomu {
            return Vec::new();
        }
        vec![
            GearDefinition {
                id: GearId::new("gear_1"),
                display_name: "Base".into(),
                order: 1,
                form_name: Some("base".into()),
                unlock_mastery: 1,
                available: true,
                progress_key: None,
                sub_gears: Vec::new(),
            },
            GearDefinition {
                id: GearId::new("gear_2"),
                display_name: "Gear 2".into(),
                order: 2,
                form_name: Some("gear_2".into()),
                unlock_mastery: 5,
                available: true,
                progress_key: Some("gear2_adaptation".into()),
                sub_gears: Vec::new(),
            },
            GearDefinition {
                id: GearId::new("gear_3"),
                display_name: "Gear 3".into(),
                order: 3,
                form_name: None,
                unlock_mastery: 7,
                available: false,
                progress_key: None,
                sub_gears: Vec::new(),
            },
            GearDefinition {
                id: GearId::new("gear_4"),
                display_name: "Gear 4".into(),
                order: 4,
                form_name: None,
                unlock_mastery: 9,
                available: false,
                progress_key: None,
                sub_gears: vec![
                    SubGearDef {
                        id: "bounce_man".into(),
                        display_name: "Bounce Man".into(),
                        index: 0,
                        available: false,
                    },
                    SubGearDef {
                        id: "snake_man".into(),
                        display_name: "Snake Man".into(),
                        index: 1,
                        available: false,
                    },
                ],
            },
            GearDefinition {
                id: GearId::new("gear_5"),
                display_name: "Gear 5".into(),
                order: 5,
                form_name: None,
                unlock_mastery: 10,
                available: false,
                progress_key: None,
                sub_gears: Vec::new(),
            },
        ]
    }
}

/// Configurable Gear 2 stamina & adaptation parameters (`Gear2.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gear2Config {
    pub enter_ticks: i32,
    pub max_active_seconds: f32,
    pub max_active_seconds_adapted: f32,
    pub warning_seconds: f32,
    pub rest_seconds: f32,
    pub exhaustion_seconds: f32,
    pub lockout_seconds: f32,
    pub hunger_per_second: f32,
    pub low_food: f32,
    pub low_food_factor: f32,
    pub jet_drain_per_second: f32,
    pub quick_enter_adaptation: f32,
    pub enter_slow: f32,
    pub adaptation_full_seconds: f32,
    pub adaptation_per_jet_use: f32,
}

impl Default for Gear2Config {
    fn default() -> Self {
        Self {
            enter_ticks: GEAR2_ENTER_TICKS,
            max_active_seconds: GEAR2_MAX_ACTIVE_SECONDS,
            max_active_seconds_adapted: GEAR2_MAX_ACTIVE_SECONDS_ADAPTED,
            warning_seconds: GEAR2_WARNING_SECONDS,
            rest_seconds: GEAR2_REST_SECONDS,
            exhaustion_seconds: GEAR2_EXHAUSTION_SECONDS,
            lockout_seconds: GEAR2_LOCKOUT_SECONDS,
            hunger_per_second: GEAR2_HUNGER_PER_SECOND,
            low_food: GEAR2_LOW_FOOD,
            low_food_factor: GEAR2_LOW_FOOD_FACTOR,
            jet_drain_per_second: GEAR2_JET_DRAIN_PER_SECOND,
            quick_enter_adaptation: GEAR2_QUICK_ENTER_ADAPTATION,
            enter_slow: GEAR2_ENTER_SLOW,
            adaptation_full_seconds: GEAR2_ADAPTATION_FULL_SECONDS,
            adaptation_per_jet_use: GEAR2_ADAPTATION_PER_JET_USE,
        }
    }
}

/// Persistent Gear 2 body adaptation progress (`Gear2Adaptation.java`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gear2Adaptation {
    /// Accumulated adaptation seconds (`0.0..=adaptation_full_seconds`).
    pub seconds: f32,
}

impl Gear2Adaptation {
    /// Normalized adaptation progress in `0.0..=1.0`.
    pub fn fraction(self, cfg: &Gear2Config) -> f32 {
        if cfg.adaptation_full_seconds <= 0.0 {
            return 1.0;
        }
        (self.seconds / cfg.adaptation_full_seconds).clamp(0.0, 1.0)
    }

    pub fn set_fraction(&mut self, frac: f32, cfg: &Gear2Config) {
        self.seconds = frac.clamp(0.0, 1.0) * cfg.adaptation_full_seconds;
    }

    pub fn add_seconds(&mut self, delta: f32, cfg: &Gear2Config) {
        self.seconds = (self.seconds + delta.max(0.0)).clamp(0.0, cfg.adaptation_full_seconds);
    }

    pub fn on_jet_technique_used(&mut self, cfg: &Gear2Config) {
        self.add_seconds(cfg.adaptation_per_jet_use, cfg);
    }

    /// True once player reaches `quick_enter_adaptation` (`0.6`), allowing instant entry without pump pose slow.
    pub fn has_quick_enter(self, cfg: &Gear2Config) -> bool {
        self.fraction(cfg) >= cfg.quick_enter_adaptation
    }

    /// Interpolated maximum continuous Gear 2 duration (`60s` -> `90s` linearly with adaptation).
    pub fn max_active_seconds(self, cfg: &Gear2Config) -> f32 {
        let t = self.fraction(cfg);
        cfg.max_active_seconds + (cfg.max_active_seconds_adapted - cfg.max_active_seconds) * t
    }
}

/// Events emitted while ticking Gear 2 stamina & overexertion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gear2TickOutcome {
    pub in_warning: bool,
    pub forced_exit: bool,
    pub exhaustion_ticks: i32,
    pub hunger_exhaustion_delta: f32,
}

/// Runtime Gear state synced per player (`GearState.java` + `Gear2.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GearState {
    pub active_gear: GearId,
    pub active_sub_gear: Option<String>,
    pub gear2_active: bool,
    pub gear2_strain_seconds: f32,
    pub gear2_lockout_until: i64,
    pub gear2_adaptation: Gear2Adaptation,
}

impl Default for GearState {
    fn default() -> Self {
        Self {
            active_gear: GearId::new("gear_1"),
            active_sub_gear: None,
            gear2_active: false,
            gear2_strain_seconds: 0.0,
            gear2_lockout_until: 0,
            gear2_adaptation: Gear2Adaptation::default(),
        }
    }
}

impl GearState {
    pub fn is_gear2_locked_out(&self, now: i64) -> bool {
        now < self.gear2_lockout_until
    }

    /// Effective Gear 2 enter windup ticks (`0` once quick-enter adaptation `>= 0.6` is reached).
    pub fn gear2_enter_ticks(&self, cfg: &Gear2Config) -> i32 {
        if self.gear2_adaptation.has_quick_enter(cfg) {
            0
        } else {
            cfg.enter_ticks
        }
    }

    /// Applies extra strain from casting a Jet technique while in Gear 2 (`3.0s` default) + adaptation gain.
    pub fn on_jet_technique_cast(&mut self, cfg: &Gear2Config) {
        if self.gear2_active {
            self.gear2_strain_seconds += cfg.jet_drain_per_second;
            self.gear2_adaptation.on_jet_technique_used(cfg);
        }
    }

    /// Advances Gear 2 strain/recovery by 1 tick (`1/20 s`).
    pub fn tick_gear2(
        &mut self,
        now: i64,
        food_level: f32,
        cfg: &Gear2Config,
    ) -> Gear2TickOutcome {
        let dt = 1.0 / 20.0;
        let max_s = self.gear2_adaptation.max_active_seconds(cfg).max(1.0);
        if !self.gear2_active {
            if self.gear2_strain_seconds > 0.0 {
                let recover_rate = max_s / cfg.rest_seconds.max(1.0);
                self.gear2_strain_seconds = (self.gear2_strain_seconds - recover_rate * dt).max(0.0);
            }
            return Gear2TickOutcome {
                in_warning: false,
                forced_exit: false,
                exhaustion_ticks: 0,
                hunger_exhaustion_delta: 0.0,
            };
        }

        let effective_max = if food_level < cfg.low_food {
            max_s * cfg.low_food_factor
        } else {
            max_s
        };
        self.gear2_strain_seconds += dt;
        self.gear2_adaptation.add_seconds(dt, cfg);

        let remaining = effective_max - self.gear2_strain_seconds;
        let in_warning = remaining <= cfg.warning_seconds;
        if self.gear2_strain_seconds >= effective_max {
            self.gear2_active = false;
            self.active_gear = GearId::new("gear_1");
            self.gear2_strain_seconds = 0.0;
            self.gear2_lockout_until = now + (cfg.lockout_seconds * 20.0).round() as i64;
            return Gear2TickOutcome {
                in_warning: true,
                forced_exit: true,
                exhaustion_ticks: (cfg.exhaustion_seconds * 20.0).round() as i32,
                hunger_exhaustion_delta: cfg.hunger_per_second * dt,
            };
        }

        Gear2TickOutcome {
            in_warning,
            forced_exit: false,
            exhaustion_ticks: 0,
            hunger_exhaustion_delta: cfg.hunger_per_second * dt,
        }
    }
}

/// Gear switch validation & execution (`com.devilfruits.core.gear.GearSwitch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GearSwitchOutcome {
    Switched,
    Unavailable,
    LockedByMastery,
    LockedOut,
    Suppressed,
    NotSupported,
}

pub struct GearSwitch;

impl GearSwitch {
    pub fn request(
        fruit: FruitId,
        mastery: u32,
        suppressed: bool,
        now: i64,
        target_gear: &str,
        sub_gear: Option<&str>,
        state: &mut GearState,
    ) -> GearSwitchOutcome {
        let defs = GearRegistry::for_fruit(fruit);
        if defs.is_empty() {
            return GearSwitchOutcome::NotSupported;
        }
        let Some(def) = defs.into_iter().find(|d| d.id.0 == target_gear) else {
            return GearSwitchOutcome::Unavailable;
        };
        if !def.available {
            return GearSwitchOutcome::Unavailable;
        }
        if target_gear != "gear_1" {
            if suppressed {
                return GearSwitchOutcome::Suppressed;
            }
            if mastery < def.unlock_mastery {
                return GearSwitchOutcome::LockedByMastery;
            }
            if target_gear == "gear_2" && state.is_gear2_locked_out(now) {
                return GearSwitchOutcome::LockedOut;
            }
        }
        state.active_gear = def.id;
        state.active_sub_gear = sub_gear.map(str::to_string);
        state.gear2_active = state.active_gear.0 == "gear_2";
        GearSwitchOutcome::Switched
    }
}
