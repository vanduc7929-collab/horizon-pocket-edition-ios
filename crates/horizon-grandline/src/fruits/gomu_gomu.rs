//! Gomu Gomu no Mi (Gum-Gum Fruit) — 1:1 port of `com.devilfruits.fruits.gomu_gomu`.
//!
//! Includes Base (`pistol`, `bazooka`, `rocket`, `axe`, `rifle`, `gatling`),
//! Gear 2 (`jet_pistol`, `jet_bazooka`, `jet_rocket`, `jet_axe`, `jet_rifle`,
//! `jet_gatling`, `soru`, `gear_second`), Gear 3 (`gear_third`, `gigant_pistol`,
//! `gigant_axe`, `gigant_bazooka`, `gigant_rifle`), `balloon` bounce/deflect,
//! `StrikeKind` / `StrikePath` stretched limb kinematics & recoil, `RubberBody`
//! blunt/fall/lightning/bullet immunities, and Gear 2 steam overheat/exhaustion.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::gear::{Gear2Config, GearId};
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Exact Java constants (`GomuGomu.java`, `ability/*.java`, `gear2/*.java`)
// ---------------------------------------------------------------------------

/// `GomuGomu.java` rubber body settings.
pub const GOMU_BULLET_HOLD_TICKS: i32 = 20;
pub const GOMU_RETURNED_BULLET_SPEED: f64 = 0.8;
pub const GOMU_RETURNED_BULLET_DAMAGE_MULTIPLIER: f32 = 1.0;
pub const GOMU_RUBBER_LANDING_HEIGHT: f32 = 4.0;

/// `Soru.java` constants.
pub const SORU_DISTANCE: f64 = 14.0;
pub const SORU_TICKS: i32 = 2;
pub const SORU_COOLDOWN_TICKS: i32 = 40;
pub const SORU_ENERGY: f32 = 6.0;
pub const SORU_HUNGER: f32 = 1.0;
pub const SORU_CHAIN_TICKS: i64 = 20;
pub const SORU_CHAIN_DAMAGE_BONUS: f32 = 0.25;

/// Base ability constants (`PistolAbility`, `BazookaAbility`, `RocketAbility`, `AxeAbility`, `RifleAbility`, `GatlingAbility`).
pub const PISTOL_ENERGY: f32 = 8.0;
pub const PISTOL_COOLDOWN_TICKS: i32 = 100;
pub const PISTOL_UNLOCK_MASTERY: u32 = 1;
pub const PISTOL_DAMAGE: f32 = 6.0;
pub const PISTOL_RANGE: f32 = 10.0;
pub const PISTOL_KNOCKBACK: f32 = 1.6;

pub const BAZOOKA_ENERGY: f32 = 18.0;
pub const BAZOOKA_COOLDOWN_TICKS: i32 = 200;
pub const BAZOOKA_UNLOCK_MASTERY: u32 = 3;
pub const BAZOOKA_DAMAGE: f32 = 18.0;
pub const BAZOOKA_RANGE: f32 = 8.0;
pub const BAZOOKA_KNOCKBACK: f32 = 3.0;
pub const BAZOOKA_MAX_HARDNESS: f32 = 2.0;
pub const BAZOOKA_MAX_RESISTANCE: f32 = 3.0;
pub const BAZOOKA_MAX_BLOCKS: usize = 16;

pub const ROCKET_ENERGY: f32 = 20.0;
pub const ROCKET_COOLDOWN_TICKS: i32 = 200;
pub const ROCKET_UNLOCK_MASTERY: u32 = 4;
pub const ROCKET_DAMAGE: f32 = 8.0;
pub const ROCKET_RANGE: f32 = 20.0;
pub const ROCKET_KNOCKBACK: f32 = 1.2;
pub const ROCKET_PULL_ACCEL: f64 = 1.2;
pub const ROCKET_MAX_PULL_SPEED: f64 = 4.4;
pub const ROCKET_MAX_PULL_TICKS: i32 = 40;
pub const ROCKET_RELEASE_DISTANCE: f64 = 2.0;
pub const ROCKET_ARRIVAL_HOP: f64 = 1.1;
pub const ROCKET_ARRIVAL_CARRY: f64 = 0.6;
pub const ROCKET_MOMENTUM_KEEP_TICKS: i32 = 10;
pub const ROCKET_SLAM_DAMAGE: f32 = 10.0;

pub const AXE_ENERGY: f32 = 28.0;
pub const AXE_COOLDOWN_TICKS: i32 = 300;
pub const AXE_UNLOCK_MASTERY: u32 = 6;
pub const AXE_DAMAGE: f32 = 16.0;
pub const AXE_RANGE: f32 = 7.0;
pub const AXE_KNOCKBACK: f32 = 1.6;
pub const AXE_RADIUS: f64 = 7.0;
pub const AXE_EDGE_FACTOR: f32 = 0.3;
pub const AXE_DISTANCE: f64 = 2.5;
pub const AXE_CRATER_RADIUS: f64 = 2.0;
pub const AXE_MAX_HARDNESS: f32 = 5.0;
pub const AXE_MAX_BLOCKS: usize = 48;

pub const RIFLE_ENERGY: f32 = 22.0;
pub const RIFLE_COOLDOWN_TICKS: i32 = 300;
pub const RIFLE_UNLOCK_MASTERY: u32 = 7;
pub const RIFLE_DAMAGE: f32 = 36.0;
pub const RIFLE_RANGE: f32 = 12.0;
pub const RIFLE_KNOCKBACK: f32 = 1.4;
pub const RIFLE_MAX_PIERCE: usize = 3;
pub const RIFLE_MAX_HARDNESS: f32 = 2.0;
pub const RIFLE_MAX_RESISTANCE: f32 = 6.0;
pub const RIFLE_MAX_BLOCKS: usize = 24;

pub const GATLING_ENERGY: f32 = 40.0;
pub const GATLING_COOLDOWN_TICKS: i32 = 800;
pub const GATLING_UNLOCK_MASTERY: u32 = 10;
pub const GATLING_DAMAGE: f32 = 8.0;
pub const GATLING_RANGE: f32 = 4.0;
pub const GATLING_KNOCKBACK: f32 = 2.4;
pub const GATLING_MAX_CHANNEL_TICKS: i32 = 120;
pub const GATLING_HIT_DAMAGE: f32 = 1.2;
pub const GATLING_DRAIN_PER_SECOND: f32 = 12.0;
pub const GATLING_SLOW: f32 = 0.7;
pub const GATLING_PUNCHES_PER_TICK: usize = 2;

/// Gear 2 (`jet_*`) ability constants (`JetPistolAbility`, `JetBazookaAbility`, `JetRocketAbility`, `JetAxeAbility`, `JetRifleAbility`, `JetGatlingAbility`).
pub const JET_PISTOL_ENERGY: f32 = 12.0;
pub const JET_PISTOL_COOLDOWN_TICKS: i32 = 50;
pub const JET_PISTOL_UNLOCK_MASTERY: u32 = 1;
pub const JET_PISTOL_DAMAGE: f32 = 12.0;
pub const JET_PISTOL_RANGE: f32 = 33.0;
pub const JET_PISTOL_KNOCKBACK: f32 = 3.2;

pub const JET_BAZOOKA_ENERGY: f32 = 26.0;
pub const JET_BAZOOKA_COOLDOWN_TICKS: i32 = 120;
pub const JET_BAZOOKA_UNLOCK_MASTERY: u32 = 3;
pub const JET_BAZOOKA_DAMAGE: f32 = 36.0;
pub const JET_BAZOOKA_RANGE: f32 = 24.0;
pub const JET_BAZOOKA_KNOCKBACK: f32 = 5.0;
pub const JET_BAZOOKA_MAX_BLOCKS: usize = 24;
pub const JET_BAZOOKA_AIR_RECOIL: f64 = 1.1;

pub const JET_ROCKET_ENERGY: f32 = 30.0;
pub const JET_ROCKET_COOLDOWN_TICKS: i32 = 100;
pub const JET_ROCKET_UNLOCK_MASTERY: u32 = 4;
pub const JET_ROCKET_DAMAGE: f32 = 16.0;
pub const JET_ROCKET_RANGE: f32 = 60.0;
pub const JET_ROCKET_KNOCKBACK: f32 = 2.0;
pub const JET_ROCKET_PULL_ACCEL: f64 = 2.5;
pub const JET_ROCKET_MAX_PULL_SPEED: f64 = 7.0;
pub const JET_ROCKET_RELEASE_DISTANCE: f64 = 3.0;
pub const JET_ROCKET_ARRIVAL_HOP: f64 = 1.0;
pub const JET_ROCKET_ARRIVAL_CARRY: f64 = 0.8;
pub const JET_ROCKET_SLAM_DAMAGE: f32 = 20.0;
pub const JET_ROCKET_ARRIVAL_STRIKE_RADIUS: f64 = 2.5;

pub const JET_AXE_ENERGY: f32 = 40.0;
pub const JET_AXE_COOLDOWN_TICKS: i32 = 200;
pub const JET_AXE_UNLOCK_MASTERY: u32 = 6;
pub const JET_AXE_DAMAGE: f32 = 32.0;
pub const JET_AXE_RANGE: f32 = 18.0;
pub const JET_AXE_KNOCKBACK: f32 = 2.4;
pub const JET_AXE_RADIUS: f64 = 18.0;
pub const JET_AXE_CRATER_RADIUS: f64 = 2.5;
pub const JET_AXE_MAX_BLOCKS: usize = 64;

pub const JET_RIFLE_ENERGY: f32 = 32.0;
pub const JET_RIFLE_COOLDOWN_TICKS: i32 = 150;
pub const JET_RIFLE_UNLOCK_MASTERY: u32 = 7;
pub const JET_RIFLE_DAMAGE: f32 = 72.0;
pub const JET_RIFLE_RANGE: f32 = 30.0;
pub const JET_RIFLE_KNOCKBACK: f32 = 2.4;
pub const JET_RIFLE_MAX_PIERCE: usize = 5;
pub const JET_RIFLE_MAX_HARDNESS: f32 = 3.0;
pub const JET_RIFLE_MAX_BLOCKS: usize = 40;

pub const JET_GATLING_ENERGY: f32 = 50.0;
pub const JET_GATLING_COOLDOWN_TICKS: i32 = 800;
pub const JET_GATLING_UNLOCK_MASTERY: u32 = 10;
pub const JET_GATLING_DAMAGE: f32 = 20.0;
pub const JET_GATLING_RANGE: f32 = 10.5;
pub const JET_GATLING_KNOCKBACK: f32 = 3.5;
pub const JET_GATLING_MAX_CHANNEL_TICKS: i32 = 160;
pub const JET_GATLING_HIT_DAMAGE: f32 = 1.6;
pub const JET_GATLING_DRAIN_PER_SECOND: f32 = 18.0;
pub const JET_GATLING_PUNCHES_PER_TICK: usize = 4;

/// Gear 3 (`gigant_*`) and Balloon (`balloon`) technique constants.
pub const BALLOON_ENERGY: f32 = 15.0;
pub const BALLOON_DRAIN_PER_SECOND: f32 = 4.0;
pub const BALLOON_COOLDOWN_TICKS: i32 = 120;
pub const BALLOON_UNLOCK_MASTERY: u32 = 3;
pub const BALLOON_DEFLECT_RADIUS: f64 = 3.5;
pub const BALLOON_BOUNCE_KNOCKBACK: f32 = 1.8;

pub const GIGANT_PISTOL_ENERGY: f32 = 38.0;
pub const GIGANT_PISTOL_COOLDOWN_TICKS: i32 = 260;
pub const GIGANT_PISTOL_UNLOCK_MASTERY: u32 = 7;
pub const GIGANT_PISTOL_DAMAGE: f32 = 42.0;
pub const GIGANT_PISTOL_RANGE: f32 = 16.0;
pub const GIGANT_PISTOL_KNOCKBACK: f32 = 3.8;

pub const GIGANT_AXE_ENERGY: f32 = 46.0;
pub const GIGANT_AXE_COOLDOWN_TICKS: i32 = 340;
pub const GIGANT_AXE_UNLOCK_MASTERY: u32 = 8;
pub const GIGANT_AXE_DAMAGE: f32 = 48.0;
pub const GIGANT_AXE_RANGE: f32 = 14.0;
pub const GIGANT_AXE_KNOCKBACK: f32 = 3.4;

pub const GIGANT_BAZOOKA_ENERGY: f32 = 44.0;
pub const GIGANT_BAZOOKA_COOLDOWN_TICKS: i32 = 320;
pub const GIGANT_BAZOOKA_UNLOCK_MASTERY: u32 = 8;
pub const GIGANT_BAZOOKA_DAMAGE: f32 = 45.0;
pub const GIGANT_BAZOOKA_RANGE: f32 = 13.0;
pub const GIGANT_BAZOOKA_KNOCKBACK: f32 = 4.6;

pub const GIGANT_RIFLE_ENERGY: f32 = 52.0;
pub const GIGANT_RIFLE_COOLDOWN_TICKS: i32 = 380;
pub const GIGANT_RIFLE_UNLOCK_MASTERY: u32 = 9;
pub const GIGANT_RIFLE_DAMAGE: f32 = 64.0;
pub const GIGANT_RIFLE_RANGE: f32 = 16.0;
pub const GIGANT_RIFLE_KNOCKBACK: f32 = 3.6;

/// Canonical Gomu-Gomu VFX identifiers (`GomuFx.java`).
pub mod gomu_fx {
    pub const STRIKE: &str = "devilfruits:gomu_gomu/strike";
    pub const STRIKE_HIT: &str = "devilfruits:gomu_gomu/strike_hit";
    pub const ROCKET_ANCHOR: &str = "devilfruits:gomu_gomu/rocket_anchor";
    pub const ROCKET_RELEASE: &str = "devilfruits:gomu_gomu/rocket_release";
    pub const ROCKET_SLAM: &str = "devilfruits:gomu_gomu/rocket_slam";
    pub const AXE: &str = "devilfruits:gomu_gomu/axe";
    pub const AXE_IMPACT: &str = "devilfruits:gomu_gomu/axe_impact";
    pub const GATLING: &str = "devilfruits:gomu_gomu/gatling";
    pub const GATLING_FINISH: &str = "devilfruits:gomu_gomu/gatling_finish";
    pub const GEAR2_ON: &str = "devilfruits:gomu_gomu/gear2_on";
    pub const GEAR2_OFF: &str = "devilfruits:gomu_gomu/gear2_off";
    pub const SORU: &str = "devilfruits:gomu_gomu/soru";
    pub const SORU_CHAIN: &str = "devilfruits:gomu_gomu/soru_chain";
    pub const JET_GATLING: &str = "devilfruits:gomu_gomu/jet_gatling";
    pub const JET_GATLING_FINISH: &str = "devilfruits:gomu_gomu/jet_gatling_finish";
}

/// Exact stretched limb strike profiles (`com.devilfruits.fruits.gomu_gomu.strike.StrikeKind`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeKindSpec {
    pub code: i32,
    pub feint: i32,
    pub coil: i32,
    pub back: f64,
    pub speed: f64,
    pub hold: i32,
    pub retract: i32,
    pub twist: f32,
    pub radius: f64,
    pub two_arms: bool,
    pub is_jet: bool,
}

impl StrikeKindSpec {
    pub const PISTOL: Self = Self {
        code: 0,
        feint: 2,
        coil: 5,
        back: 2.5,
        speed: 4.0,
        hold: 2,
        retract: 5,
        twist: 0.0,
        radius: 0.42,
        two_arms: false,
        is_jet: false,
    };
    pub const BAZOOKA: Self = Self {
        code: 1,
        feint: 2,
        coil: 7,
        back: 2.2,
        speed: 4.0,
        hold: 3,
        retract: 6,
        twist: 0.0,
        radius: 0.55,
        two_arms: true,
        is_jet: false,
    };
    pub const RIFLE: Self = Self {
        code: 2,
        feint: 2,
        coil: 7,
        back: 2.5,
        speed: 4.0,
        hold: 2,
        retract: 6,
        twist: 3.5,
        radius: 0.32,
        two_arms: false,
        is_jet: false,
    };
    pub const ROCKET: Self = Self {
        code: 3,
        feint: 1,
        coil: 5,
        back: 1.6,
        speed: 3.5,
        hold: 3,
        retract: 6,
        twist: 0.0,
        radius: 0.45,
        two_arms: true,
        is_jet: false,
    };
    pub const JET_PISTOL: Self = Self {
        code: 4,
        feint: 0,
        coil: 1,
        back: 0.5,
        speed: 11.0,
        hold: 1,
        retract: 2,
        twist: 0.0,
        radius: 0.42,
        two_arms: false,
        is_jet: true,
    };
    pub const JET_BAZOOKA: Self = Self {
        code: 5,
        feint: 0,
        coil: 1,
        back: 0.5,
        speed: 10.0,
        hold: 2,
        retract: 2,
        twist: 0.0,
        radius: 0.55,
        two_arms: true,
        is_jet: true,
    };
    pub const JET_RIFLE: Self = Self {
        code: 6,
        feint: 0,
        coil: 1,
        back: 0.7,
        speed: 11.0,
        hold: 1,
        retract: 2,
        twist: 3.5,
        radius: 0.32,
        two_arms: false,
        is_jet: true,
    };
    pub const JET_ROCKET: Self = Self {
        code: 7,
        feint: 0,
        coil: 2,
        back: 0.8,
        speed: 9.0,
        hold: 2,
        retract: 3,
        twist: 0.0,
        radius: 0.45,
        two_arms: true,
        is_jet: true,
    };

    pub const fn release_tick(self) -> i32 {
        self.feint + self.coil
    }

    pub fn end_tick(self, end: f64) -> f64 {
        (self.release_tick() as f64) + (end + self.back) / self.speed
    }

    pub fn done_tick(self, end: f64) -> f64 {
        self.end_tick(end) + (self.hold + self.retract) as f64
    }

    /// Exact limb stretch & recoil trajectory (`StrikePath.distance(kind, t, end)`).
    pub fn distance_at(self, t: f64, end: f64) -> f64 {
        if t < (self.feint as f64) && self.feint > 0 {
            let u = (t / (self.feint as f64)).clamp(0.0, 1.0);
            return 0.35 * (u * u * (3.0 - 2.0 * u));
        }
        let release = self.release_tick() as f64;
        if t < release && self.coil > 0 {
            let u = ((t - (self.feint as f64)) / (self.coil as f64)).clamp(0.0, 1.0);
            let k = 1.0 - (1.0 - u).powi(3);
            return 0.35 + (-self.back - 0.35) * k;
        }
        let shot = -self.back + self.speed * (t - release);
        let t_end = self.end_tick(end);
        if t < t_end {
            return shot;
        }
        if t < t_end + (self.hold as f64) {
            return end;
        }
        let u = ((t - t_end - (self.hold as f64)) / (self.retract as f64).max(1.0)).clamp(0.0, 1.0);
        let c = 0.9;
        let back = 1.0 + (c + 1.0) * (u - 1.0).powi(3) + c * (u - 1.0).powi(2);
        end * (1.0 - back)
    }
}

/// Rubber body immunity classification (`com.devilfruits.fruits.gomu_gomu.rubber.RubberBody.Shrug`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RubberShrug {
    Blunt,
    Fall,
    Lightning,
    BulletCatch,
}

/// Gomu Gomu no Mi module (`com.devilfruits.fruits.gomu_gomu.GomuGomu`).
#[derive(Clone, Copy, Debug, Default)]
pub struct GomuGomuModule;

impl GomuGomuModule {
    pub const fn new() -> Self {
        Self
    }

    /// Evaluates whether a non-Haki attack or environmental damage is shrugged off by a Gomu-Gomu user
    /// (`RubberBody.shrugs` + `BulletCatch.catches`).
    pub fn evaluate_rubber_immunity(
        suppressed: bool,
        is_blunt: bool,
        is_fall: bool,
        is_electric: bool,
        is_bullet: bool,
        has_haki_or_seastone_bypass: bool,
    ) -> Option<RubberShrug> {
        if suppressed {
            return None;
        }
        if is_electric {
            return Some(RubberShrug::Lightning);
        }
        if is_fall {
            return Some(RubberShrug::Fall);
        }
        if !has_haki_or_seastone_bypass && is_bullet {
            return Some(RubberShrug::BulletCatch);
        }
        if !has_haki_or_seastone_bypass && is_blunt {
            return Some(RubberShrug::Blunt);
        }
        None
    }
}

impl DevilFruitModule for GomuGomuModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::GomuGomu
    }

    fn primary_damage_type(&self) -> GrandLineDamageType {
        GrandLineDamageType::DevilFruit
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let base_only = vec!["base".to_string()];
        let gear2_only = vec!["gear_2".to_string()];
        let all_forms = vec!["base".to_string(), "gear_2".to_string()];

        vec![
            // --- Base Form Abilities ---
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "pistol"),
                display_name: "Gomu Gomu no Pistol".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: PISTOL_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: PISTOL_COOLDOWN_TICKS,
                    unlock_mastery: PISTOL_UNLOCK_MASTERY,
                    damage: PISTOL_DAMAGE,
                    range: PISTOL_RANGE,
                    knockback: PISTOL_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "bazooka"),
                display_name: "Gomu Gomu no Bazooka".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: BAZOOKA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: BAZOOKA_COOLDOWN_TICKS,
                    unlock_mastery: BAZOOKA_UNLOCK_MASTERY,
                    damage: BAZOOKA_DAMAGE,
                    range: BAZOOKA_RANGE,
                    knockback: BAZOOKA_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "rocket"),
                display_name: "Gomu Gomu no Rocket".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: ROCKET_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: ROCKET_COOLDOWN_TICKS,
                    unlock_mastery: ROCKET_UNLOCK_MASTERY,
                    damage: ROCKET_DAMAGE,
                    range: ROCKET_RANGE,
                    knockback: ROCKET_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "axe"),
                display_name: "Gomu Gomu no Axe".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: AXE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: AXE_COOLDOWN_TICKS,
                    unlock_mastery: AXE_UNLOCK_MASTERY,
                    damage: AXE_DAMAGE,
                    range: AXE_RANGE,
                    knockback: AXE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "rifle"),
                display_name: "Gomu Gomu no Rifle".into(),
                cast_type: CastType::Instant,
                allowed_forms: base_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: RIFLE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: RIFLE_COOLDOWN_TICKS,
                    unlock_mastery: RIFLE_UNLOCK_MASTERY,
                    damage: RIFLE_DAMAGE,
                    range: RIFLE_RANGE,
                    knockback: RIFLE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gatling"),
                display_name: "Gomu Gomu no Gatling".into(),
                cast_type: CastType::Held,
                allowed_forms: base_only,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GATLING_ENERGY,
                    energy_per_second: GATLING_DRAIN_PER_SECOND,
                    cooldown_ticks: GATLING_COOLDOWN_TICKS,
                    unlock_mastery: GATLING_UNLOCK_MASTERY,
                    damage: GATLING_DAMAGE,
                    range: GATLING_RANGE,
                    knockback: GATLING_KNOCKBACK,
                    max_channel_ticks: GATLING_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            // --- Gear 2 (`jet_*`) Abilities ---
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_pistol"),
                display_name: "Gomu Gomu no Jet Pistol".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_PISTOL_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: JET_PISTOL_COOLDOWN_TICKS,
                    unlock_mastery: JET_PISTOL_UNLOCK_MASTERY,
                    damage: JET_PISTOL_DAMAGE,
                    range: JET_PISTOL_RANGE,
                    knockback: JET_PISTOL_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_bazooka"),
                display_name: "Gomu Gomu no Jet Bazooka".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_BAZOOKA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: JET_BAZOOKA_COOLDOWN_TICKS,
                    unlock_mastery: JET_BAZOOKA_UNLOCK_MASTERY,
                    damage: JET_BAZOOKA_DAMAGE,
                    range: JET_BAZOOKA_RANGE,
                    knockback: JET_BAZOOKA_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_rocket"),
                display_name: "Gomu Gomu no Jet Rocket".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_ROCKET_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: JET_ROCKET_COOLDOWN_TICKS,
                    unlock_mastery: JET_ROCKET_UNLOCK_MASTERY,
                    damage: JET_ROCKET_DAMAGE,
                    range: JET_ROCKET_RANGE,
                    knockback: JET_ROCKET_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_axe"),
                display_name: "Gomu Gomu no Jet Axe".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_AXE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: JET_AXE_COOLDOWN_TICKS,
                    unlock_mastery: JET_AXE_UNLOCK_MASTERY,
                    damage: JET_AXE_DAMAGE,
                    range: JET_AXE_RANGE,
                    knockback: JET_AXE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_rifle"),
                display_name: "Gomu Gomu no Jet Rifle".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_RIFLE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: JET_RIFLE_COOLDOWN_TICKS,
                    unlock_mastery: JET_RIFLE_UNLOCK_MASTERY,
                    damage: JET_RIFLE_DAMAGE,
                    range: JET_RIFLE_RANGE,
                    knockback: JET_RIFLE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "jet_gatling"),
                display_name: "Gomu Gomu no Jet Gatling".into(),
                cast_type: CastType::Held,
                allowed_forms: gear2_only.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: JET_GATLING_ENERGY,
                    energy_per_second: JET_GATLING_DRAIN_PER_SECOND,
                    cooldown_ticks: JET_GATLING_COOLDOWN_TICKS,
                    unlock_mastery: JET_GATLING_UNLOCK_MASTERY,
                    damage: JET_GATLING_DAMAGE,
                    range: JET_GATLING_RANGE,
                    knockback: JET_GATLING_KNOCKBACK,
                    max_channel_ticks: JET_GATLING_MAX_CHANNEL_TICKS,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "soru"),
                display_name: "Soru".into(),
                cast_type: CastType::Instant,
                allowed_forms: gear2_only,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: SORU_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: SORU_COOLDOWN_TICKS,
                    unlock_mastery: 5,
                    damage: 0.0,
                    range: SORU_DISTANCE as f32,
                    knockback: 0.45,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            // --- Gear Switches & Balloon & Gear 3 (`gigant_*`) ---
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gear_second"),
                display_name: "Gear Second".into(),
                cast_type: CastType::FormSwitch,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: 15.0,
                    energy_per_second: 1.0,
                    cooldown_ticks: 40,
                    unlock_mastery: 5,
                    damage: 0.0,
                    range: 0.0,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gear_third"),
                display_name: "Gear Third".into(),
                cast_type: CastType::Instant,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: 25.0,
                    energy_per_second: 0.0,
                    cooldown_ticks: 60,
                    unlock_mastery: 7,
                    damage: 0.0,
                    range: 0.0,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "balloon"),
                display_name: "Gomu Gomu no Fusen".into(),
                cast_type: CastType::Toggle,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: BALLOON_ENERGY,
                    energy_per_second: BALLOON_DRAIN_PER_SECOND,
                    cooldown_ticks: BALLOON_COOLDOWN_TICKS,
                    unlock_mastery: BALLOON_UNLOCK_MASTERY,
                    damage: 6.0,
                    range: BALLOON_DEFLECT_RADIUS as f32,
                    knockback: BALLOON_BOUNCE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gigant_pistol"),
                display_name: "Gomu Gomu no Gigant Pistol".into(),
                cast_type: CastType::Instant,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GIGANT_PISTOL_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GIGANT_PISTOL_COOLDOWN_TICKS,
                    unlock_mastery: GIGANT_PISTOL_UNLOCK_MASTERY,
                    damage: GIGANT_PISTOL_DAMAGE,
                    range: GIGANT_PISTOL_RANGE,
                    knockback: GIGANT_PISTOL_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gigant_axe"),
                display_name: "Gomu Gomu no Gigant Axe".into(),
                cast_type: CastType::Instant,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GIGANT_AXE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GIGANT_AXE_COOLDOWN_TICKS,
                    unlock_mastery: GIGANT_AXE_UNLOCK_MASTERY,
                    damage: GIGANT_AXE_DAMAGE,
                    range: GIGANT_AXE_RANGE,
                    knockback: GIGANT_AXE_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gigant_bazooka"),
                display_name: "Gomu Gomu no Gigant Bazooka".into(),
                cast_type: CastType::Instant,
                allowed_forms: all_forms.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GIGANT_BAZOOKA_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GIGANT_BAZOOKA_COOLDOWN_TICKS,
                    unlock_mastery: GIGANT_BAZOOKA_UNLOCK_MASTERY,
                    damage: GIGANT_BAZOOKA_DAMAGE,
                    range: GIGANT_BAZOOKA_RANGE,
                    knockback: GIGANT_BAZOOKA_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::GomuGomu, "gigant_rifle"),
                display_name: "Gomu Gomu no Gigant Rifle".into(),
                cast_type: CastType::Instant,
                allowed_forms: all_forms,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: GIGANT_RIFLE_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: GIGANT_RIFLE_COOLDOWN_TICKS,
                    unlock_mastery: GIGANT_RIFLE_UNLOCK_MASTERY,
                    damage: GIGANT_RIFLE_DAMAGE,
                    range: GIGANT_RIFLE_RANGE,
                    knockback: GIGANT_RIFLE_KNOCKBACK,
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
        let flat = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let flat = if flat.length_squared() < 1e-8 {
            DVec3::Z
        } else {
            flat
        };
        let now = ctx.ability_ctx.tick;
        let g2_cfg = Gear2Config::default();

        match event {
            AbilityEvent::FormSwitchRequested { ability }
            | AbilityEvent::CastInstant { ability, .. }
                if ability == "gear_second" =>
            {
                let entering_g2 = ctx.player.active_form != "gear_2";
                if entering_g2 {
                    res.form_switch_to = Some("gear_2".into());
                    ctx.gear_state.active_gear = GearId::new("gear_2");
                    ctx.gear_state.gear2_active = true;
                    res.animation_clip = Some(
                        if ctx.gear_state.gear2_adaptation.has_quick_enter(&g2_cfg) {
                            "devilfruits:gomu_gomu/gear2_enter_quick".into()
                        } else {
                            "devilfruits:gomu_gomu/gear2_enter".into()
                        },
                    );
                    ctx.world_fx
                        .emit_fx(gomu_fx::GEAR2_ON, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                } else {
                    res.form_switch_to = Some("base".into());
                    ctx.gear_state.active_gear = GearId::new("gear_1");
                    ctx.gear_state.gear2_active = false;
                    res.animation_clip = Some("devilfruits:gomu_gomu/gear2_exit".into());
                    ctx.world_fx
                        .emit_fx(gomu_fx::GEAR2_OFF, 0, eye, DVec3::ZERO, 1.0, now, 0, 64.0);
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "gear_third" => {
                ctx.gear_state.active_gear = GearId::new("gear_3");
                res.animation_clip = Some("devilfruits:gomu_gomu/bone_balloon".into());
            }
            AbilityEvent::CastInstant { ability, .. } if ability == "soru" => {
                ctx.gear_state.on_jet_technique_cast(&g2_cfg);
                let speed = SORU_DISTANCE / (SORU_TICKS.max(1) as f64);
                res.caster_velocity_impulse = Some(flat * speed);
                res.animation_clip = Some("devilfruits:gomu_gomu/soru".into());
                ctx.world_fx.emit_fx(
                    gomu_fx::SORU,
                    0,
                    eye,
                    flat * SORU_DISTANCE,
                    SORU_DISTANCE as f32,
                    now,
                    SORU_TICKS,
                    64.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::Toggled { ability, enabled } if ability == "balloon" => {
                res.cast_result = Some(CastResult::Toggled);
                res.animation_clip = Some(if *enabled {
                    "devilfruits:gomu_gomu/balloon_inflate".into()
                } else {
                    "devilfruits:gomu_gomu/balloon_deflate".into()
                });
            }
            AbilityEvent::ToggleTick { ability } if ability == "balloon" => {
                res.cast_result = Some(CastResult::Toggled);
                let Some(def) = self.ability("balloon") else {
                    return res;
                };
                let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                for target in CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    BALLOON_DEFLECT_RADIUS,
                ) {
                    let away = (target.center - eye).normalize_or_zero();
                    let bounce = away * kb + DVec3::new(0.0, 0.35, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::DevilFruit,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            bounce,
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
            AbilityEvent::CastInstant { ability, .. }
                if matches!(
                    ability.as_str(),
                    "pistol"
                        | "bazooka"
                        | "rifle"
                        | "jet_pistol"
                        | "jet_bazooka"
                        | "jet_rifle"
                        | "gigant_pistol"
                        | "gigant_bazooka"
                        | "gigant_rifle"
                ) =>
            {
                let Some(def) = self.ability(ability) else {
                    return res;
                };
                let is_jet = ability.starts_with("jet_");
                let is_gigant = ability.starts_with("gigant_");
                if is_jet {
                    ctx.gear_state.on_jet_technique_cast(&g2_cfg);
                }

                let kind = match ability.as_str() {
                    "pistol" => StrikeKindSpec::PISTOL,
                    "bazooka" | "gigant_bazooka" => StrikeKindSpec::BAZOOKA,
                    "rifle" | "gigant_rifle" => StrikeKindSpec::RIFLE,
                    "jet_pistol" => StrikeKindSpec::JET_PISTOL,
                    "jet_bazooka" => StrikeKindSpec::JET_BAZOOKA,
                    "jet_rifle" => StrikeKindSpec::JET_RIFLE,
                    _ => StrikeKindSpec::PISTOL,
                };
                let reach = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let max_pierce = match ability.as_str() {
                    "rifle" => RIFLE_MAX_PIERCE,
                    "jet_rifle" | "gigant_rifle" => JET_RIFLE_MAX_PIERCE,
                    _ => 0,
                };
                let capsule_radius = if is_gigant {
                    kind.radius * 3.0
                } else {
                    kind.radius
                };

                // JetBazookaAbility air recoil:
                if ability == "jet_bazooka" && !ctx.on_ground && !ctx.submerged_in_water {
                    res.caster_velocity_impulse =
                        Some(-look * JET_BAZOOKA_AIR_RECOIL + DVec3::new(0.0, 0.15, 0.0));
                }

                res.animation_clip = Some(format!("devilfruits:gomu_gomu/{ability}"));
                ctx.world_fx.emit_fx(
                    gomu_fx::STRIKE,
                    0,
                    eye,
                    look,
                    reach as f32,
                    now,
                    kind.code,
                    64.0,
                );

                let end = eye + look * reach;
                let mut candidates = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    end,
                    capsule_radius,
                );
                candidates.sort_by(|a, b| {
                    a.center
                        .distance_squared(eye)
                        .partial_cmp(&b.center.distance_squared(eye))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let push_dir =
                    DVec3::new(look.x, look.y * 0.4, look.z).normalize_or_zero() * kb
                        + DVec3::new(0.0, 0.3, 0.0);
                for (idx, target) in candidates.into_iter().enumerate() {
                    if idx > max_pierce {
                        break;
                    }
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::DevilFruit,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push_dir,
                            target.knockback_resistance,
                        ),
                        stun_ticks: if is_gigant { 14 } else { 0 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                    ctx.world_fx.emit_fx(
                        gomu_fx::STRIKE_HIT,
                        0,
                        target.center,
                        look,
                        reach as f32,
                        now,
                        kind.code,
                        64.0,
                    );
                }

                // Bazooka & Rifle & Gigant strikes break blocks on impact
                if matches!(
                    ability.as_str(),
                    "bazooka"
                        | "rifle"
                        | "jet_bazooka"
                        | "jet_rifle"
                        | "gigant_pistol"
                        | "gigant_bazooka"
                        | "gigant_rifle"
                ) {
                    let max_blocks = match ability.as_str() {
                        "bazooka" => BAZOOKA_MAX_BLOCKS,
                        "rifle" | "jet_bazooka" => RIFLE_MAX_BLOCKS,
                        "jet_rifle" => JET_RIFLE_MAX_BLOCKS,
                        _ => 32,
                    };
                    ctx.budget.blocks_remaining = ctx.budget.blocks_remaining.min(max_blocks);
                    let break_r = if is_gigant { 2.2 } else { 0.9 };
                    res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                        eye + look * (reach * 0.5).min(4.0),
                        break_r,
                        &mut ctx.budget,
                        false,
                        now,
                        ctx.config,
                        ctx.game_rules,
                        |_pos| ("minecraft:oak_planks".to_string(), 2.0, false),
                    );
                }

                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. }
                if ability == "rocket" || ability == "jet_rocket" =>
            {
                let is_jet = ability == "jet_rocket";
                if is_jet {
                    ctx.gear_state.on_jet_technique_cast(&g2_cfg);
                }
                let Some(def) = self.ability(ability) else {
                    return res;
                };
                let reach = def.stats.scaled_range(mastery, ctx.config, &mods) as f64;
                let dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let max_speed = if is_jet {
                    JET_ROCKET_MAX_PULL_SPEED
                } else {
                    ROCKET_MAX_PULL_SPEED
                };
                let hop = if is_jet {
                    JET_ROCKET_ARRIVAL_HOP
                } else {
                    ROCKET_ARRIVAL_HOP
                };

                res.animation_clip = Some(format!("devilfruits:gomu_gomu/{ability}"));
                let anchor_pos = ctx.ability_ctx.aim_point.unwrap_or(eye + look * reach);
                let pull_dir = (anchor_pos - eye).normalize_or_zero();
                res.caster_velocity_impulse =
                    Some(pull_dir * (max_speed * 0.65) + DVec3::new(0.0, hop * 0.35, 0.0));

                ctx.world_fx.emit_fx(
                    gomu_fx::ROCKET_ANCHOR,
                    0,
                    anchor_pos,
                    look,
                    1.0,
                    now,
                    0,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    gomu_fx::ROCKET_RELEASE,
                    0,
                    eye,
                    pull_dir * max_speed,
                    max_speed as f32,
                    now,
                    0,
                    64.0,
                );

                for target in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    anchor_pos,
                    if is_jet {
                        JET_ROCKET_ARRIVAL_STRIKE_RADIUS
                    } else {
                        1.0
                    },
                ) {
                    let launch = pull_dir * kb + DVec3::new(0.0, 0.35, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: dmg,
                        damage_type: GrandLineDamageType::DevilFruit,
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
                    ctx.world_fx.emit_fx(
                        gomu_fx::ROCKET_SLAM,
                        0,
                        target.center,
                        pull_dir,
                        max_speed as f32,
                        now,
                        0,
                        64.0,
                    );
                }
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::CastInstant { ability, .. }
                if ability == "axe" || ability == "jet_axe" || ability == "gigant_axe" =>
            {
                let is_jet = ability == "jet_axe";
                let is_gigant = ability == "gigant_axe";
                if is_jet {
                    ctx.gear_state.on_jet_technique_cast(&g2_cfg);
                }
                let Some(def) = self.ability(ability) else {
                    return res;
                };
                let stomp_at =
                    DVec3::new(eye.x, eye.y - 1.62, eye.z) + flat * AXE_DISTANCE;
                let radius = if is_jet {
                    JET_AXE_RADIUS
                } else if is_gigant {
                    14.0
                } else {
                    AXE_RADIUS
                };
                let crater_r = if is_jet {
                    JET_AXE_CRATER_RADIUS
                } else if is_gigant {
                    3.5
                } else {
                    AXE_CRATER_RADIUS
                };
                let max_blocks = if is_jet {
                    JET_AXE_MAX_BLOCKS
                } else {
                    AXE_MAX_BLOCKS
                };
                let base_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;

                res.animation_clip = Some(format!("devilfruits:gomu_gomu/{ability}"));
                ctx.world_fx.emit_fx(
                    gomu_fx::AXE,
                    0,
                    stomp_at,
                    flat,
                    5.0,
                    now,
                    if is_jet { 1 } else { 0 },
                    64.0,
                );

                for target in CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    stomp_at,
                    radius,
                ) {
                    let d = target.center.distance(stomp_at);
                    let t = (d / radius.max(0.01)).clamp(0.0, 1.0) as f32;
                    let falloff = 1.0 + (AXE_EDGE_FACTOR - 1.0) * t;
                    let away = DVec3::new(
                        target.center.x - stomp_at.x,
                        0.0,
                        target.center.z - stomp_at.z,
                    )
                    .normalize_or_zero();
                    let away = if away.length_squared() < 1e-4 {
                        flat
                    } else {
                        away
                    };
                    let launch = away * (kb * (falloff as f64))
                        + DVec3::new(0.0, 0.55 * (falloff as f64) + 0.15, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: base_dmg * falloff,
                        damage_type: GrandLineDamageType::DevilFruit,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            launch,
                            target.knockback_resistance,
                        ),
                        stun_ticks: if is_gigant { 16 } else { 0 },
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.budget.blocks_remaining = ctx.budget.blocks_remaining.min(max_blocks);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    stomp_at,
                    crater_r,
                    &mut ctx.budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_pos| ("minecraft:stone".to_string(), 1.5, false),
                );
                ctx.world_fx.emit_fx(
                    gomu_fx::AXE_IMPACT,
                    0,
                    stomp_at,
                    flat,
                    radius as f32,
                    now,
                    res.destroyed_blocks.len() as i32,
                    64.0,
                );
                let shake_radius = (radius * 3.0) as i32;
                ctx.world_fx.emit_shake(stomp_at, 1.2, shake_radius);
                res.screen_shake = Some((1.2, shake_radius));
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::ChannelStarted { ability }
                if ability == "gatling" || ability == "jet_gatling" =>
            {
                res.cast_result = Some(CastResult::Channelling);
                let is_jet = ability == "jet_gatling";
                if is_jet {
                    ctx.gear_state.on_jet_technique_cast(&g2_cfg);
                }
                res.animation_clip = Some(format!("devilfruits:gomu_gomu/{ability}"));
                ctx.world_fx.emit_fx(
                    if is_jet {
                        gomu_fx::JET_GATLING
                    } else {
                        gomu_fx::GATLING
                    },
                    0,
                    eye,
                    look,
                    1.0,
                    now,
                    0,
                    64.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            AbilityEvent::ChannelTick {
                ability,
                elapsed_ticks: _,
            } if ability == "gatling" || ability == "jet_gatling" =>
            {
                res.cast_result = Some(CastResult::Channelling);
                let is_jet = ability == "jet_gatling";
                let reach = if is_jet {
                    JET_GATLING_RANGE as f64
                } else {
                    GATLING_RANGE as f64
                };
                let punch_dmg = if is_jet {
                    JET_GATLING_HIT_DAMAGE * (JET_GATLING_PUNCHES_PER_TICK as f32)
                } else {
                    GATLING_HIT_DAMAGE * (GATLING_PUNCHES_PER_TICK as f32)
                };
                let end = eye + look * reach;
                let push = DVec3::new(look.x, look.y * 0.2, look.z).normalize_or_zero() * 0.06;

                let mut targets =
                    CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, eye, end, 0.45);
                if !is_jet && targets.len() > 1 {
                    targets.sort_by(|a, b| {
                        a.center
                            .distance_squared(eye)
                            .partial_cmp(&b.center.distance_squared(eye))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    targets.truncate(1);
                }
                for target in targets {
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: punch_dmg,
                        damage_type: GrandLineDamageType::DevilFruit,
                        knockback_impulse: push,
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
            }
            AbilityEvent::ChannelEnded {
                ability,
                elapsed_ticks,
            } if ability == "gatling" || ability == "jet_gatling" =>
            {
                let is_jet = ability == "jet_gatling";
                let Some(def) = self.ability(ability) else {
                    return res;
                };
                let reach = if is_jet {
                    JET_GATLING_RANGE as f64
                } else {
                    GATLING_RANGE as f64
                };
                let finisher_dmg = def.stats.scaled_damage(mastery, ctx.config, &mods);
                let kb = def.stats.knockback as f64;
                let end = eye + look * reach;
                let push = DVec3::new(look.x, look.y * 0.2, look.z).normalize_or_zero() * kb
                    + DVec3::new(0.0, 0.45, 0.0);

                for target in
                    CombatTargeting::in_capsule(ctx.nearby_targets, ctx.player_id, eye, end, 0.8)
                {
                    res.hits.push(AbilityHitEffect {
                        target_id: target.id,
                        damage: finisher_dmg,
                        damage_type: GrandLineDamageType::DevilFruit,
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
                res.animation_clip = Some(format!("devilfruits:gomu_gomu/{ability}_finish"));
                ctx.world_fx.emit_fx(
                    if is_jet {
                        gomu_fx::JET_GATLING_FINISH
                    } else {
                        gomu_fx::GATLING_FINISH
                    },
                    0,
                    end,
                    look,
                    1.0,
                    now,
                    *elapsed_ticks,
                    64.0,
                );
                res.vfx_events = ctx.world_fx.drain_vfx();
            }
            _ => {}
        }

        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult::default();
        let g2_cfg = Gear2Config::default();
        // Sync Gear 2 warning & adaptation look level flags (`Gear2.tick` & `Gear2.syncLook`)
        let in_g2 = ctx.player.active_form == "gear_2" || ctx.gear_state.gear2_active;
        let warning_bit = form_flags::fruit_bit(0);
        if in_g2 {
            let max_s = ctx.gear_state.gear2_adaptation.max_active_seconds(&g2_cfg);
            let remaining = max_s - ctx.gear_state.gear2_strain_seconds;
            let warn = remaining <= g2_cfg.warning_seconds;
            ctx.form_runtime.set_flag(warning_bit, warn);
            if ctx.gear_state.gear2_strain_seconds >= max_s {
                ctx.gear_state.gear2_active = false;
                ctx.gear_state.active_gear = GearId::new("gear_1");
                res.form_switch_to = Some("base".into());
                res.animation_clip = Some("devilfruits:gomu_gomu/overexerted".into());
            }
        } else {
            ctx.form_runtime.set_flag(warning_bit, false);
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
        // Gomu-Gomu airborne Rocket / Soru / Gear 2 jump resets anti-cheat above-ground tick counter
        if player.active_form == "gear_2" {
            mods.reset_above_ground_ticks = true;
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
    fn gomu_gomu_base_jet_and_gigant_abilities_match_java() {
        let module = GomuGomuModule::new();
        assert_eq!(module.fruit_id(), FruitId::GomuGomu);

        let pistol = module.ability("pistol").expect("pistol");
        assert_eq!(pistol.stats.energy_cost, 8.0);
        assert_eq!(pistol.stats.cooldown_ticks, 100);
        assert_eq!(pistol.stats.damage, 6.0);
        assert_eq!(pistol.stats.range, 10.0);

        let jet_pistol = module.ability("jet_pistol").expect("jet_pistol");
        assert_eq!(jet_pistol.stats.energy_cost, 12.0);
        assert_eq!(jet_pistol.stats.cooldown_ticks, 50);
        assert_eq!(jet_pistol.stats.damage, 12.0);
        assert_eq!(jet_pistol.stats.range, 33.0);
        assert_eq!(jet_pistol.allowed_forms, vec!["gear_2".to_string()]);

        let jet_gatling = module.ability("jet_gatling").expect("jet_gatling");
        assert_eq!(jet_gatling.cast_type, CastType::Held);
        assert!(jet_gatling.is_ultimate);
        assert_eq!(jet_gatling.stats.max_channel_ticks, 160);

        assert!(module.ability("gear_second").is_some());
        assert!(module.ability("gear_third").is_some());
        assert!(module.ability("gigant_pistol").is_some());
        assert!(module.ability("balloon").is_some());

        // StrikeKind trajectory check
        let d_release = StrikeKindSpec::PISTOL.distance_at(7.0, 10.0);
        assert!((d_release - (-2.5)).abs() < 1e-6);
        let d_peak = StrikeKindSpec::PISTOL.distance_at(StrikeKindSpec::PISTOL.end_tick(10.0), 10.0);
        assert!((d_peak - 10.0).abs() < 1e-6);

        // Rubber immunity check
        assert_eq!(
            GomuGomuModule::evaluate_rubber_immunity(false, true, false, false, false, false),
            Some(RubberShrug::Blunt)
        );
        assert_eq!(
            GomuGomuModule::evaluate_rubber_immunity(false, false, false, true, false, true),
            Some(RubberShrug::Lightning)
        );
        assert_eq!(
            GomuGomuModule::evaluate_rubber_immunity(false, true, false, false, false, true),
            None
        );
    }

    #[test]
    fn gomu_gomu_execution_and_gear2_overheat() {
        let module = GomuGomuModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::GomuGomu),
            mastery: 10,
            energy: 230.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = [make_target(2, DVec3::new(0.0, 64.0, 4.0))];

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
                fruit: FruitId::GomuGomu,
                ability_name: "pistol".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 50,
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

        let pistol_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "pistol".into(),
                mastery: 10,
            },
        );
        assert_eq!(pistol_res.hits.len(), 1);

        // Switch to Gear 2
        let g2_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::FormSwitchRequested {
                ability: "gear_second".into(),
            },
        );
        assert_eq!(g2_res.form_switch_to.as_deref(), Some("gear_2"));
        ctx.player.active_form = "gear_2".into();

        // Jet Bazooka in air gives backward recoil
        let jb_res = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "jet_bazooka".into(),
                mastery: 10,
            },
        );
        assert_eq!(jb_res.hits.len(), 1);
        assert!(jb_res.caster_velocity_impulse.unwrap().z < 0.0);
        assert!(ctx.gear_state.gear2_strain_seconds >= 3.0);

        // Overstrain in Gear 2 forces revert on tick
        ctx.gear_state.gear2_strain_seconds = 100.0;
        let tick_res = module.on_tick(&mut ctx);
        assert_eq!(tick_res.form_switch_to.as_deref(), Some("base"));
        assert!(!ctx.gear_state.gear2_active);
    }
}
