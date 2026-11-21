//! Devil Fruit Gacha casket & gold key system, 4-tier weighted rolling
//! (`LEGENDARY = 2`, `RARE = 8`, `UNCOMMON = 25`, `COMMON = 65`), world-uniqueness
//! filtering, and cinematic reveal pending queue.
//!
//! Mirrors `core/gacha/{GachaTier,GachaRoll,CasketOpening,CasketItem,CasketKey,GoldKeyItem,CasketRevealPayload}.java`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::rules::{CoreConfig, PlayerUuid, WorldFruitData};

/// Rarity tiers for Devil Fruit Casket gacha (`com.devilfruits.core.gacha.GachaTier`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GachaTier {
    Legendary = 0,
    Rare = 1,
    Uncommon = 2,
    Common = 3,
}

impl GachaTier {
    pub const ALL: [Self; 4] = [Self::Legendary, Self::Rare, Self::Uncommon, Self::Common];

    /// Default tier weight matching `GachaTier.defaultWeight()`:
    /// Legendary `2`, Rare `8`, Uncommon `25`, Common `65`.
    pub const fn default_weight(self) -> u32 {
        match self {
            Self::Legendary => 2,
            Self::Rare => 8,
            Self::Uncommon => 25,
            Self::Common => 65,
        }
    }

    /// Translation key (`gacha.devilfruits.tier.<name>`).
    pub const fn translation_key(self) -> &'static str {
        match self {
            Self::Legendary => "gacha.devilfruits.tier.legendary",
            Self::Rare => "gacha.devilfruits.tier.rare",
            Self::Uncommon => "gacha.devilfruits.tier.uncommon",
            Self::Common => "gacha.devilfruits.tier.common",
        }
    }

    /// RGB color matching `GachaTier.color()` (`GOLD`, `LIGHT_PURPLE`, `AQUA`, `GREEN`).
    pub const fn color(self) -> u32 {
        match self {
            Self::Legendary => 0xFF_AA_00,
            Self::Rare => 0xFF_55_FF,
            Self::Uncommon => 0x55_FF_FF,
            Self::Common => 0x55_FF_55,
        }
    }

    pub const fn at_least(self, min_tier: Self) -> bool {
        (self as u8) <= (min_tier as u8)
    }
}

/// Configurable gacha & casket opening rules (`CoreConfig` gacha section).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GachaConfig {
    pub weight_legendary: u32,
    pub weight_rare: u32,
    pub weight_uncommon: u32,
    pub weight_common: u32,
    pub casket_key_consumed: bool,
    pub casket_reveal_seconds: f32,
    pub casket_cinematic: bool,
    pub announce_min_tier: GachaTier,
}

impl Default for GachaConfig {
    fn default() -> Self {
        Self {
            weight_legendary: GachaTier::Legendary.default_weight(),
            weight_rare: GachaTier::Rare.default_weight(),
            weight_uncommon: GachaTier::Uncommon.default_weight(),
            weight_common: GachaTier::Common.default_weight(),
            casket_key_consumed: true,
            casket_reveal_seconds: 3.5,
            casket_cinematic: true,
            announce_min_tier: GachaTier::Rare,
        }
    }
}

impl GachaConfig {
    pub const fn tier_weight(&self, tier: GachaTier) -> u32 {
        match tier {
            GachaTier::Legendary => self.weight_legendary,
            GachaTier::Rare => self.weight_rare,
            GachaTier::Uncommon => self.weight_uncommon,
            GachaTier::Common => self.weight_common,
        }
    }

    pub fn reveal_ticks(&self) -> i32 {
        ((self.casket_reveal_seconds * 20.0).round() as i32).max(1)
    }

    pub const fn announces(&self, tier: GachaTier) -> bool {
        tier.at_least(self.announce_min_tier)
    }
}

/// Weighted gacha candidate (`GachaRoll.Candidate<FruitId>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GachaCandidate {
    pub fruit: FruitId,
    pub tier: GachaTier,
    pub weight: u32,
}

/// Two-stage weighted gacha roller (`com.devilfruits.core.gacha.GachaRoll`).
pub struct GachaRoll;

impl GachaRoll {
    /// Collects all eligible playable Devil Fruits that are currently `Free` in `world` (when `one_per_world` is enabled).
    pub fn candidates(world: &WorldFruitData, core_cfg: &CoreConfig) -> Vec<GachaCandidate> {
        let mut out = Vec::new();
        for fruit in FruitId::ALL {
            let meta = fruit.meta();
            if meta.loot_weight > 0 && (!core_cfg.one_per_world || world.is_free(fruit)) {
                out.push(GachaCandidate {
                    fruit,
                    tier: meta.gacha_tier,
                    weight: meta.loot_weight,
                });
            }
        }
        out
    }

    pub fn is_rollable(candidates: &[GachaCandidate], gacha_cfg: &GachaConfig) -> bool {
        candidates
            .iter()
            .any(|c| c.weight > 0 && gacha_cfg.tier_weight(c.tier) > 0)
    }

    /// Exact two-stage roll matching `GachaRoll.roll`:
    /// 1. Group non-zero candidates by [`GachaTier`] and pick a populated tier using `gacha_cfg.tier_weight(tier)`.
    /// 2. Pick a fruit within that tier weighted by `candidate.weight`.
    pub fn roll_with_seed(
        candidates: &[GachaCandidate],
        gacha_cfg: &GachaConfig,
        seed: u64,
    ) -> Option<GachaCandidate> {
        let mut by_tier: BTreeMap<GachaTier, Vec<GachaCandidate>> = BTreeMap::new();
        for &c in candidates {
            if c.weight > 0 {
                by_tier.entry(c.tier).or_default().push(c);
            }
        }
        let total_tier_weight: u32 = by_tier
            .keys()
            .map(|&t| gacha_cfg.tier_weight(t))
            .sum();
        if total_tier_weight == 0 {
            return None;
        }

        // SplitMix64 step 1 for tier pick, step 2 for intra-tier pick.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let r1 = (z ^ (z >> 31)) as u32;

        let mut tier_pick = (r1 % total_tier_weight) as i64;
        let mut chosen_list = None;
        for (&tier, list) in &by_tier {
            tier_pick -= gacha_cfg.tier_weight(tier) as i64;
            if tier_pick < 0 {
                chosen_list = Some(list);
                break;
            }
        }
        let tier_candidates = chosen_list?;
        let total_fruit_weight: u32 = tier_candidates.iter().map(|c| c.weight).sum();
        if total_fruit_weight == 0 {
            return None;
        }

        let mut z2 = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z2 = (z2 ^ (z2 >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z2 = (z2 ^ (z2 >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let r2 = (z2 ^ (z2 >> 31)) as u32;

        let mut fruit_pick = (r2 % total_fruit_weight) as i64;
        for &c in tier_candidates {
            fruit_pick -= c.weight as i64;
            if fruit_pick < 0 {
                return Some(c);
            }
        }
        tier_candidates.last().copied()
    }
}

/// Outcome of attempting to open a Devil Fruit Casket (`CasketOpening.Outcome`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CasketOpenOutcome {
    Opened {
        fruit: FruitId,
        tier: GachaTier,
        deliver_at_tick: i64,
    },
    NoCasket,
    NoKey,
    Busy,
    NoFruit,
}

/// Pending casket reveal entry (`CasketOpening.Pending`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingCasketDelivery {
    pub fruit: FruitId,
    pub tier: GachaTier,
    pub deliver_at_tick: i64,
}

/// Server-side casket reveal & delivery tracker (`com.devilfruits.core.gacha.CasketOpening`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasketOpeningTracker {
    pub pending: BTreeMap<PlayerUuid, PendingCasketDelivery>,
}

impl CasketOpeningTracker {
    pub fn is_pending(&self, player: PlayerUuid) -> bool {
        self.pending.contains_key(&player)
    }

    /// Opens a casket with a Gold Key (`CasketOpening.open`), claiming the rolled fruit in `world`.
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        &mut self,
        player: PlayerUuid,
        has_casket_in_main_hand: bool,
        has_key_in_off_hand: bool,
        world: &mut WorldFruitData,
        seed: u64,
        now: i64,
        core_cfg: &CoreConfig,
        gacha_cfg: &GachaConfig,
    ) -> CasketOpenOutcome {
        if !has_casket_in_main_hand {
            return CasketOpenOutcome::NoCasket;
        }
        if !has_key_in_off_hand {
            return CasketOpenOutcome::NoKey;
        }
        if self.is_pending(player) {
            return CasketOpenOutcome::Busy;
        }
        let candidates = GachaRoll::candidates(world, core_cfg);
        let Some(rolled) = GachaRoll::roll_with_seed(&candidates, gacha_cfg, seed) else {
            return CasketOpenOutcome::NoFruit;
        };
        if core_cfg.one_per_world && !world.claim_item(rolled.fruit) {
            return CasketOpenOutcome::NoFruit;
        }
        let deliver_at_tick = now + gacha_cfg.reveal_ticks() as i64;
        self.pending.insert(
            player,
            PendingCasketDelivery {
                fruit: rolled.fruit,
                tier: rolled.tier,
                deliver_at_tick,
            },
        );
        CasketOpenOutcome::Opened {
            fruit: rolled.fruit,
            tier: rolled.tier,
            deliver_at_tick,
        }
    }

    /// Ticks pending casket reveals and returns all deliveries ready at `now` (`CasketOpening.onServerTick`).
    pub fn tick_deliveries(&mut self, now: i64) -> Vec<(PlayerUuid, PendingCasketDelivery)> {
        let ready_players: Vec<PlayerUuid> = self
            .pending
            .iter()
            .filter_map(|(&p, d)| (d.deliver_at_tick <= now).then_some(p))
            .collect();
        let mut out = Vec::with_capacity(ready_players.len());
        for p in ready_players {
            if let Some(d) = self.pending.remove(&p) {
                out.push((p, d));
            }
        }
        out
    }

    /// Immediately delivers any pending casket for `player` (e.g. on logout/dimension change).
    pub fn deliver_now(&mut self, player: PlayerUuid) -> Option<PendingCasketDelivery> {
        self.pending.remove(&player)
    }
}
