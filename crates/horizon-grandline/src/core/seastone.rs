//! Seastone (Kairoseki) gear, handcuffs, Jitte techniques, armor damage
//! reduction, and fruit suppression combat rules.
//!
//! Mirrors `core/seastone/{SeastoneGear,SeastoneCombat,SeastoneCuffsItem,SeastoneArmorItem,JitteTechniques}.java`
//! and `core/item/SeastoneItem.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::{
    SEASTONE_CUFFS_TICKS, SEASTONE_HIT_SUPPRESS_TICKS, SEASTONE_JITTE_STUN_TICKS,
    SeastoneConfig as WorldSeastoneConfig, SeastoneTouchState,
};
use serde::{Deserialize, Serialize};

use crate::core::logia::{LogiaBypassResult, LogiaBypassSource};
use crate::core::rules::{PlayerFruitData, PlayerUuid};

/// Canonical Seastone item/block IDs (`SeastoneGear.java` & `DFItems.java`).
pub const SEASTONE_ORE_ID: &str = "devilfruits:seastone_ore";
pub const DEEPSLATE_SEASTONE_ORE_ID: &str = "devilfruits:deepslate_seastone_ore";
pub const SEASTONE_BLOCK_ID: &str = "devilfruits:seastone_block";
pub const RAW_SEASTONE_ID: &str = "devilfruits:raw_seastone";
pub const SEASTONE_NUGGET_ID: &str = "devilfruits:seastone_nugget";
pub const SEASTONE_INGOT_ID: &str = "devilfruits:seastone_ingot";
pub const SEASTONE_PICKAXE_ID: &str = "devilfruits:seastone_pickaxe";
pub const SEASTONE_AXE_ID: &str = "devilfruits:seastone_axe";
pub const SEASTONE_SHOVEL_ID: &str = "devilfruits:seastone_shovel";
pub const SEASTONE_HOE_ID: &str = "devilfruits:seastone_hoe";
pub const SEASTONE_SWORD_ID: &str = "devilfruits:seastone_sword";
pub const SEASTONE_JITTE_ID: &str = "devilfruits:seastone_jitte";
pub const SEASTONE_ARROW_ID: &str = "devilfruits:seastone_arrow";
pub const SEASTONE_CUFFS_ID: &str = "devilfruits:seastone_cuffs";
pub const SEASTONE_HELMET_ID: &str = "devilfruits:seastone_helmet";
pub const SEASTONE_CHESTPLATE_ID: &str = "devilfruits:seastone_chestplate";
pub const SEASTONE_LEGGINGS_ID: &str = "devilfruits:seastone_leggings";
pub const SEASTONE_BOOTS_ID: &str = "devilfruits:seastone_boots";

/// Seastone tool tier parameters (`SeastoneGear.TIER`: durability 2031, speed 9.0, bonus 4.0, enchantability 15).
pub const SEASTONE_TIER_DURABILITY: u32 = 2031;
pub const SEASTONE_TIER_MINING_SPEED: f32 = 9.0;
pub const SEASTONE_TIER_ATTACK_BONUS: f32 = 4.0;
pub const SEASTONE_TIER_ENCHANTABILITY: u32 = 15;

/// Seastone armor material parameters (`SeastoneGear.ARMOR`: defense [3,6,8,3], toughness 3.0, kb resist 0.1).
pub const SEASTONE_ARMOR_TOUGHNESS: f32 = 3.0;
pub const SEASTONE_ARMOR_KNOCKBACK_RESISTANCE: f32 = 0.1;

/// Default per-piece Devil Fruit damage reduction for non-fruit wearers (`PoneglyphConfig.ARMOR_FRUIT_DAMAGE_REDUCTION`).
pub const SEASTONE_ARMOR_DEFAULT_FRUIT_DAMAGE_REDUCTION: f32 = 0.10;

/// Seastone handcuffs durability (`SeastoneGear.CUFFS`: 24 uses).
pub const SEASTONE_CUFFS_MAX_DURABILITY: u32 = 24;

/// `seastone_suppressed` effect color (`3968147` = `0x3C_8C_93`).
pub const SEASTONE_SUPPRESSED_COLOR: u32 = 3_968_147;

/// All Seastone item identifiers belonging to `#devilfruits:seastone`.
pub const ALL_SEASTONE_ITEMS: [&str; 17] = [
    RAW_SEASTONE_ID,
    SEASTONE_NUGGET_ID,
    SEASTONE_INGOT_ID,
    SEASTONE_BLOCK_ID,
    SEASTONE_ORE_ID,
    DEEPSLATE_SEASTONE_ORE_ID,
    SEASTONE_PICKAXE_ID,
    SEASTONE_AXE_ID,
    SEASTONE_SHOVEL_ID,
    SEASTONE_HOE_ID,
    SEASTONE_SWORD_ID,
    SEASTONE_JITTE_ID,
    SEASTONE_ARROW_ID,
    SEASTONE_CUFFS_ID,
    SEASTONE_HELMET_ID,
    SEASTONE_CHESTPLATE_ID,
    SEASTONE_LEGGINGS_ID,
];

/// Returns true if `item_id` is any Seastone item or block (`#devilfruits:seastone`).
pub fn is_seastone_item(item_id: &str) -> bool {
    let id = crate::core::fruit::df_id(item_id);
    ALL_SEASTONE_ITEMS.contains(&id.as_str()) || id == SEASTONE_BOOTS_ID
}

/// Returns true if `item_id` is in `#devilfruits:seastone_gear` (tools, weapons, armor, cuffs).
pub fn is_seastone_gear(item_id: &str) -> bool {
    let id = crate::core::fruit::df_id(item_id);
    matches!(
        id.as_str(),
        SEASTONE_PICKAXE_ID
            | SEASTONE_AXE_ID
            | SEASTONE_SHOVEL_ID
            | SEASTONE_HOE_ID
            | SEASTONE_SWORD_ID
            | SEASTONE_JITTE_ID
            | SEASTONE_CUFFS_ID
            | SEASTONE_HELMET_ID
            | SEASTONE_CHESTPLATE_ID
            | SEASTONE_LEGGINGS_ID
            | SEASTONE_BOOTS_ID
    )
}

/// Returns true if `block_id` is in `#devilfruits:seastone` (`seastone_block`, `seastone_ore`, `deepslate_seastone_ore`).
pub fn is_seastone_block(block_id: &str) -> bool {
    let id = crate::core::fruit::df_id(block_id);
    matches!(
        id.as_str(),
        SEASTONE_BLOCK_ID | SEASTONE_ORE_ID | DEEPSLATE_SEASTONE_ORE_ID
    )
}

/// Returns true if `item_id` is a Seastone armor piece.
pub fn is_seastone_armor(item_id: &str) -> bool {
    let id = crate::core::fruit::df_id(item_id);
    matches!(
        id.as_str(),
        SEASTONE_HELMET_ID | SEASTONE_CHESTPLATE_ID | SEASTONE_LEGGINGS_ID | SEASTONE_BOOTS_ID
    )
}

/// Complete Seastone combat and suppression configuration (`PoneglyphConfig` seastone section).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeastoneCombatConfig {
    pub seastone_weakness: bool,
    pub gear_suppresses_holder: bool,
    pub armor_suppresses_fruit_user: bool,
    pub armor_fruit_damage_reduction: f32,
    pub hit_suppress_ticks: i32,
    pub jitte_stun_ticks: i32,
    pub jitte_safe_handle: bool,
    pub cuffs_ticks: i32,
}

impl Default for SeastoneCombatConfig {
    fn default() -> Self {
        Self {
            seastone_weakness: true,
            gear_suppresses_holder: true,
            armor_suppresses_fruit_user: true,
            armor_fruit_damage_reduction: SEASTONE_ARMOR_DEFAULT_FRUIT_DAMAGE_REDUCTION,
            hit_suppress_ticks: SEASTONE_HIT_SUPPRESS_TICKS,
            jitte_stun_ticks: SEASTONE_JITTE_STUN_TICKS,
            jitte_safe_handle: true,
            cuffs_ticks: SEASTONE_CUFFS_TICKS,
        }
    }
}

impl SeastoneCombatConfig {
    pub const fn to_world_config(self) -> WorldSeastoneConfig {
        WorldSeastoneConfig {
            seastone_weakness: self.seastone_weakness,
            armor_suppresses_fruit_user: self.armor_suppresses_fruit_user,
            jitte_safe_handle: self.jitte_safe_handle,
            gear_suppresses_holder: self.gear_suppresses_holder,
        }
    }
}

/// Active Seastone handcuffs / suppression status on a player (`SeastoneCuffsItem.java` & `SeastoneCombat.java`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeastoneHandcuffs {
    pub suppressed_until_tick: i64,
    pub cuffed_until_tick: i64,
    pub cuffed_by: Option<PlayerUuid>,
}

impl SeastoneHandcuffs {
    pub fn is_suppressed(&self, now: i64) -> bool {
        now < self.suppressed_until_tick || now < self.cuffed_until_tick
    }

    pub fn is_cuffed(&self, now: i64) -> bool {
        now < self.cuffed_until_tick
    }

    pub fn remaining_suppress_ticks(&self, now: i64) -> i32 {
        let max_until = self.suppressed_until_tick.max(self.cuffed_until_tick);
        (max_until - now).max(0) as i32
    }

    /// Applies `seastone_suppressed` for `ticks` (refreshing if longer than current remaining duration).
    pub fn apply_suppression(&mut self, now: i64, ticks: i32) {
        if ticks > 0 {
            let until = now + ticks as i64;
            if until > self.suppressed_until_tick {
                self.suppressed_until_tick = until;
            }
        }
    }

    /// Applies Seastone Cuffs (`SeastoneCuffsItem.interactLivingEntity`):
    /// - Sets `seastone_suppressed` + Slowness III (`amplifier = 2`) + Weakness II (`1`) + Mining Fatigue II (`1`) for `cuffs_ticks` (`600` default).
    pub fn apply_cuffs(&mut self, now: i64, cuffs_ticks: i32, by: Option<PlayerUuid>) -> i32 {
        let dur = cuffs_ticks.max(20);
        let until = now + dur as i64;
        self.cuffed_until_tick = self.cuffed_until_tick.max(until);
        self.suppressed_until_tick = self.suppressed_until_tick.max(until);
        self.cuffed_by = by;
        dur
    }

    pub fn clear(&mut self) {
        self.suppressed_until_tick = 0;
        self.cuffed_until_tick = 0;
        self.cuffed_by = None;
    }
}

/// Outcome of a Seastone weapon or arrow striking a target (`SeastoneCombat.hit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SeastoneStrikeOutcome {
    pub suppress_ticks_applied: i32,
    pub stun_ticks_applied: i32,
    pub logia_bypass: Option<LogiaBypassResult>,
}

/// Seastone combat and damage reduction evaluator (`com.devilfruits.core.seastone.SeastoneCombat`).
pub struct SeastoneCombat;

impl SeastoneCombat {
    /// Builds a [`SeastoneTouchState`] from held item IDs, armor count, block contact, and active suppression.
    pub fn build_touch_state(
        main_hand_item: Option<&str>,
        off_hand_item: Option<&str>,
        seastone_armor_pieces: u32,
        standing_on_seastone_block: bool,
        inside_seastone_block: bool,
        has_suppressed_effect: bool,
    ) -> SeastoneTouchState {
        let classify = |item: Option<&str>| -> (bool, bool, bool) {
            let Some(raw) = item else {
                return (false, false, false);
            };
            let id = crate::core::fruit::df_id(raw);
            let is_ss = is_seastone_item(&id);
            let is_jitte = id == SEASTONE_JITTE_ID;
            let is_gear = is_seastone_gear(&id);
            (is_ss, is_jitte, is_gear)
        };
        let (main_ss, main_jitte, main_gear) = classify(main_hand_item);
        let (off_ss, off_jitte, off_gear) = classify(off_hand_item);
        SeastoneTouchState {
            main_hand_seastone: main_ss,
            main_hand_is_jitte: main_jitte,
            main_hand_is_gear: main_gear,
            off_hand_seastone: off_ss,
            off_hand_is_jitte: off_jitte,
            off_hand_is_gear: off_gear,
            has_suppressed_effect,
            seastone_armor_pieces,
            standing_on_seastone_block,
            inside_seastone_block,
        }
    }

    /// Evaluates a direct melee blow or Seastone arrow hit on `victim` (`SeastoneCombat.hit`).
    pub fn on_hit(
        victim: &PlayerFruitData,
        victim_cuffs: &mut SeastoneHandcuffs,
        weapon_item: Option<&str>,
        is_seastone_arrow: bool,
        rule_override: Option<(i32, i32)>,
        now: i64,
        cfg: &SeastoneCombatConfig,
    ) -> SeastoneStrikeOutcome {
        let weapon_id = weapon_item.map(crate::core::fruit::df_id);
        let is_gear_blow = weapon_id.as_deref().is_some_and(is_seastone_gear);
        let is_jitte = weapon_id.as_deref() == Some(SEASTONE_JITTE_ID);

        let logia_bypass = if is_gear_blow || is_seastone_arrow {
            Some(LogiaBypassResult {
                source: LogiaBypassSource::SeastoneWeapon,
                fraction: 1.0,
            })
        } else {
            None
        };

        if !victim.has_fruit() || (!is_gear_blow && !is_seastone_arrow) {
            return SeastoneStrikeOutcome {
                suppress_ticks_applied: 0,
                stun_ticks_applied: 0,
                logia_bypass,
            };
        }

        let (suppress_ticks, stun_ticks) =
            rule_override.unwrap_or((cfg.hit_suppress_ticks, cfg.jitte_stun_ticks));

        if suppress_ticks > 0 {
            victim_cuffs.apply_suppression(now, suppress_ticks);
        }
        let stun_applied = if is_jitte && stun_ticks > 0 {
            stun_ticks
        } else {
            0
        };

        SeastoneStrikeOutcome {
            suppress_ticks_applied: suppress_ticks.max(0),
            stun_ticks_applied: stun_applied,
            logia_bypass,
        }
    }

    /// Scales incoming Devil Fruit attack damage for non-fruit wearers of Seastone armor (`SeastoneCombat.onIncomingDamage`):
    /// `amount * max(0.0, 1.0 - pieces * armor_fruit_damage_reduction)`.
    pub fn scale_incoming_fruit_damage(
        victim_has_fruit: bool,
        seastone_armor_pieces: u32,
        incoming_damage: f32,
        cfg: &SeastoneCombatConfig,
    ) -> f32 {
        if victim_has_fruit || seastone_armor_pieces == 0 {
            return incoming_damage;
        }
        let factor = (1.0
            - (seastone_armor_pieces.min(4) as f32) * cfg.armor_fruit_damage_reduction)
            .max(0.0);
        incoming_damage * factor
    }
}

/// Seastone Jitte 4-step combo & pin specification (`com.devilfruits.core.seastone.JitteTechniques`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JitteComboSpec {
    pub cooldown_ticks: i32,
    pub strike_ticks: [i32; 4],
    pub busy_ticks: [i32; 4],
    pub damage_shares: [f32; 4],
    pub reach: f64,
    pub arc_degrees: f64,
    pub seal_ticks: i32,
    pub knockdown_ticks: i32,
}

impl Default for JitteComboSpec {
    fn default() -> Self {
        Self {
            cooldown_ticks: 10,
            strike_ticks: [4, 4, 5, 7],
            busy_ticks: [7, 7, 8, 15],
            damage_shares: [0.72, 0.72, 0.80, 1.05],
            reach: 3.3,
            arc_degrees: 70.0,
            seal_ticks: 40,
            knockdown_ticks: 20,
        }
    }
}

impl JitteComboSpec {
    /// Computes damage share, extra knockback, and optional smash launch velocity for `step` (`0..=3`).
    pub fn step_profile(&self, step: usize, look_dir: DVec3) -> (f32, f32, Option<DVec3>, i32) {
        let s = step.min(3);
        let share = self.damage_shares[s];
        let kb = match s {
            3 => 0.4,
            2 => 0.2,
            _ => 0.0,
        };
        let smash_launch = if s == 3 {
            Some(DVec3::new(look_dir.x * 0.15, -0.6, look_dir.z * 0.15))
        } else {
            None
        };
        let stun = if s == 3 { self.knockdown_ticks } else { 0 };
        (share, kb, smash_launch, stun)
    }
}

/// Seastone Jitte Pin held technique specification (`JitteTechniques.Pin`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct JittePinSpec {
    pub cooldown_ticks: i32,
    pub reach: f64,
    pub leash: f64,
    pub seconds: f32,
    pub damage_per_second: f32,
    pub durability_per_second: i32,
    pub max_struggle_cut: f32,
    pub struggle_presses: f32,
}

impl Default for JittePinSpec {
    fn default() -> Self {
        Self {
            cooldown_ticks: 160,
            reach: 3.0,
            leash: 3.5,
            seconds: 4.0,
            damage_per_second: 1.0,
            durability_per_second: 2,
            max_struggle_cut: 0.5,
            struggle_presses: 16.0,
        }
    }
}

impl JittePinSpec {
    pub fn pin_ticks(&self) -> i32 {
        (self.seconds * 20.0).round() as i32
    }

    pub fn max_ticks(&self) -> i32 {
        self.pin_ticks() + 2
    }
}
