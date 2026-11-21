//! Core fruit rules, player fruit persistence, world uniqueness, mastery curves,
//! and water/seastone weakness logic.
//!
//! Mirrors `CoreConfig.java`, `DFGameRules.java`, `PlayerFruitData.java`,
//! `WorldFruitData.java`, `Mastery.java`, `FruitRules.java`, and `Weakness.java`.

use std::collections::BTreeMap;

use glam::DVec3;
use horizon_world::grandline_hooks::{
    SeastoneConfig as WorldSeastoneConfig, SeastoneTouchState, WEAKNESS_DEFAULT_SINK_SPEED,
    WEAKNESS_EFFECT_DURATION_TICKS, WEAKNESS_EFFECT_INTERVAL_TICKS,
    WEAKNESS_MINING_FATIGUE_AMPLIFIER, WEAKNESS_SINK_HORIZONTAL_DRAG,
    WEAKNESS_SLOWNESS_AMPLIFIER, WEAKNESS_SUBMERSION_THRESHOLD, WEAKNESS_WEAKNESS_AMPLIFIER,
    WeaknessDebuff, WeaknessEffectKind,
};
use serde::{Deserialize, Serialize};

use crate::core::fruit::{EnergyRules, FruitId};

/// Canonical player identifier (128-bit UUID bytes matching Minecraft & Horizon).
pub type PlayerUuid = [u8; 16];

/// Exact mastery level cap (`Mastery.MAX_LEVEL`).
pub const MAX_MASTERY: u32 = 10;

/// Exact XP needed for each level transition 1->2 through 9->10 (`Mastery.XP_CURVE`).
pub const MASTERY_XP_CURVE: [f32; 9] = [
    100.0, 180.0, 300.0, 450.0, 650.0, 900.0, 1200.0, 1600.0, 2100.0,
];

/// Reincarnation behavior when a player loses a Devil Fruit on death (`CoreConfig.ReincarnationMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReincarnationMode {
    #[default]
    NearbyChest,
    Drop,
    LootPool,
}

/// Terrain debris cleanup mode (`CoreConfig.DebrisMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebrisMode {
    #[default]
    Temporary,
    Permanent,
    None,
}

/// World-level configuration matching `com.devilfruits.core.config.CoreConfig`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreConfig {
    pub one_per_world: bool,
    pub lose_on_death: bool,
    pub reincarnation_mode: ReincarnationMode,
    pub reincarnation_radius: i32,
    pub keep_mastery_on_loss: bool,
    pub second_fruit_kills: bool,

    pub water_weakness: bool,
    pub sink_speed: f64,
    pub seastone_weakness: bool,

    pub combat_seconds: i32,
    pub creative_free_energy: bool,
    pub xp_multiplier: f32,
    pub xp_per_hit: f32,
    pub damage_per_level: f32,
    pub range_per_level: f32,
    pub cooldown_reduction_per_level: f32,
    pub global_damage_multiplier: f32,

    pub revert_on_death: bool,
    pub revert_in_water: bool,
    pub revert_on_depletion: bool,

    pub block_destruction: bool,
    pub respect_mob_griefing: bool,
    pub debris_mode: DebrisMode,
    pub restore_delay_seconds: i32,
    pub max_blocks_per_cast: usize,
    pub max_debris_entities_per_cast: usize,
    pub max_block_hardness: f32,
    pub drop_blocks: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            one_per_world: true,
            lose_on_death: true,
            reincarnation_mode: ReincarnationMode::NearbyChest,
            reincarnation_radius: 64,
            keep_mastery_on_loss: false,
            second_fruit_kills: true,
            water_weakness: true,
            sink_speed: WEAKNESS_DEFAULT_SINK_SPEED,
            seastone_weakness: true,
            combat_seconds: 6,
            creative_free_energy: true,
            xp_multiplier: 1.0,
            xp_per_hit: 2.0,
            damage_per_level: 0.12,
            range_per_level: 0.06,
            cooldown_reduction_per_level: 0.03,
            global_damage_multiplier: 1.0,
            revert_on_death: true,
            revert_in_water: true,
            revert_on_depletion: true,
            block_destruction: true,
            respect_mob_griefing: true,
            debris_mode: DebrisMode::Temporary,
            restore_delay_seconds: 12,
            max_blocks_per_cast: 160,
            max_debris_entities_per_cast: 40,
            max_block_hardness: 6.0,
            drop_blocks: false,
        }
    }
}

impl CoreConfig {
    pub fn combat_ticks(&self) -> i64 {
        (self.combat_seconds.max(0) as i64) * 20
    }

    pub fn restore_delay_ticks(&self) -> i64 {
        (self.restore_delay_seconds.max(0) as i64) * 20
    }
}

/// Custom gamerules (`com.devilfruits.core.rules.DFGameRules`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DFGameRules {
    /// `keepFruit`: keeps Devil Fruit on normal deaths (still killed/lost on `SECOND_FRUIT`).
    pub keep_fruit: bool,
    /// Mirrors vanilla `mobGriefing` for `TerrainDestruction`.
    pub mob_griefing: bool,
}

impl DFGameRules {
    pub fn vanilla_default() -> Self {
        Self {
            keep_fruit: false,
            mob_griefing: true,
        }
    }
}

/// Ownership state of a unique Devil Fruit in a world (`WorldFruitData.State`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldFruitState {
    /// Available in loot tables / gacha caskets.
    Free,
    /// Exists in the world as an un-eaten item/entity/chest stack.
    Item,
    /// Currently eaten and owned by a player.
    Eaten,
}

/// World record for a single Devil Fruit (`WorldFruitData.Entry`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFruitEntry {
    pub state: WorldFruitState,
    pub owner: Option<PlayerUuid>,
}

/// Persistent world uniqueness ledger (`com.devilfruits.core.rules.WorldFruitData`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFruitData {
    pub entries: BTreeMap<FruitId, WorldFruitEntry>,
}

impl WorldFruitData {
    pub fn state(&self, fruit: FruitId) -> WorldFruitState {
        self.entries
            .get(&fruit)
            .map(|e| e.state)
            .unwrap_or(WorldFruitState::Free)
    }

    pub fn is_free(&self, fruit: FruitId) -> bool {
        self.state(fruit) == WorldFruitState::Free
    }

    pub fn owner(&self, fruit: FruitId) -> Option<PlayerUuid> {
        self.entries.get(&fruit).and_then(|e| e.owner)
    }

    /// Claims a fruit as an existing physical item in the world. Returns `false` if not `Free`.
    pub fn claim_item(&mut self, fruit: FruitId) -> bool {
        if !self.is_free(fruit) {
            return false;
        }
        self.entries.insert(
            fruit,
            WorldFruitEntry {
                state: WorldFruitState::Item,
                owner: None,
            },
        );
        true
    }

    /// Marks a fruit as eaten by `owner`.
    pub fn mark_eaten(&mut self, fruit: FruitId, owner: PlayerUuid) {
        self.entries.insert(
            fruit,
            WorldFruitEntry {
                state: WorldFruitState::Eaten,
                owner: Some(owner),
            },
        );
    }

    /// Releases a fruit back to `Free` (reincarnated into loot pool / gacha).
    pub fn release(&mut self, fruit: FruitId) {
        self.entries.remove(&fruit);
    }

    /// Releases whatever fruit `owner` currently holds.
    pub fn release_owner(&mut self, owner: PlayerUuid) -> Option<FruitId> {
        let owned = self
            .entries
            .iter()
            .find_map(|(&fruit, e)| (e.owner == Some(owner)).then_some(fruit));
        if let Some(fruit) = owned {
            self.entries.remove(&fruit);
        }
        owned
    }

    /// Returns all fruits currently `Free`.
    pub fn free_fruits(&self) -> Vec<FruitId> {
        FruitId::ALL
            .iter()
            .copied()
            .filter(|&f| self.is_free(f))
            .collect()
    }
}

/// Active cooldown stamp (`PlayerFruitData.Cooldown`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cooldown {
    pub end_tick: i64,
    pub total_ticks: i32,
}

impl Cooldown {
    pub const fn new(end_tick: i64, total_ticks: i32) -> Self {
        Self {
            end_tick,
            total_ticks,
        }
    }

    pub fn remaining(self, now: i64) -> i64 {
        (self.end_tick - now).max(0)
    }

    pub fn is_active(self, now: i64) -> bool {
        now < self.end_tick
    }

    /// Normalized remaining fraction in `0.0..=1.0`.
    pub fn fraction(self, now: i64) -> f32 {
        if self.total_ticks <= 0 {
            return 0.0;
        }
        (self.remaining(now) as f32 / self.total_ticks as f32).clamp(0.0, 1.0)
    }
}

/// Persistent per-player Devil Fruit attachment (`com.devilfruits.core.data.PlayerFruitData`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerFruitData {
    pub fruit_id: Option<FruitId>,
    pub mastery: u32,
    pub xp: f32,
    pub energy: f32,
    pub active_form: String,
    pub selected_ability: Option<String>,
    pub cooldowns: BTreeMap<String, Cooldown>,
    pub last_combat_tick: i64,
    /// Per-fruit saved mastery when `keep_mastery_on_loss` is enabled.
    pub saved_mastery: BTreeMap<FruitId, (u32, f32)>,
}

impl Default for PlayerFruitData {
    fn default() -> Self {
        Self {
            fruit_id: None,
            mastery: 1,
            xp: 0.0,
            energy: 0.0,
            active_form: "base".to_string(),
            selected_ability: None,
            cooldowns: BTreeMap::new(),
            last_combat_tick: i64::MIN / 4,
            saved_mastery: BTreeMap::new(),
        }
    }
}

impl PlayerFruitData {
    pub fn has_fruit(&self) -> bool {
        self.fruit_id.is_some()
    }

    pub fn energy_rules(&self) -> EnergyRules {
        self.fruit_id
            .map(|id| id.meta().energy)
            .unwrap_or(EnergyRules::DEFAULT)
    }

    pub fn max_energy(&self) -> f32 {
        self.energy_rules().max_energy(self.mastery)
    }

    pub fn in_combat(&self, now: i64, cfg: &CoreConfig) -> bool {
        now.saturating_sub(self.last_combat_tick) < cfg.combat_ticks()
    }

    pub fn mark_combat(&mut self, now: i64) {
        self.last_combat_tick = now;
    }

    pub fn spend_energy(&mut self, amount: f32, creative: bool, cfg: &CoreConfig) -> bool {
        if amount <= 0.0 || (creative && cfg.creative_free_energy) {
            return true;
        }
        if self.energy + 1e-4 < amount {
            return false;
        }
        self.energy = (self.energy - amount).max(0.0);
        true
    }

    pub fn regen_energy(&mut self, now: i64, cfg: &CoreConfig, multiplier: f32) {
        if !self.has_fruit() || multiplier <= 0.0 {
            return;
        }
        let max_e = self.max_energy();
        let per_tick = self.energy_rules().regen_per_tick(self.in_combat(now, cfg)) * multiplier;
        self.energy = (self.energy + per_tick).clamp(0.0, max_e);
    }

    pub fn set_cooldown(&mut self, ability: impl Into<String>, now: i64, duration_ticks: i32) {
        if duration_ticks > 0 {
            self.cooldowns.insert(
                ability.into(),
                Cooldown::new(now + duration_ticks as i64, duration_ticks),
            );
        }
    }

    pub fn cooldown_remaining(&self, ability: &str, now: i64) -> i64 {
        self.cooldowns
            .get(ability)
            .map(|c| c.remaining(now))
            .unwrap_or(0)
    }

    pub fn prune_cooldowns(&mut self, now: i64) {
        self.cooldowns.retain(|_, cd| cd.is_active(now));
    }

    pub fn clear_cooldowns(&mut self) {
        self.cooldowns.clear();
    }
}

/// Mastery scaling & leveling formulas (`com.devilfruits.core.rules.Mastery`).
pub struct Mastery;

impl Mastery {
    /// XP required to advance from `level` (1..=9) to `level + 1`. Returns `0.0` at level 10.
    pub fn xp_for_next_level(level: u32) -> f32 {
        if level == 0 || level >= MAX_MASTERY {
            0.0
        } else {
            MASTERY_XP_CURVE[(level - 1) as usize]
        }
    }

    /// Grants raw mastery XP (scaled by `cfg.xp_multiplier`), returning the number of levels gained.
    pub fn award_xp(data: &mut PlayerFruitData, raw_xp: f32, cfg: &CoreConfig) -> u32 {
        if !data.has_fruit() || raw_xp <= 0.0 || data.mastery >= MAX_MASTERY {
            return 0;
        }
        let scaled = raw_xp * cfg.xp_multiplier.max(0.0);
        if scaled <= 0.0 {
            return 0;
        }
        data.xp += scaled;
        let mut gained = 0u32;
        while data.mastery < MAX_MASTERY {
            let needed = Self::xp_for_next_level(data.mastery);
            if data.xp + 1e-4 < needed {
                break;
            }
            data.xp = (data.xp - needed).max(0.0);
            data.mastery += 1;
            gained += 1;
        }
        if data.mastery >= MAX_MASTERY {
            data.mastery = MAX_MASTERY;
            data.xp = 0.0;
        }
        if gained > 0 {
            let new_max = data.max_energy();
            if data.energy > new_max {
                data.energy = new_max;
            }
        }
        gained
    }

    /// Damage multiplier at `mastery`: `(1.0 + damage_per_level * (mastery - 1)) * global_damage_multiplier`.
    pub fn damage_multiplier(mastery: u32, cfg: &CoreConfig) -> f32 {
        let lvl_bonus = (mastery.clamp(1, MAX_MASTERY) - 1) as f32;
        (1.0 + cfg.damage_per_level * lvl_bonus) * cfg.global_damage_multiplier
    }

    /// Range multiplier at `mastery`: `1.0 + range_per_level * (mastery - 1)`.
    pub fn range_multiplier(mastery: u32, cfg: &CoreConfig) -> f32 {
        let lvl_bonus = (mastery.clamp(1, MAX_MASTERY) - 1) as f32;
        1.0 + cfg.range_per_level * lvl_bonus
    }

    /// Cooldown multiplier at `mastery`: `max(0.1, 1.0 - cooldown_reduction_per_level * (mastery - 1))`.
    pub fn cooldown_multiplier(mastery: u32, cfg: &CoreConfig) -> f32 {
        let lvl_bonus = (mastery.clamp(1, MAX_MASTERY) - 1) as f32;
        (1.0 - cfg.cooldown_reduction_per_level * lvl_bonus).max(0.1)
    }

    /// Scales an integer cooldown by mastery, rounding to at least 1 tick if base > 0.
    pub fn scale_cooldown(base_ticks: i32, mastery: u32, cfg: &CoreConfig) -> i32 {
        if base_ticks <= 0 {
            return 0;
        }
        ((base_ticks as f32) * Self::cooldown_multiplier(mastery, cfg))
            .round()
            .max(1.0) as i32
    }

    /// Mastery progress fraction toward next level in `0.0..=1.0`.
    pub fn progress_fraction(mastery: u32, xp: f32) -> f32 {
        if mastery >= MAX_MASTERY {
            1.0
        } else {
            let need = Self::xp_for_next_level(mastery).max(1.0);
            (xp / need).clamp(0.0, 1.0)
        }
    }
}

/// Outcome of eating a Devil Fruit item (`FruitRules.eat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EatOutcome {
    /// Successfully granted the fruit.
    Granted(FruitId),
    /// Player already has a fruit and `second_fruit_kills = true` -> deals `f32::MAX` `SECOND_FRUIT` lethal damage.
    SecondFruitFatal,
    /// Player already has a fruit and `second_fruit_kills = false` -> rejected harmlessly.
    AlreadyHasFruit,
    /// `one_per_world = true` and another player already owns this fruit.
    WorldDuplicateBlocked,
}

/// Outcome of player death with respect to fruit retention/reincarnation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathFruitOutcome {
    NoFruit,
    Kept,
    LostAndReincarnated {
        fruit: FruitId,
        mode: ReincarnationMode,
    },
}

/// Rules engine governing eating, giving, removing, and death reincarnation (`FruitRules.java`).
pub struct FruitRules;

impl FruitRules {
    /// Player eats a Devil Fruit item in gameplay.
    pub fn try_eat(
        player_id: PlayerUuid,
        player: &mut PlayerFruitData,
        world: &mut WorldFruitData,
        fruit: FruitId,
        cfg: &CoreConfig,
    ) -> EatOutcome {
        if player.has_fruit() {
            return if cfg.second_fruit_kills {
                EatOutcome::SecondFruitFatal
            } else {
                EatOutcome::AlreadyHasFruit
            };
        }
        if cfg.one_per_world {
            if let Some(owner) = world.owner(fruit) {
                if owner != player_id {
                    return EatOutcome::WorldDuplicateBlocked;
                }
            }
        }
        Self::grant(player_id, player, world, fruit, cfg);
        EatOutcome::Granted(fruit)
    }

    /// Force-grants a fruit (used by eat and `/devilfruit give`).
    pub fn grant(
        player_id: PlayerUuid,
        player: &mut PlayerFruitData,
        world: &mut WorldFruitData,
        fruit: FruitId,
        cfg: &CoreConfig,
    ) {
        if let Some(old) = player.fruit_id {
            if cfg.keep_mastery_on_loss {
                player.saved_mastery.insert(old, (player.mastery, player.xp));
            }
            if world.owner(old) == Some(player_id) {
                world.release(old);
            }
        }
        let (mastery, xp) = if cfg.keep_mastery_on_loss {
            player.saved_mastery.get(&fruit).copied().unwrap_or((1, 0.0))
        } else {
            (1, 0.0)
        };
        let meta = fruit.meta();
        player.fruit_id = Some(fruit);
        player.mastery = mastery.clamp(1, MAX_MASTERY);
        player.xp = xp;
        player.active_form = meta.default_form.to_string();
        player.selected_ability = meta.base_abilities.first().map(|s| (*s).to_string());
        player.cooldowns.clear();
        player.energy = player.max_energy();
        world.mark_eaten(fruit, player_id);
    }

    /// Removes a player's current fruit and releases it in `WorldFruitData`.
    pub fn remove(
        player_id: PlayerUuid,
        player: &mut PlayerFruitData,
        world: &mut WorldFruitData,
        cfg: &CoreConfig,
    ) -> Option<FruitId> {
        let old = player.fruit_id.take()?;
        if cfg.keep_mastery_on_loss {
            player.saved_mastery.insert(old, (player.mastery, player.xp));
        }
        player.mastery = 1;
        player.xp = 0.0;
        player.energy = 0.0;
        player.active_form = "base".to_string();
        player.selected_ability = None;
        player.cooldowns.clear();
        if world.owner(old) == Some(player_id) {
            world.release(old);
        }
        Some(old)
    }

    /// Applies death rules: `keepFruit` keeps the fruit EXCEPT on `SECOND_FRUIT` death.
    pub fn on_player_death(
        player_id: PlayerUuid,
        player: &mut PlayerFruitData,
        world: &mut WorldFruitData,
        caused_by_second_fruit: bool,
        cfg: &CoreConfig,
        rules: DFGameRules,
    ) -> DeathFruitOutcome {
        let Some(fruit) = player.fruit_id else {
            return DeathFruitOutcome::NoFruit;
        };
        let should_lose = caused_by_second_fruit || (cfg.lose_on_death && !rules.keep_fruit);
        if !should_lose {
            if cfg.revert_on_death {
                player.active_form = fruit.meta().default_form.to_string();
            }
            return DeathFruitOutcome::Kept;
        }
        Self::remove(player_id, player, world, cfg);
        match cfg.reincarnation_mode {
            ReincarnationMode::NearbyChest | ReincarnationMode::Drop => {
                world.claim_item(fruit);
            }
            ReincarnationMode::LootPool => {
                world.release(fruit);
            }
        }
        DeathFruitOutcome::LostAndReincarnated {
            fruit,
            mode: cfg.reincarnation_mode,
        }
    }
}

/// Complete suppression / weakness snapshot computed each tick (`Weakness.java`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaknessStatus {
    pub submerged_in_water: bool,
    pub touching_seastone: bool,
    pub suppressed: bool,
}

/// Water & Seastone weakness engine (`com.devilfruits.core.rules.Weakness`).
pub struct Weakness;

impl Weakness {
    /// Checks whether submersion depth triggers Devil Fruit water weakness (`> 1.0` block unless `on_water`).
    pub fn is_submerged(water_depth: f64, on_water_override: bool, cfg: &CoreConfig) -> bool {
        cfg.water_weakness && !on_water_override && water_depth > WEAKNESS_SUBMERSION_THRESHOLD
    }

    /// Evaluates Seastone contact suppression mirroring `Weakness.touchesSeastone`.
    pub fn touches_seastone(state: SeastoneTouchState, cfg: WorldSeastoneConfig) -> bool {
        if !cfg.seastone_weakness {
            return false;
        }
        if state.has_suppressed_effect
            || state.standing_on_seastone_block
            || state.inside_seastone_block
        {
            return true;
        }
        if cfg.armor_suppresses_fruit_user && state.seastone_armor_pieces > 0 {
            return true;
        }
        let main_suppresses = state.main_hand_seastone
            && !(cfg.jitte_safe_handle && state.main_hand_is_jitte)
            && (cfg.gear_suppresses_holder || !state.main_hand_is_gear);
        let off_suppresses = state.off_hand_seastone
            && !(cfg.jitte_safe_handle && state.off_hand_is_jitte)
            && (cfg.gear_suppresses_holder || !state.off_hand_is_gear);
        main_suppresses || off_suppresses
    }

    /// Combined weakness evaluation. Creative/Spectator players are immune to water/seastone suppression.
    pub fn evaluate(
        has_fruit: bool,
        creative_or_spectator: bool,
        water_depth: f64,
        on_water_override: bool,
        seastone_touch: SeastoneTouchState,
        cfg: &CoreConfig,
        seastone_cfg: WorldSeastoneConfig,
    ) -> WeaknessStatus {
        if !has_fruit || creative_or_spectator {
            return WeaknessStatus::default();
        }
        let submerged_in_water = Self::is_submerged(water_depth, on_water_override, cfg);
        let mut scfg = seastone_cfg;
        scfg.seastone_weakness = scfg.seastone_weakness && cfg.seastone_weakness;
        let touching_seastone = Self::touches_seastone(seastone_touch, scfg);
        WeaknessStatus {
            submerged_in_water,
            touching_seastone,
            suppressed: submerged_in_water || touching_seastone,
        }
    }

    /// Exact sinking velocity transformation (`Weakness.applySinking`):
    /// `vx *= 0.6`, `vz *= 0.6`, `vy = min(vy, -sink_speed)`.
    pub fn apply_sinking(velocity: DVec3, cfg: &CoreConfig) -> DVec3 {
        let sink = cfg.sink_speed.clamp(0.0, 1.0);
        DVec3::new(
            velocity.x * WEAKNESS_SINK_HORIZONTAL_DRAG,
            velocity.y.min(-sink),
            velocity.z * WEAKNESS_SINK_HORIZONTAL_DRAG,
        )
    }

    /// Returns the 3 canonical debuffs refreshed every 10 ticks while suppressed (`Weakness.tickServer`).
    pub fn tick_debuffs(tick: i64, suppressed: bool) -> Option<[WeaknessDebuff; 3]> {
        if !suppressed || (tick % WEAKNESS_EFFECT_INTERVAL_TICKS as i64) != 0 {
            return None;
        }
        Some([
            WeaknessDebuff {
                kind: WeaknessEffectKind::MovementSlowdown,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_SLOWNESS_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
            WeaknessDebuff {
                kind: WeaknessEffectKind::Weakness,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_WEAKNESS_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
            WeaknessDebuff {
                kind: WeaknessEffectKind::DigSlowdown,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_MINING_FATIGUE_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
        ])
    }
}
