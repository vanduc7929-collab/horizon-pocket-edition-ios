//! Complete Logia intangibility, bypass, elemental counter, hit-hole carving,
//! limb dissolve, elemental dash/flight, reform, environment scaling, and
//! SolidBody (Hie-Hie ice plating/shatter) engine.
//!
//! Mirrors `core/logia/{LogiaProfile,LogiaState,LogiaData,LogiaManager,LogiaBypass,
//! LogiaBypassSource,LogiaCounter,CounterTrigger,DamageCategory,HitRule,Limbs,
//! LogiaDamage,LogiaDash,LogiaMovement,LogiaReform,LogiaEnvironment,LogiaHits,
//! LogiaInteractionRegistry,solid/SolidBody,solid/SolidBodies,solid/SolidState}.java`
//! and `client/logia/LogiaRenderState.java`.

use std::collections::BTreeMap;

use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::rules::{CoreConfig, PlayerFruitData};

/// Maximum simultaneous punched holes / capsule tunnels (`LogiaRenderState.MAX_HOLES`).
pub const LOGIA_MAX_HOLES: usize = 8;

/// Maximum crater decals carved per limb (`LogiaRenderState.CRATERS_PER_LIMB`).
pub const LOGIA_CRATERS_PER_LIMB: usize = 3;

/// Ticks for a carved projectile tunnel to refill (`LogiaRenderState.TUNNEL_REFILL`).
pub const LOGIA_TUNNEL_REFILL_TICKS: i32 = 24;

/// Ticks before an unrefreshed elemental hit fully releases (`LogiaRenderState.RELEASE_AFTER`).
pub const LOGIA_RELEASE_AFTER_TICKS: i32 = 200;

/// Awakening duration on initial elemental entry (`LogiaManager.AWAKEN_TICKS`).
pub const LOGIA_AWAKEN_TICKS: i32 = 30;

/// Arrival invulnerability/grace ticks after elemental dash (`LogiaDash.ARRIVAL_GRACE`).
pub const LOGIA_ARRIVAL_GRACE_TICKS: i32 = 10;

/// SolidBody regrow cadence per limb (`SolidBodies.REGROW_STEP`).
pub const SOLID_REGROW_STEP_TICKS: i32 = 6;

/// SolidBody crack decay cadence (`SolidBodies.CRACK_DECAY_STEP`).
pub const SOLID_CRACK_DECAY_STEP_TICKS: i32 = 10;

/// Maximum crack stage per limb (`0..=4`, packed 3 bits each in `cracks: u32`).
pub const SOLID_MAX_CRACK_STAGE: u8 = 4;

/// 6-limb bitmask constants (`com.devilfruits.core.logia.Limbs`).
pub mod limbs {
    pub const HEAD: u8 = 1;
    pub const BODY: u8 = 2;
    pub const RIGHT_ARM: u8 = 4;
    pub const LEFT_ARM: u8 = 8;
    pub const RIGHT_LEG: u8 = 16;
    pub const LEFT_LEG: u8 = 32;
    pub const ARMS: u8 = RIGHT_ARM | LEFT_ARM;
    pub const LEGS: u8 = RIGHT_LEG | LEFT_LEG;
    pub const ALL: u8 = 63;
    pub const COUNT: usize = 6;

    pub const fn bit(index: usize) -> u8 {
        if index < COUNT { 1u8 << index } else { 0 }
    }
}

/// 5 Logia body states (`com.devilfruits.core.logia.LogiaState`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogiaState {
    #[default]
    Solid,
    Partial,
    ElementalBody,
    Reforming,
    Suppressed,
}

impl LogiaState {
    pub const fn is_elemental(self) -> bool {
        matches!(self, Self::Partial | Self::ElementalBody)
    }

    pub const fn wire_id(self) -> u8 {
        match self {
            Self::Solid => 0,
            Self::Partial => 1,
            Self::ElementalBody => 2,
            Self::Reforming => 3,
            Self::Suppressed => 4,
        }
    }

    pub const fn from_wire(v: u8) -> Self {
        match v {
            1 => Self::Partial,
            2 => Self::ElementalBody,
            3 => Self::Reforming,
            4 => Self::Suppressed,
            _ => Self::Solid,
        }
    }
}

/// 13 damage categories classified for Logia intangibility (`DamageCategory.java`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DamageCategory {
    Melee,
    Projectile,
    Explosion,
    Fall,
    Environment,
    Fire,
    Lightning,
    Freeze,
    Ability,
    Magic,
    Water,
    Always,
    Other,
}

impl DamageCategory {
    /// Default hit rule (`DamageCategory.defaultRule()`).
    pub const fn default_rule(self) -> HitRule {
        match self {
            Self::Melee
            | Self::Projectile
            | Self::Explosion
            | Self::Fall
            | Self::Environment => HitRule::Pass,
            Self::Fire | Self::Lightning | Self::Freeze | Self::Ability => HitRule::Partial,
            Self::Magic | Self::Water | Self::Always | Self::Other => HitRule::Connect,
        }
    }
}

/// How an incoming damage category interacts with an active Logia body (`HitRule.java`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitRule {
    /// Passes cleanly through the element (0 damage, carves hole / dissolves limb).
    Pass,
    /// Deals `partial_factor` (default 0.5) while Solid/Partial; becomes Pass in `ElementalBody`.
    Partial,
    /// Connects directly at full damage.
    Connect,
}

/// Reason a strike bypassed Logia intangibility (`LogiaBypassSource.java`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogiaBypassSource {
    SeastoneWeapon,
    ArmamentHaki,
    ElementalCounter,
    LogiaSuperiority,
    SolidShatter,
    Other,
}

/// Result of querying bypass evaluators (`LogiaBypass.Result`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaBypassResult {
    pub source: LogiaBypassSource,
    /// Fraction of damage allowed through intangibility (`0.0..=1.0`).
    pub fraction: f32,
}

/// Cross-Logia elemental superiority table (`LogiaInteractionRegistry.java`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogiaInteraction {
    Neutral,
    /// Attacker's element overpowers defender's Logia (e.g. `magma -> ice`).
    Bypass,
    /// Defender's element resists attacker's element (e.g. `ice -> magma`).
    Resisted,
}

impl LogiaInteraction {
    /// Exact built-in matrix from `MaguMagu.java` & `LogiaInteractionRegistry.java`.
    pub fn evaluate(attacker_element: &str, defender_element: &str) -> Self {
        match (attacker_element, defender_element) {
            ("magma", "ice") => Self::Bypass,
            ("ice", "magma") => Self::Resisted,
            _ => Self::Neutral,
        }
    }
}

/// Trigger conditions for a Logia elemental counter (`CounterTrigger.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterTrigger {
    InWater,
    Rain,
    SplashWater,
    WaterBucket,
    WaterDamage,
    SeastoneTouch,
    SeastoneNear(f64),
    HeatDamage,
    NearFireOrLava,
}

/// Effects imposed while a Logia counter is active (`LogiaCounter.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterAction {
    SuppressIntangibility,
    ForceReform,
    BlockAbilities,
    Weaken(f32),
    Slow(f32),
}

/// Counter specification (`LogiaCounter.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaCounter {
    pub id: String,
    pub triggers: Vec<CounterTrigger>,
    pub actions: Vec<CounterAction>,
    pub linger_ticks: i32,
}

impl LogiaCounter {
    pub fn suppresses_intangibility(&self) -> bool {
        self.actions
            .iter()
            .any(|a| matches!(a, CounterAction::SuppressIntangibility))
    }

    pub fn forces_reform(&self) -> bool {
        self.actions
            .iter()
            .any(|a| matches!(a, CounterAction::ForceReform))
    }

    pub fn blocks_abilities(&self) -> bool {
        self.actions
            .iter()
            .any(|a| matches!(a, CounterAction::BlockAbilities))
    }

    pub fn damage_multiplier(&self) -> f32 {
        self.actions.iter().fold(1.0f32, |acc, a| match a {
            CounterAction::Weaken(m) => acc * *m,
            _ => acc,
        })
    }

    pub fn slow_amount(&self) -> f32 {
        self.actions.iter().fold(0.0f32, |acc, a| match a {
            CounterAction::Slow(s) => acc.max(*s),
            _ => acc,
        })
    }
}

/// Environmental regen modifier profile (`LogiaEnvironment.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaEnvironment {
    pub source_regen_mult: f32,
    pub opposed_regen_mult: f32,
    pub rain_opposes: bool,
}

impl Default for LogiaEnvironment {
    fn default() -> Self {
        Self {
            source_regen_mult: 1.5,
            opposed_regen_mult: 0.5,
            rain_opposes: false,
        }
    }
}

/// Solid-body plating spec (used by Hie-Hie ice body; `core/logia/solid/SolidBody.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SolidBodySpec {
    pub base_plating: f32,
    pub plating_per_level: f32,
    pub shatter_fraction: f32,
    pub regrow_delay_seconds: f32,
    pub regrow_per_second: f32,
    pub elemental_factor: f32,
    pub stagger_ticks: i32,
    pub mult_lava: f32,
    pub mult_heat: f32,
    pub mult_shatter: f32,
    pub mult_chip: f32,
    pub mult_explosion: f32,
    pub last_shatter_enabled: bool,
    pub last_shatter_health: f32,
    pub last_shatter_cooldown_seconds: i32,
    pub last_shatter_energy_fraction: f32,
}

impl SolidBodySpec {
    /// Exact Hie-Hie solid ice body parameters from `HieHie.java` & `SolidBody.java`.
    pub const HIE_HIE: Self = Self {
        base_plating: 12.0,
        plating_per_level: 2.0,
        shatter_fraction: 0.35,
        regrow_delay_seconds: 3.0,
        regrow_per_second: 4.0,
        elemental_factor: 0.4,
        stagger_ticks: 8,
        mult_lava: 3.0,
        mult_heat: 2.0,
        mult_shatter: 1.5,
        mult_chip: 0.7,
        mult_explosion: 1.5,
        last_shatter_enabled: false,
        last_shatter_health: 2.0,
        last_shatter_cooldown_seconds: 300,
        last_shatter_energy_fraction: 0.5,
    };

    pub fn max_plating(self, mastery: u32) -> f32 {
        self.base_plating + self.plating_per_level * (mastery.max(1) - 1) as f32
    }
}

/// Per-player SolidBody runtime (`core/logia/solid/SolidState.java`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SolidState {
    pub plating: f32,
    /// Packed 6 limbs × 3 bits (each `0..=4`).
    pub cracks: u32,
    /// 6-bit mask of completely shattered limbs (`limbs::HEAD..limbs::LEFT_LEG`).
    pub shattered_limbs: u8,
    pub last_hit_tick: i64,
    pub stagger_until: i64,
    pub last_shatter_cooldown_until: i64,
}

impl SolidState {
    pub fn crack_stage(self, limb_index: usize) -> u8 {
        if limb_index >= limbs::COUNT {
            return 0;
        }
        ((self.cracks >> (limb_index * 3)) & 0b111) as u8
    }

    pub fn set_crack_stage(&mut self, limb_index: usize, stage: u8) {
        if limb_index >= limbs::COUNT {
            return;
        }
        let clamped = stage.min(SOLID_MAX_CRACK_STAGE) as u32;
        let shift = limb_index * 3;
        self.cracks = (self.cracks & !(0b111 << shift)) | (clamped << shift);
    }

    pub fn is_shattered(self, limb_index: usize) -> bool {
        (self.shattered_limbs & limbs::bit(limb_index)) != 0
    }

    /// Applies an impact to the solid plating; returns `(hp_damage, shattered_now, stagger_ticks)`.
    pub fn absorb_hit(
        &mut self,
        spec: SolidBodySpec,
        mastery: u32,
        limb_index: usize,
        raw_damage: f32,
        category_mult: f32,
        now: i64,
    ) -> (f32, bool, i32) {
        let max_p = spec.max_plating(mastery).max(1.0);
        if self.plating <= 0.0 {
            self.plating = max_p;
        }
        self.last_hit_tick = now;
        let eff = raw_damage * category_mult;
        let absorbed = eff.min(self.plating);
        self.plating = (self.plating - absorbed).max(0.0);
        let overflow = (eff - absorbed).max(0.0);

        let cur_stage = self.crack_stage(limb_index);
        let stage_inc = if eff >= max_p * spec.shatter_fraction { 2 } else { 1 };
        let next_stage = (cur_stage + stage_inc).min(SOLID_MAX_CRACK_STAGE);
        self.set_crack_stage(limb_index, next_stage);

        let mut shattered_now = false;
        let mut stagger = 0;
        if self.plating <= 0.0 || next_stage >= SOLID_MAX_CRACK_STAGE {
            self.shattered_limbs |= limbs::bit(limb_index);
            shattered_now = true;
            stagger = spec.stagger_ticks;
            self.stagger_until = now + stagger as i64;
        }

        let hp_damage = overflow + if shattered_now { raw_damage * 0.25 } else { 0.0 };
        (hp_damage, shattered_now, stagger)
    }

    /// Ticks plating regrowth & crack healing (`SolidBodies.tick`).
    pub fn tick(&mut self, spec: SolidBodySpec, mastery: u32, now: i64) {
        let delay_ticks = (spec.regrow_delay_seconds * 20.0).round() as i64;
        if now.saturating_sub(self.last_hit_tick) < delay_ticks {
            return;
        }
        let max_p = spec.max_plating(mastery);
        self.plating = (self.plating + spec.regrow_per_second / 20.0).min(max_p);

        if (now % SOLID_REGROW_STEP_TICKS as i64) == 0 && self.shattered_limbs != 0 {
            for idx in 0..limbs::COUNT {
                if self.is_shattered(idx) {
                    self.shattered_limbs &= !limbs::bit(idx);
                    self.set_crack_stage(idx, SOLID_MAX_CRACK_STAGE.saturating_sub(1));
                    break;
                }
            }
        }
        if (now % SOLID_CRACK_DECAY_STEP_TICKS as i64) == 0 && self.cracks != 0 {
            for idx in 0..limbs::COUNT {
                let st = self.crack_stage(idx);
                if st > 0 {
                    self.set_crack_stage(idx, st - 1);
                }
            }
        }
    }
}

/// Logia configuration defaults (`CoreConfig.java` Logia section).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaConfig {
    pub intangibility_enabled: bool,
    pub partial_factor: f32,
    pub negate_energy: f32,
    pub negate_energy_per_damage: f32,
    pub seastone_weapons_bypass: bool,
    pub armament_bypass: bool,
    pub reform_ticks: i32,
    pub default_dash_range: f32,
    pub default_dash_cooldown: i32,
    pub default_dash_energy: f32,
    pub default_dash_speed: f32,
}

impl Default for LogiaConfig {
    fn default() -> Self {
        Self {
            intangibility_enabled: true,
            partial_factor: 0.5,
            negate_energy: 0.0,
            negate_energy_per_damage: 0.0,
            seastone_weapons_bypass: true,
            armament_bypass: true,
            reform_ticks: 8,
            default_dash_range: 8.0,
            default_dash_cooldown: 40,
            default_dash_energy: 10.0,
            default_dash_speed: 1.0,
        }
    }
}

/// Complete profile for one of the 5 Logia fruits (`LogiaProfile.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaProfile {
    pub fruit: FruitId,
    pub element: String,
    pub elemental_drain_per_second: f32,
    pub dash_range: f32,
    pub dash_speed: f32,
    pub rule_overrides: BTreeMap<DamageCategory, HitRule>,
    pub counters: Vec<LogiaCounter>,
    pub environment: LogiaEnvironment,
    pub solid_body: Option<SolidBodySpec>,
}

impl LogiaProfile {
    fn seastone_counter(near_dist: f64) -> LogiaCounter {
        LogiaCounter {
            id: "seastone".into(),
            triggers: vec![
                CounterTrigger::SeastoneTouch,
                CounterTrigger::SeastoneNear(near_dist),
            ],
            actions: vec![
                CounterAction::SuppressIntangibility,
                CounterAction::ForceReform,
                CounterAction::BlockAbilities,
            ],
            linger_ticks: 10,
        }
    }

    /// Returns the exact canonical `LogiaProfile` for each of the 5 Logia fruits.
    pub fn for_fruit(fruit: FruitId) -> Option<Self> {
        match fruit {
            FruitId::SunaSuna => Some(Self {
                fruit,
                element: "sand".into(),
                elemental_drain_per_second: 3.0,
                dash_range: 16.0,
                dash_speed: 2.2,
                rule_overrides: BTreeMap::new(),
                counters: vec![
                    LogiaCounter {
                        id: "wet".into(),
                        triggers: vec![
                            CounterTrigger::InWater,
                            CounterTrigger::Rain,
                            CounterTrigger::SplashWater,
                            CounterTrigger::WaterBucket,
                            CounterTrigger::WaterDamage,
                        ],
                        actions: vec![
                            CounterAction::SuppressIntangibility,
                            CounterAction::ForceReform,
                            CounterAction::Weaken(0.6),
                            CounterAction::Slow(0.15),
                        ],
                        linger_ticks: 100,
                    },
                    Self::seastone_counter(2.0),
                ],
                environment: LogiaEnvironment {
                    source_regen_mult: 1.5,
                    opposed_regen_mult: 0.5,
                    rain_opposes: true,
                },
                solid_body: None,
            }),
            FruitId::PikaPika => Some(Self {
                fruit,
                element: "light".into(),
                elemental_drain_per_second: 3.0,
                dash_range: 32.0,
                dash_speed: 4.5,
                rule_overrides: BTreeMap::new(),
                counters: vec![Self::seastone_counter(2.0)],
                environment: LogiaEnvironment {
                    source_regen_mult: 1.5,
                    opposed_regen_mult: 0.5,
                    rain_opposes: false,
                },
                solid_body: None,
            }),
            FruitId::HieHie => Some(Self {
                fruit,
                element: "ice".into(),
                elemental_drain_per_second: 4.0,
                dash_range: 24.0,
                dash_speed: 3.0,
                rule_overrides: BTreeMap::new(),
                counters: vec![
                    Self::seastone_counter(2.0),
                    LogiaCounter {
                        id: "heat".into(),
                        triggers: vec![CounterTrigger::HeatDamage, CounterTrigger::NearFireOrLava],
                        actions: vec![CounterAction::Weaken(0.7), CounterAction::ForceReform],
                        linger_ticks: 40,
                    },
                ],
                environment: LogiaEnvironment {
                    source_regen_mult: 1.5,
                    opposed_regen_mult: 0.5,
                    rain_opposes: false,
                },
                solid_body: Some(SolidBodySpec::HIE_HIE),
            }),
            FruitId::MaguMagu => {
                let mut rule_overrides = BTreeMap::new();
                rule_overrides.insert(DamageCategory::Fire, HitRule::Pass);
                Some(Self {
                    fruit,
                    element: "magma".into(),
                    elemental_drain_per_second: 4.0,
                    dash_range: 24.0,
                    dash_speed: 3.0,
                    rule_overrides,
                    counters: vec![Self::seastone_counter(2.0)],
                    environment: LogiaEnvironment {
                        source_regen_mult: 1.5,
                        opposed_regen_mult: 0.5,
                        rain_opposes: false,
                    },
                    solid_body: None,
                })
            }
            FruitId::MokuMoku => Some(Self {
                fruit,
                element: "smoke".into(),
                elemental_drain_per_second: 3.0,
                dash_range: 28.0,
                dash_speed: 3.5,
                rule_overrides: BTreeMap::new(),
                counters: vec![Self::seastone_counter(2.0)],
                environment: LogiaEnvironment {
                    source_regen_mult: 1.5,
                    opposed_regen_mult: 0.7,
                    rain_opposes: true,
                },
                solid_body: None,
            }),
            _ => None,
        }
    }

    pub fn hit_rule(&self, category: DamageCategory) -> HitRule {
        self.rule_overrides
            .get(&category)
            .copied()
            .unwrap_or_else(|| category.default_rule())
    }
}

/// Carved capsule hole / projectile tunnel in a Logia body (`LogiaRenderState.Hole`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaCapsuleHole {
    pub entry_local: [f32; 3],
    pub exit_local: [f32; 3],
    pub radius: f32,
    pub limb_index: u8,
    pub created_tick: i64,
    pub refill_ticks: i32,
}

impl LogiaCapsuleHole {
    pub fn age(self, now: i64) -> i32 {
        (now - self.created_tick).max(0) as i32
    }

    pub fn is_expired(self, now: i64) -> bool {
        self.age(now) >= self.refill_ticks.max(1)
    }

    /// Remaining open radius fraction in `0.0..=1.0`.
    pub fn open_fraction(self, now: i64) -> f32 {
        let t = (self.age(now) as f32 / self.refill_ticks.max(1) as f32).clamp(0.0, 1.0);
        1.0 - t
    }
}

/// Active elemental dash trajectory (`LogiaDash.java`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveElementalDash {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub started_tick: i64,
    pub duration_ticks: i32,
}

impl ActiveElementalDash {
    /// Exact quadratic ease-out progress matching `LogiaDash`: `1 - (1 - c)^2`.
    pub fn progress(self, now: i64) -> f64 {
        let c = ((now - self.started_tick).max(0) as f64 / self.duration_ticks.max(1) as f64)
            .clamp(0.0, 1.0);
        1.0 - (1.0 - c) * (1.0 - c)
    }

    pub fn position(self, now: i64) -> DVec3 {
        let s = DVec3::from_array(self.start);
        let e = DVec3::from_array(self.end);
        s.lerp(e, self.progress(now))
    }

    pub fn is_finished(self, now: i64) -> bool {
        now >= self.started_tick + self.duration_ticks as i64
    }
}

/// Per-player synced & persistent Logia runtime (`LogiaData.java` + `LogiaRenderState.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaData {
    pub state: LogiaState,
    pub intangibility_toggled: bool,
    pub dissolved_limbs: u8,
    pub limb_reform_until: [i64; limbs::COUNT],
    pub holes: Vec<LogiaCapsuleHole>,
    pub active_counters: BTreeMap<String, i64>,
    pub reform_until_tick: i64,
    pub awaken_until_tick: i64,
    pub dash_cooldown_until: i64,
    pub arrival_grace_until: i64,
    pub active_dash: Option<ActiveElementalDash>,
    pub last_hit_tick: i64,
    pub solid_state: SolidState,
}

impl Default for LogiaData {
    fn default() -> Self {
        Self {
            state: LogiaState::Solid,
            intangibility_toggled: true,
            dissolved_limbs: 0,
            limb_reform_until: [0; limbs::COUNT],
            holes: Vec::new(),
            active_counters: BTreeMap::new(),
            reform_until_tick: 0,
            awaken_until_tick: 0,
            dash_cooldown_until: 0,
            arrival_grace_until: 0,
            active_dash: None,
            last_hit_tick: 0,
            solid_state: SolidState::default(),
        }
    }
}

impl LogiaData {
    pub fn is_counter_active(&self, id: &str, now: i64) -> bool {
        self.active_counters
            .get(id)
            .is_some_and(|&until| now < until)
    }

    /// Applies/refreshes a counter with its linger duration.
    pub fn trigger_counter(&mut self, counter: &LogiaCounter, now: i64) {
        let until = now + counter.linger_ticks.max(1) as i64;
        let entry = self.active_counters.entry(counter.id.clone()).or_insert(0);
        if until > *entry {
            *entry = until;
        }
        if counter.forces_reform() && self.state.is_elemental() {
            self.state = LogiaState::Reforming;
            self.reform_until_tick = now + 8;
        }
    }

    /// Carves a capsule hole/tunnel, respecting `LOGIA_MAX_HOLES` (8) and `LOGIA_CRATERS_PER_LIMB` (3).
    pub fn carve_hole(
        &mut self,
        entry_local: Vec3,
        exit_local: Vec3,
        radius: f32,
        limb_index: usize,
        now: i64,
        reform_ticks: i32,
    ) {
        let limb = (limb_index.min(limbs::COUNT - 1)) as u8;
        let per_limb_count = self.holes.iter().filter(|h| h.limb_index == limb).count();
        if per_limb_count >= LOGIA_CRATERS_PER_LIMB {
            if let Some(pos) = self.holes.iter().position(|h| h.limb_index == limb) {
                self.holes.remove(pos);
            }
        }
        if self.holes.len() >= LOGIA_MAX_HOLES {
            self.holes.remove(0);
        }
        self.holes.push(LogiaCapsuleHole {
            entry_local: entry_local.to_array(),
            exit_local: exit_local.to_array(),
            radius,
            limb_index: limb,
            created_tick: now,
            refill_ticks: LOGIA_TUNNEL_REFILL_TICKS.max(reform_ticks),
        });
        self.dissolve_limb(limb as usize, now, reform_ticks);
        self.last_hit_tick = now;
    }

    pub fn dissolve_limb(&mut self, limb_index: usize, now: i64, reform_ticks: i32) {
        if limb_index >= limbs::COUNT {
            return;
        }
        self.dissolved_limbs |= limbs::bit(limb_index);
        self.limb_reform_until[limb_index] = now + reform_ticks.max(1) as i64;
    }
}

/// Outcome of routing an incoming hit through `LogiaManager::evaluate_damage`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogiaDamageOutcome {
    pub final_damage: f32,
    pub passed_through: bool,
    pub carved_hole: bool,
    pub bypass: Option<LogiaBypassResult>,
    pub solid_shattered: bool,
    pub stagger_ticks: i32,
}

/// Core Logia controller (`LogiaManager`, `LogiaDamage`, `LogiaDash`, `LogiaReform`).
pub struct LogiaManager;

impl LogiaManager {
    /// Returns true if any currently active counter suppresses intangibility.
    pub fn is_suppressed_by_counter(profile: &LogiaProfile, data: &LogiaData, now: i64) -> bool {
        profile
            .counters
            .iter()
            .any(|c| c.suppresses_intangibility() && data.is_counter_active(&c.id, now))
    }

    /// Returns true if any active counter blocks fruit abilities (e.g. Seastone).
    pub fn abilities_blocked_by_counter(profile: &LogiaProfile, data: &LogiaData, now: i64) -> bool {
        profile
            .counters
            .iter()
            .any(|c| c.blocks_abilities() && data.is_counter_active(&c.id, now))
    }

    /// Combined damage output multiplier from active counters (e.g. Suna-Suna wet `0.6`, Hie-Hie heat `0.7`).
    pub fn active_weaken_multiplier(profile: &LogiaProfile, data: &LogiaData, now: i64) -> f32 {
        profile.counters.iter().fold(1.0f32, |acc, c| {
            if data.is_counter_active(&c.id, now) {
                acc * c.damage_multiplier()
            } else {
                acc
            }
        })
    }

    /// Evaluates incoming damage against a Logia user (`LogiaDamage.java`).
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_damage(
        profile: &LogiaProfile,
        player: &mut PlayerFruitData,
        data: &mut LogiaData,
        category: DamageCategory,
        raw_damage: f32,
        limb_index: usize,
        hit_entry: Vec3,
        hit_exit: Vec3,
        bypass: Option<LogiaBypassResult>,
        attacker_logia_element: Option<&str>,
        now: i64,
        creative: bool,
        core_cfg: &CoreConfig,
        logia_cfg: &LogiaConfig,
    ) -> LogiaDamageOutcome {
        // Arrival grace after elemental dash gives brief pass-through on non-Always hits.
        if now < data.arrival_grace_until && category != DamageCategory::Always {
            return LogiaDamageOutcome {
                final_damage: 0.0,
                passed_through: true,
                carved_hole: false,
                bypass: None,
                solid_shattered: false,
                stagger_ticks: 0,
            };
        }

        // Cross-Logia elemental superiority (e.g. Magma bypasses Ice; Ice resisted by Magma).
        if let Some(atk_elem) = attacker_logia_element {
            match LogiaInteraction::evaluate(atk_elem, &profile.element) {
                LogiaInteraction::Bypass => {
                    return LogiaDamageOutcome {
                        final_damage: raw_damage,
                        passed_through: false,
                        carved_hole: false,
                        bypass: Some(LogiaBypassResult {
                            source: LogiaBypassSource::LogiaSuperiority,
                            fraction: 1.0,
                        }),
                        solid_shattered: false,
                        stagger_ticks: 0,
                    };
                }
                LogiaInteraction::Resisted => {
                    return LogiaDamageOutcome {
                        final_damage: raw_damage * 0.25,
                        passed_through: false,
                        carved_hole: false,
                        bypass: None,
                        solid_shattered: false,
                        stagger_ticks: 0,
                    };
                }
                LogiaInteraction::Neutral => {}
            }
        }

        let suppressed = data.state == LogiaState::Suppressed
            || Self::is_suppressed_by_counter(profile, data, now);
        if suppressed || !logia_cfg.intangibility_enabled || !data.intangibility_toggled {
            return LogiaDamageOutcome {
                final_damage: raw_damage,
                passed_through: false,
                carved_hole: false,
                bypass,
                solid_shattered: false,
                stagger_ticks: 0,
            };
        }

        // Explicit bypass (Seastone weapon, Armament Haki, etc.).
        if let Some(bp) = bypass {
            let allow = match bp.source {
                LogiaBypassSource::SeastoneWeapon => logia_cfg.seastone_weapons_bypass,
                LogiaBypassSource::ArmamentHaki => logia_cfg.armament_bypass,
                _ => true,
            };
            if allow && bp.fraction > 0.0 {
                return LogiaDamageOutcome {
                    final_damage: raw_damage * bp.fraction.clamp(0.0, 1.0),
                    passed_through: false,
                    carved_hole: false,
                    bypass: Some(bp),
                    solid_shattered: false,
                    stagger_ticks: 0,
                };
            }
        }

        // SolidBody (Hie-Hie ice plating) absorbs hits into plating & cracks instead of pure ghosting.
        if let Some(solid_spec) = profile.solid_body {
            if data.state != LogiaState::ElementalBody {
                let mult = match category {
                    DamageCategory::Fire => solid_spec.mult_heat,
                    DamageCategory::Explosion => solid_spec.mult_explosion,
                    DamageCategory::Melee => solid_spec.mult_shatter,
                    DamageCategory::Projectile => solid_spec.mult_chip,
                    _ => 1.0,
                };
                let (hp_dmg, shattered, stagger) = data.solid_state.absorb_hit(
                    solid_spec,
                    player.mastery,
                    limb_index,
                    raw_damage,
                    mult,
                    now,
                );
                return LogiaDamageOutcome {
                    final_damage: hp_dmg,
                    passed_through: hp_dmg <= 0.0,
                    carved_hole: false,
                    bypass: None,
                    solid_shattered: shattered,
                    stagger_ticks: stagger,
                };
            }
        }

        let rule = match (profile.hit_rule(category), data.state) {
            (HitRule::Partial, LogiaState::ElementalBody) => HitRule::Pass,
            (r, _) => r,
        };

        match rule {
            HitRule::Connect => LogiaDamageOutcome {
                final_damage: raw_damage,
                passed_through: false,
                carved_hole: false,
                bypass: None,
                solid_shattered: false,
                stagger_ticks: 0,
            },
            HitRule::Partial => {
                let dmg = raw_damage * logia_cfg.partial_factor;
                data.dissolve_limb(limb_index, now, logia_cfg.reform_ticks);
                LogiaDamageOutcome {
                    final_damage: dmg,
                    passed_through: false,
                    carved_hole: false,
                    bypass: None,
                    solid_shattered: false,
                    stagger_ticks: 0,
                }
            }
            HitRule::Pass => {
                let energy_cost =
                    logia_cfg.negate_energy + logia_cfg.negate_energy_per_damage * raw_damage;
                if energy_cost > 0.0 && !player.spend_energy(energy_cost, creative, core_cfg) {
                    return LogiaDamageOutcome {
                        final_damage: raw_damage,
                        passed_through: false,
                        carved_hole: false,
                        bypass: None,
                        solid_shattered: false,
                        stagger_ticks: 0,
                    };
                }
                if data.state == LogiaState::Solid {
                    data.state = LogiaState::Partial;
                }
                data.carve_hole(
                    hit_entry,
                    hit_exit,
                    0.28,
                    limb_index,
                    now,
                    logia_cfg.reform_ticks,
                );
                LogiaDamageOutcome {
                    final_damage: 0.0,
                    passed_through: true,
                    carved_hole: true,
                    bypass: None,
                    solid_shattered: false,
                    stagger_ticks: 0,
                }
            }
        }
    }

    /// Toggles full `ElementalBody` mode (`LogiaInputPayload`).
    pub fn toggle_elemental_body(
        profile: &LogiaProfile,
        data: &mut LogiaData,
        now: i64,
        suppressed: bool,
        logia_cfg: &LogiaConfig,
    ) -> LogiaState {
        if suppressed || Self::is_suppressed_by_counter(profile, data, now) {
            data.state = LogiaState::Suppressed;
            return data.state;
        }
        if data.state == LogiaState::ElementalBody {
            data.state = LogiaState::Reforming;
            data.reform_until_tick = now + logia_cfg.reform_ticks.max(1) as i64;
        } else {
            data.state = LogiaState::ElementalBody;
            data.awaken_until_tick = now + LOGIA_AWAKEN_TICKS as i64;
            data.dissolved_limbs = limbs::ALL;
        }
        data.state
    }

    /// Launches an elemental dash (`LogiaDash.java`): `flightTicks = clamp(ceil(dist / speed), 3, 40)`.
    #[allow(clippy::too_many_arguments)]
    pub fn start_dash(
        profile: &LogiaProfile,
        player: &mut PlayerFruitData,
        data: &mut LogiaData,
        origin: DVec3,
        target: DVec3,
        now: i64,
        suppressed: bool,
        creative: bool,
        core_cfg: &CoreConfig,
        logia_cfg: &LogiaConfig,
    ) -> Option<ActiveElementalDash> {
        if suppressed
            || Self::is_suppressed_by_counter(profile, data, now)
            || now < data.dash_cooldown_until
            || data.active_dash.is_some()
        {
            return None;
        }
        if !player.spend_energy(logia_cfg.default_dash_energy, creative, core_cfg) {
            return None;
        }
        let max_range = profile.dash_range.max(logia_cfg.default_dash_range) as f64;
        let speed = profile.dash_speed.max(logia_cfg.default_dash_speed) as f64;
        let delta = target - origin;
        let dist = delta.length().min(max_range);
        if dist < 0.25 {
            return None;
        }
        let clamped_end = origin + delta.normalize_or_zero() * dist;
        let flight_ticks = ((dist / speed.max(0.1)).ceil() as i32).clamp(3, 40);
        let dash = ActiveElementalDash {
            start: origin.to_array(),
            end: clamped_end.to_array(),
            started_tick: now,
            duration_ticks: flight_ticks,
        };
        data.active_dash = Some(dash);
        data.dash_cooldown_until = now + logia_cfg.default_dash_cooldown as i64;
        data.state = LogiaState::ElementalBody;
        data.dissolved_limbs = limbs::ALL;
        Some(dash)
    }

    /// Advances Logia reform, hole refill, counters, elemental drain, dash position, and SolidBody.
    /// Returns `(optional_dash_position, env_regen_multiplier)`.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        profile: &LogiaProfile,
        player: &mut PlayerFruitData,
        data: &mut LogiaData,
        now: i64,
        globally_suppressed: bool,
        in_source_env: bool,
        in_opposed_env: bool,
        in_rain: bool,
        creative: bool,
        core_cfg: &CoreConfig,
        logia_cfg: &LogiaConfig,
    ) -> (Option<DVec3>, f32) {
        // Prune expired counters and holes.
        data.active_counters.retain(|_, until| now < *until);
        data.holes.retain(|h| !h.is_expired(now));

        // Reform individual dissolved limbs once timers pass.
        if data.state != LogiaState::ElementalBody {
            for idx in 0..limbs::COUNT {
                if (data.dissolved_limbs & limbs::bit(idx)) != 0
                    && now >= data.limb_reform_until[idx]
                {
                    data.dissolved_limbs &= !limbs::bit(idx);
                }
            }
            if data.state == LogiaState::Partial
                && data.dissolved_limbs == 0
                && data.holes.is_empty()
            {
                data.state = LogiaState::Solid;
            }
            if data.state == LogiaState::Partial
                && now.saturating_sub(data.last_hit_tick) >= LOGIA_RELEASE_AFTER_TICKS as i64
            {
                data.dissolved_limbs = 0;
                data.holes.clear();
                data.state = LogiaState::Solid;
            }
        }

        // Tick SolidBody plating regrowth (Hie-Hie).
        if let Some(solid_spec) = profile.solid_body {
            data.solid_state.tick(solid_spec, player.mastery, now);
        }

        // Advance active elemental dash.
        let mut dash_pos = None;
        if let Some(dash) = data.active_dash {
            dash_pos = Some(dash.position(now));
            if dash.is_finished(now) {
                data.active_dash = None;
                data.arrival_grace_until = now + LOGIA_ARRIVAL_GRACE_TICKS as i64;
                data.state = LogiaState::Reforming;
                data.reform_until_tick = now + logia_cfg.reform_ticks.max(1) as i64;
            }
        }

        // Handle global or counter suppression.
        let counter_suppressed = Self::is_suppressed_by_counter(profile, data, now);
        if globally_suppressed || counter_suppressed {
            data.state = LogiaState::Suppressed;
            data.dissolved_limbs = 0;
            data.active_dash = None;
        } else if data.state == LogiaState::Suppressed {
            data.state = LogiaState::Solid;
        }

        // Complete reforming transition.
        if data.state == LogiaState::Reforming && now >= data.reform_until_tick {
            data.state = LogiaState::Solid;
            data.dissolved_limbs = 0;
        }

        // Drain energy while in ElementalBody.
        if data.state == LogiaState::ElementalBody && data.active_dash.is_none() {
            let drain = profile.elemental_drain_per_second / 20.0;
            if !player.spend_energy(drain, creative, core_cfg) {
                data.state = LogiaState::Reforming;
                data.reform_until_tick = now + logia_cfg.reform_ticks.max(1) as i64;
            }
        }

        // Compute environment regen multiplier.
        let opposed = in_opposed_env || (profile.environment.rain_opposes && in_rain);
        let regen_mult = if opposed {
            profile.environment.opposed_regen_mult
        } else if in_source_env {
            profile.environment.source_regen_mult
        } else {
            1.0
        };

        (dash_pos, regen_mult)
    }
}
