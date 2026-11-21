//! Armament Haki (Busoshoku) progression, 9-part body/weapon coating, Spirit
//! resource management, Logia bypass reach, Ryuo emission/internal destruction,
//! Haki Clashes, and bare-fist combat training.
//!
//! Mirrors `core/haki/{HakiTier,HakiParts,HakiConfig,PlayerHakiData,HakiManager,
//! HakiCombat,HakiClash,HakiTraining}.java`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::logia::{LogiaBypassResult, LogiaBypassSource};

/// Maximum Armament Haki level (`HakiTier.MAX_LEVEL` / `PlayerHakiData`).
pub const HAKI_MAX_LEVEL: u32 = 10;

/// Minimum Ryuo charge fraction required on release (`HakiCombat.MIN_RYUO_FRACTION`).
pub const HAKI_MIN_RYUO_RELEASE: f32 = 0.2;

/// Armament Haki progression tiers (`com.devilfruits.core.haki.HakiTier`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HakiTier {
    Novice,
    Adept,
    Expert,
    Master,
}

impl HakiTier {
    /// Exact tier bands: Novice 1..=3, Adept 4..=6, Expert 7..=9, Master 10.
    pub const fn for_level(level: u32) -> Self {
        match level {
            0..=3 => Self::Novice,
            4..=6 => Self::Adept,
            7..=9 => Self::Expert,
            _ => Self::Master,
        }
    }

    pub const fn unlocks_ryuo(self) -> bool {
        matches!(self, Self::Expert | Self::Master)
    }

    /// Allowed coatable part bitmask per tier (`HakiParts.allowedForTier`):
    /// Novice = `259` (fists+weapon), Adept = `319` (fists+arms+legs+weapon), Expert/Master = `511`.
    pub const fn allowed_parts(self) -> u16 {
        match self {
            Self::Novice => 259,
            Self::Adept => 319,
            Self::Expert | Self::Master => 511,
        }
    }

    /// Default coating mask when entering this tier (`HakiParts.defaultForTier`):
    /// Novice = `259`, Adept = `271`, Expert/Master = `511`.
    pub const fn default_parts(self) -> u16 {
        match self {
            Self::Novice => 259,
            Self::Adept => 271,
            Self::Expert | Self::Master => 511,
        }
    }

    /// Spirit drain multiplier per tier (`HakiConfig`): Novice `1.6`, Adept `1.2`, Expert `0.9`, Master `0.6`.
    pub const fn drain_multiplier(self, cfg: &HakiConfig) -> f32 {
        match self {
            Self::Novice => cfg.drain_mult_novice,
            Self::Adept => cfg.drain_mult_adept,
            Self::Expert => cfg.drain_mult_expert,
            Self::Master => cfg.drain_mult_master,
        }
    }
}

/// The 9 coatable Armament Haki slots (`com.devilfruits.core.haki.HakiParts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HakiPart {
    RightFist = 0,
    LeftFist = 1,
    RightArm = 2,
    LeftArm = 3,
    RightLeg = 4,
    LeftLeg = 5,
    Torso = 6,
    Head = 7,
    Weapon = 8,
}

impl HakiPart {
    pub const ALL: [Self; 9] = [
        Self::RightFist,
        Self::LeftFist,
        Self::RightArm,
        Self::LeftArm,
        Self::RightLeg,
        Self::LeftLeg,
        Self::Torso,
        Self::Head,
        Self::Weapon,
    ];

    pub const fn bit(self) -> u16 {
        1u16 << (self as u16)
    }

    /// Exact body coverage weight (`HakiParts.WEIGHTS`):
    /// `[0.05, 0.05, 0.1, 0.1, 0.12, 0.12, 0.32, 0.14, 0.0]`.
    pub const fn weight(self) -> f32 {
        match self {
            Self::RightFist => 0.05,
            Self::LeftFist => 0.05,
            Self::RightArm => 0.10,
            Self::LeftArm => 0.10,
            Self::RightLeg => 0.12,
            Self::LeftLeg => 0.12,
            Self::Torso => 0.32,
            Self::Head => 0.14,
            Self::Weapon => 0.0,
        }
    }
}

/// Bitmask helpers matching `com.devilfruits.core.haki.HakiParts`.
pub struct HakiParts;

impl HakiParts {
    pub const RIGHT_FIST: u16 = 1;
    pub const LEFT_FIST: u16 = 2;
    pub const RIGHT_ARM: u16 = 4;
    pub const LEFT_ARM: u16 = 8;
    pub const RIGHT_LEG: u16 = 16;
    pub const LEFT_LEG: u16 = 32;
    pub const TORSO: u16 = 64;
    pub const HEAD: u16 = 128;
    pub const WEAPON: u16 = 256;
    pub const BODY: u16 = 255;
    pub const ALL: u16 = 511;

    /// Exact `HakiParts.normalize(mask)`:
    /// coating `RIGHT_ARM` automatically coats `RIGHT_FIST`; coating `LEFT_ARM` coats `LEFT_FIST`; masked to `& 511`.
    pub const fn normalize(mut mask: u16) -> u16 {
        if (mask & Self::RIGHT_ARM) != 0 {
            mask |= Self::RIGHT_FIST;
        }
        if (mask & Self::LEFT_ARM) != 0 {
            mask |= Self::LEFT_FIST;
        }
        mask & Self::ALL
    }

    /// Total body coverage fraction in `0.0..=1.0` (sum of active body part weights).
    pub fn coverage(mask: u16) -> f32 {
        let m = Self::normalize(mask);
        let mut sum = 0.0f32;
        for part in HakiPart::ALL {
            if (m & part.bit()) != 0 {
                sum += part.weight();
            }
        }
        sum.clamp(0.0, 1.0)
    }

    /// Exact Spirit drain share (`HakiManager`): `max(0.2, coverage(mask) + (weapon ? 0.05 : 0.0))`.
    pub fn drain_share(mask: u16) -> f32 {
        let m = Self::normalize(mask);
        let weapon_add = if (m & Self::WEAPON) != 0 { 0.05 } else { 0.0 };
        (Self::coverage(m) + weapon_add).max(0.2)
    }

    /// True if any offensive part (fist or weapon) is coated.
    pub const fn has_offense(mask: u16) -> bool {
        (mask & (Self::RIGHT_FIST | Self::LEFT_FIST | Self::WEAPON)) != 0
    }
}

/// Complete Armament Haki balance configuration (`com.devilfruits.core.haki.HakiConfig`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakiConfig {
    pub enabled: bool,
    pub xp_base: f32,
    pub xp_exponent: f32,
    pub xp_per_health: f32,
    pub xp_per_fist_hit: f32,
    pub xp_coated_mult: f32,
    pub xp_logia_victim_mult: f32,
    pub xp_haki_victim_mult: f32,
    pub xp_boss_mult: f32,
    pub min_victim_health: f32,
    pub same_type_window_seconds: i32,
    pub same_type_falloff: f32,
    pub daily_soft_cap: f32,
    pub over_cap_multiplier: f32,

    pub spirit_base: f32,
    pub spirit_per_level: f32,
    pub drain_per_second: f32,
    pub activation_cost: f32,
    pub regen_per_second: f32,
    pub regen_delay_ticks: i32,
    pub drain_mult_novice: f32,
    pub drain_mult_adept: f32,
    pub drain_mult_expert: f32,
    pub drain_mult_master: f32,

    pub attack_per_level: f32,
    pub defense_per_level: f32,
    pub max_defense_reduction: f32,
    pub logia_base_fraction: f32,
    pub logia_per_level: f32,
    pub logia_element_reduction: f32,
    pub ability_multiplier: f32,
    pub clash_bonus: f32,
    pub clash_penalty: f32,
    pub clash_jitter: f32,
    pub ryuo_charge_ticks: i32,
    pub ryuo_cost: f32,
    pub ryuo_multiplier: f32,
    pub ryuo_internal_fraction: f32,
    pub ryuo_internal_master_fraction: f32,
}

impl Default for HakiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            xp_base: 120.0,
            xp_exponent: 1.5,
            xp_per_health: 1.0,
            xp_per_fist_hit: 0.5,
            xp_coated_mult: 2.0,
            xp_logia_victim_mult: 3.0,
            xp_haki_victim_mult: 3.0,
            xp_boss_mult: 5.0,
            min_victim_health: 6.0,
            same_type_window_seconds: 300,
            same_type_falloff: 0.85,
            daily_soft_cap: 2500.0,
            over_cap_multiplier: 0.25,

            spirit_base: 60.0,
            spirit_per_level: 12.0,
            drain_per_second: 6.0,
            activation_cost: 8.0,
            regen_per_second: 4.0,
            regen_delay_ticks: 20,
            drain_mult_novice: 1.6,
            drain_mult_adept: 1.2,
            drain_mult_expert: 0.9,
            drain_mult_master: 0.6,

            attack_per_level: 0.06,
            defense_per_level: 0.05,
            max_defense_reduction: 0.8,
            logia_base_fraction: 0.2,
            logia_per_level: 0.2,
            logia_element_reduction: 0.15,
            ability_multiplier: 1.25,
            clash_bonus: 0.25,
            clash_penalty: 0.5,
            clash_jitter: 1.0,
            ryuo_charge_ticks: 30,
            ryuo_cost: 20.0,
            ryuo_multiplier: 1.8,
            ryuo_internal_fraction: 0.35,
            ryuo_internal_master_fraction: 0.6,
        }
    }
}

/// Persistent per-player Armament Haki data (`com.devilfruits.core.haki.PlayerHakiData`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerHakiData {
    pub unlocked: bool,
    pub level: u32,
    pub xp: f32,
    pub spirit: f32,
    pub active: bool,
    pub selected_parts: u16,
    pub last_spend_tick: i64,
    pub ryuo_charging_since: Option<i64>,
    pub ryuo_armed_fraction: f32,
    pub ryuo_armed_until: i64,
    /// Training anti-grind trackers: `(last_window_day, daily_xp_earned)`.
    pub training_day: i64,
    pub daily_xp: f32,
    /// Recent kill timestamps by mob type ID for same-type falloff (`300s` = `6000` ticks).
    pub recent_kills_by_type: BTreeMap<String, Vec<i64>>,
}

impl Default for PlayerHakiData {
    fn default() -> Self {
        Self {
            unlocked: false,
            level: 0,
            xp: 0.0,
            spirit: 0.0,
            active: false,
            selected_parts: HakiTier::Novice.default_parts(),
            last_spend_tick: 0,
            ryuo_charging_since: None,
            ryuo_armed_fraction: 0.0,
            ryuo_armed_until: 0,
            training_day: 0,
            daily_xp: 0.0,
            recent_kills_by_type: BTreeMap::new(),
        }
    }
}

impl PlayerHakiData {
    pub fn tier(&self) -> HakiTier {
        HakiTier::for_level(self.level.max(1))
    }

    /// Exact max Spirit pool: `60 + 12 * level`.
    pub fn max_spirit(&self, cfg: &HakiConfig) -> f32 {
        if !self.unlocked || self.level == 0 {
            return 0.0;
        }
        cfg.spirit_base + cfg.spirit_per_level * (self.level as f32)
    }

    /// Exact XP needed to advance from `level` to `level + 1`: `xp_base * level^xp_exponent`.
    pub fn xp_to_next(level: u32, cfg: &HakiConfig) -> f32 {
        if level == 0 || level >= HAKI_MAX_LEVEL {
            return 0.0;
        }
        cfg.xp_base * (level as f32).powf(cfg.xp_exponent)
    }

    /// Active coated part mask (0 if inactive), clamped to tier allowed parts and normalized.
    pub fn effective_parts(&self) -> u16 {
        if !self.unlocked || !self.active {
            return 0;
        }
        let allowed = self.tier().allowed_parts();
        HakiParts::normalize(self.selected_parts & allowed)
    }

    /// Logia bypass reach percentage (`0.0..=1.0`): `min(1.0, 0.2 + 0.2 * level)` (full 100% at level 4!).
    pub fn logia_reach(&self, cfg: &HakiConfig) -> f32 {
        if !self.unlocked || self.level == 0 {
            return 0.0;
        }
        (cfg.logia_base_fraction + cfg.logia_per_level * (self.level as f32)).min(1.0)
    }

    pub fn can_use_ryuo(&self) -> bool {
        self.unlocked && self.tier().unlocks_ryuo()
    }

    /// Unlocks Armament Haki at level 1 and fills Spirit.
    pub fn unlock(&mut self, cfg: &HakiConfig) {
        if !self.unlocked {
            self.unlocked = true;
            self.level = 1;
            self.xp = 0.0;
            self.selected_parts = HakiTier::Novice.default_parts();
            self.spirit = self.max_spirit(cfg);
        }
    }

    /// Sets explicit Haki level (`0` locks Haki; `1..=10` unlocks and updates allowed parts).
    pub fn set_level(&mut self, level: u32, cfg: &HakiConfig) {
        if level == 0 {
            self.unlocked = false;
            self.level = 0;
            self.xp = 0.0;
            self.spirit = 0.0;
            self.active = false;
            return;
        }
        self.unlocked = true;
        self.level = level.min(HAKI_MAX_LEVEL);
        self.xp = 0.0;
        let allowed = self.tier().allowed_parts();
        if (self.selected_parts & allowed) == 0 {
            self.selected_parts = self.tier().default_parts();
        } else {
            self.selected_parts = HakiParts::normalize(self.selected_parts & allowed);
        }
        self.spirit = self.max_spirit(cfg);
    }

    /// Grants Haki XP and returns the number of levels gained.
    pub fn add_xp(&mut self, amount: f32, cfg: &HakiConfig) -> u32 {
        if !self.unlocked || amount <= 0.0 || self.level >= HAKI_MAX_LEVEL {
            return 0;
        }
        self.xp += amount;
        let mut gained = 0u32;
        while self.level < HAKI_MAX_LEVEL {
            let need = Self::xp_to_next(self.level, cfg);
            if self.xp + 1e-4 < need {
                break;
            }
            self.xp = (self.xp - need).max(0.0);
            self.level += 1;
            gained += 1;
        }
        if self.level >= HAKI_MAX_LEVEL {
            self.level = HAKI_MAX_LEVEL;
            self.xp = 0.0;
        }
        gained
    }
}

/// Core Haki lifecycle manager (`com.devilfruits.core.haki.HakiManager`).
pub struct HakiManager;

impl HakiManager {
    /// Toggles coating on/off (checks `activation_cost` when activating).
    pub fn toggle_coating(
        data: &mut PlayerHakiData,
        now: i64,
        creative: bool,
        cfg: &HakiConfig,
    ) -> bool {
        if !cfg.enabled || !data.unlocked {
            data.active = false;
            return false;
        }
        if data.active {
            data.active = false;
            data.ryuo_charging_since = None;
            return false;
        }
        if !creative && data.spirit + 1e-4 < cfg.activation_cost {
            return false;
        }
        if !creative {
            data.spirit = (data.spirit - cfg.activation_cost).max(0.0);
            data.last_spend_tick = now;
        }
        let allowed = data.tier().allowed_parts();
        if (data.selected_parts & allowed) == 0 {
            data.selected_parts = data.tier().default_parts();
        }
        data.active = true;
        true
    }

    /// Updates the player's selected coatable parts mask, gated by current `HakiTier`.
    pub fn set_parts(data: &mut PlayerHakiData, requested_mask: u16) -> u16 {
        let allowed = data.tier().allowed_parts();
        let next = HakiParts::normalize(requested_mask & allowed);
        data.selected_parts = next;
        next
    }

    /// Starts charging Ryuo (requires Expert/Master tier, level >= 7).
    pub fn start_ryuo(data: &mut PlayerHakiData, now: i64) -> bool {
        if !data.can_use_ryuo() || !data.active {
            return false;
        }
        data.ryuo_charging_since = Some(now);
        true
    }

    /// Releases Ryuo charge; returns armed fraction `Some(f)` if `>= 0.2` and Spirit paid.
    pub fn release_ryuo(
        data: &mut PlayerHakiData,
        now: i64,
        creative: bool,
        cfg: &HakiConfig,
    ) -> Option<f32> {
        let start = data.ryuo_charging_since.take()?;
        let elapsed = (now - start).max(0) as f32;
        let frac = (elapsed / cfg.ryuo_charge_ticks.max(1) as f32).clamp(0.0, 1.0);
        if frac < HAKI_MIN_RYUO_RELEASE {
            return None;
        }
        let cost = cfg.ryuo_cost * frac;
        if !creative {
            if data.spirit + 1e-4 < cost {
                return None;
            }
            data.spirit = (data.spirit - cost).max(0.0);
            data.last_spend_tick = now;
        }
        data.ryuo_armed_fraction = frac;
        data.ryuo_armed_until = now + 40;
        Some(frac)
    }

    /// Ticks Spirit drain while active and Spirit regen after `regen_delay_ticks` (20t).
    pub fn tick(data: &mut PlayerHakiData, now: i64, creative: bool, cfg: &HakiConfig) {
        if !cfg.enabled || !data.unlocked {
            data.active = false;
            return;
        }
        if now >= data.ryuo_armed_until {
            data.ryuo_armed_fraction = 0.0;
        }

        let max_sp = data.max_spirit(cfg);
        if data.active {
            if !creative {
                let mask = data.effective_parts();
                let share = HakiParts::drain_share(mask);
                let tier_mult = data.tier().drain_multiplier(cfg);
                let drain_per_tick = (cfg.drain_per_second * share * tier_mult) / 20.0;
                data.spirit = (data.spirit - drain_per_tick).max(0.0);
                data.last_spend_tick = now;
                if data.spirit <= 0.0 {
                    data.active = false;
                    data.ryuo_charging_since = None;
                }
            }
        } else if now.saturating_sub(data.last_spend_tick) >= cfg.regen_delay_ticks as i64 {
            data.spirit = (data.spirit + cfg.regen_per_second / 20.0).min(max_sp);
        }
    }
}

/// Outcome of an Armament Haki clash (`com.devilfruits.core.haki.HakiClash`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakiClashOutcome {
    pub attacker_score: f32,
    pub defender_score: f32,
    pub attacker_won: bool,
    pub strength: f32,
    pub damage_multiplier: f32,
}

/// Armament vs Armament clash resolver (`HakiClash.java`).
pub struct HakiClash;

impl HakiClash {
    /// Exact clash score formula: `level + coverage * 3.0 + (ryuo ? 2.0 : 0.0) + rand * jitter`.
    pub fn score(
        data: &PlayerHakiData,
        ryuo_active: bool,
        rand_unit: f32,
        cfg: &HakiConfig,
    ) -> f32 {
        let cov = HakiParts::coverage(data.effective_parts());
        (data.level as f32)
            + cov * 3.0
            + if ryuo_active { 2.0 } else { 0.0 }
            + rand_unit * cfg.clash_jitter
    }

    /// Exact shockwave VFX strength: `min(1.0, 0.3 + (a_lv + d_lv) / 20.0 * 0.7)`.
    pub fn strength(attacker_level: u32, defender_level: u32) -> f32 {
        (0.3 + ((attacker_level + defender_level) as f32 / 20.0) * 0.7).min(1.0)
    }

    pub fn resolve(
        attacker: &PlayerHakiData,
        defender: &PlayerHakiData,
        attacker_ryuo: bool,
        defender_ryuo: bool,
        atk_rand: f32,
        def_rand: f32,
        cfg: &HakiConfig,
    ) -> HakiClashOutcome {
        let a = Self::score(attacker, attacker_ryuo, atk_rand, cfg);
        let d = Self::score(defender, defender_ryuo, def_rand, cfg);
        let attacker_won = a >= d;
        let damage_multiplier = if attacker_won {
            1.0 + cfg.clash_bonus
        } else {
            1.0 - cfg.clash_penalty
        };
        HakiClashOutcome {
            attacker_score: a,
            defender_score: d,
            attacker_won,
            strength: Self::strength(attacker.level, defender.level),
            damage_multiplier,
        }
    }
}

/// Combat calculation output (`com.devilfruits.core.haki.HakiCombat`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakiHitResult {
    pub scaled_damage: f32,
    pub armor_bypass_damage: f32,
    pub logia_bypass: Option<LogiaBypassResult>,
    pub used_ryuo: bool,
    pub clash: Option<HakiClashOutcome>,
}

/// Offensive and defensive Armament Haki modifiers (`HakiCombat.java`).
pub struct HakiCombat;

impl HakiCombat {
    /// Offensive damage multiplier: `1 + 0.06 * level` (plus Ryuo multiplier if armed).
    pub fn attack_multiplier(
        attacker: &PlayerHakiData,
        using_weapon: bool,
        is_ability: bool,
        ryuo_fraction: f32,
        cfg: &HakiConfig,
    ) -> f32 {
        let mask = attacker.effective_parts();
        if mask == 0 {
            return 1.0;
        }
        let coated_strike = if using_weapon {
            (mask & HakiParts::WEAPON) != 0
        } else {
            (mask & (HakiParts::RIGHT_FIST | HakiParts::LEFT_FIST)) != 0
        };
        if !coated_strike && !is_ability {
            return 1.0;
        }
        let mut mult = 1.0 + cfg.attack_per_level * (attacker.level as f32);
        if is_ability {
            mult *= cfg.ability_multiplier;
        }
        if ryuo_fraction >= HAKI_MIN_RYUO_RELEASE && attacker.can_use_ryuo() {
            mult *= 1.0 + (cfg.ryuo_multiplier - 1.0) * ryuo_fraction.clamp(0.0, 1.0);
        }
        mult
    }

    /// Defensive incoming damage multiplier (`1.0 - min(0.8, 0.05 * level * coverage)`).
    pub fn defense_multiplier(
        defender: &PlayerHakiData,
        is_logia_element_damage: bool,
        cfg: &HakiConfig,
    ) -> f32 {
        let mask = defender.effective_parts();
        if mask == 0 {
            return 1.0;
        }
        let cov = HakiParts::coverage(mask);
        let mut reduction = (cfg.defense_per_level * (defender.level as f32) * cov)
            .min(cfg.max_defense_reduction);
        if is_logia_element_damage {
            reduction = (reduction + cfg.logia_element_reduction).min(cfg.max_defense_reduction);
        }
        (1.0 - reduction).clamp(0.0, 1.0)
    }

    /// Evaluates a full attacker-vs-defender hit with optional Ryuo, Logia bypass, and Haki Clash.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_hit(
        attacker: &mut PlayerHakiData,
        defender: Option<&PlayerHakiData>,
        base_damage: f32,
        using_weapon: bool,
        is_ability: bool,
        is_logia_element_damage: bool,
        clash_rand: (f32, f32),
        cfg: &HakiConfig,
    ) -> HakiHitResult {
        let ryuo_frac = attacker.ryuo_armed_fraction;
        let used_ryuo = ryuo_frac >= HAKI_MIN_RYUO_RELEASE && attacker.can_use_ryuo();
        if used_ryuo {
            attacker.ryuo_armed_fraction = 0.0;
        }

        let atk_mult = Self::attack_multiplier(attacker, using_weapon, is_ability, ryuo_frac, cfg);
        let def_mult = defender
            .map(|d| Self::defense_multiplier(d, is_logia_element_damage, cfg))
            .unwrap_or(1.0);

        let atk_active = HakiParts::has_offense(attacker.effective_parts());
        let def_active = defender.is_some_and(|d| d.effective_parts() != 0);

        let clash = if atk_active && def_active {
            defender.map(|d| {
                HakiClash::resolve(
                    attacker,
                    d,
                    used_ryuo,
                    d.ryuo_armed_fraction >= HAKI_MIN_RYUO_RELEASE,
                    clash_rand.0,
                    clash_rand.1,
                    cfg,
                )
            })
        } else {
            None
        };

        let clash_mult = clash.map(|c| c.damage_multiplier).unwrap_or(1.0);
        let total = base_damage * atk_mult * def_mult * clash_mult;

        let internal_frac = if used_ryuo {
            if attacker.tier() == HakiTier::Master {
                cfg.ryuo_internal_master_fraction
            } else {
                cfg.ryuo_internal_fraction
            }
        } else {
            0.0
        };
        let armor_bypass_damage = total * internal_frac;
        let scaled_damage = total - armor_bypass_damage;

        let logia_bypass = if atk_active {
            let reach = attacker.logia_reach(cfg);
            (reach > 0.0).then_some(LogiaBypassResult {
                source: LogiaBypassSource::ArmamentHaki,
                fraction: reach,
            })
        } else {
            None
        };

        HakiHitResult {
            scaled_damage,
            armor_bypass_damage,
            logia_bypass,
            used_ryuo,
            clash,
        }
    }
}

/// Bare-fist & coated combat training engine (`com.devilfruits.core.haki.HakiTraining`).
pub struct HakiTraining;

impl HakiTraining {
    fn reset_day_if_needed(data: &mut PlayerHakiData, now: i64) {
        let day = now / 24_000;
        if day != data.training_day {
            data.training_day = day;
            data.daily_xp = 0.0;
        }
    }

    fn apply_daily_soft_cap(data: &mut PlayerHakiData, raw: f32, now: i64, cfg: &HakiConfig) -> f32 {
        Self::reset_day_if_needed(data, now);
        if raw <= 0.0 {
            return 0.0;
        }
        let room = (cfg.daily_soft_cap - data.daily_xp).max(0.0);
        let granted = if raw <= room {
            raw
        } else {
            room + (raw - room) * cfg.over_cap_multiplier
        };
        data.daily_xp += granted;
        granted
    }

    /// Awards XP for landing a bare-fist punch (`xp_per_fist_hit = 0.5`, doubled if coated).
    pub fn on_fist_hit(
        data: &mut PlayerHakiData,
        victim_max_health: f32,
        now: i64,
        cfg: &HakiConfig,
    ) -> (f32, u32) {
        if !cfg.enabled || victim_max_health < cfg.min_victim_health {
            return (0.0, 0);
        }
        if !data.unlocked {
            data.unlock(cfg);
        }
        let mut raw = cfg.xp_per_fist_hit;
        if data.active {
            raw *= cfg.xp_coated_mult;
        }
        let xp = Self::apply_daily_soft_cap(data, raw, now, cfg);
        let gained = data.add_xp(xp, cfg);
        (xp, gained)
    }

    /// Awards XP for defeating an enemy, applying coated/Logia/Haki/boss multipliers,
    /// same-mob-type falloff (`0.85^n` within `300s`), and daily soft cap (`2500`).
    #[allow(clippy::too_many_arguments)]
    pub fn on_kill(
        data: &mut PlayerHakiData,
        victim_type: &str,
        victim_max_health: f32,
        victim_is_logia: bool,
        victim_has_haki: bool,
        victim_is_boss: bool,
        now: i64,
        cfg: &HakiConfig,
    ) -> (f32, u32) {
        if !cfg.enabled || victim_max_health < cfg.min_victim_health {
            return (0.0, 0);
        }
        if !data.unlocked {
            data.unlock(cfg);
        }

        let window = (cfg.same_type_window_seconds.max(1) as i64) * 20;
        let history = data
            .recent_kills_by_type
            .entry(victim_type.to_string())
            .or_default();
        history.retain(|&ts| now.saturating_sub(ts) <= window);
        let prior_count = history.len() as i32;
        history.push(now);

        let mut raw = victim_max_health * cfg.xp_per_health;
        if data.active {
            raw *= cfg.xp_coated_mult;
        }
        if victim_is_logia {
            raw *= cfg.xp_logia_victim_mult;
        }
        if victim_has_haki {
            raw *= cfg.xp_haki_victim_mult;
        }
        if victim_is_boss {
            raw *= cfg.xp_boss_mult;
        }
        raw *= cfg.same_type_falloff.powi(prior_count);

        let xp = Self::apply_daily_soft_cap(data, raw, now, cfg);
        let gained = data.add_xp(xp, cfg);
        (xp, gained)
    }
}
