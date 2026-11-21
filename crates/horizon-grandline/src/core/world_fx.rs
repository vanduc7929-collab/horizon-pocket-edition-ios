//! World destruction, craters, temporary block transforms & timed block restore
//! (`BlockRestoreData`, ice/magma/wax decay timers), flying debris entities,
//! screen shake, and server VFX dispatch queue.
//!
//! Mirrors `core/world/{TerrainDestruction,BlockRestoreData,DebrisEntity,AbilityEntity}.java`
//! and `core/fx/{CoreFx,ServerFx,ExplosionProfile,BombExplosionPayload}.java`.

use std::collections::BTreeMap;

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::ability::CastBudget;
use crate::core::rules::{CoreConfig, DFGameRules, DebrisMode};
use crate::core::seastone::is_seastone_block;

/// Canonical core VFX resource identifiers (`com.devilfruits.core.fx.CoreFx`).
pub mod core_fx {
    pub const AWAKEN: &str = "devilfruits:awaken";
    pub const LEVEL_UP: &str = "devilfruits:level_up";
    pub const REINCARNATE: &str = "devilfruits:reincarnate";
    pub const BLOCK_BURST: &str = "devilfruits:block_burst";
    pub const SUPPRESSED: &str = "devilfruits:suppressed";
    pub const STUN: &str = "devilfruits:stun";
    pub const SHAKE: &str = "devilfruits:shake";
    pub const FORM_CHANGE: &str = "devilfruits:form_change";
    pub const STRUCTURE_IMPACT: &str = "devilfruits:structure_impact";
    pub const STRUCTURE_DECAL: &str = "devilfruits:structure_decal";
    pub const EXPLOSION: &str = "devilfruits:explosion";
}

/// Blocks immune to Devil Fruit terrain destruction (`DFTags.DESTRUCTION_IMMUNE` & `Poneglyphs.UNBREAKABLE`).
pub const DESTRUCTION_IMMUNE_BLOCKS: [&str; 9] = [
    "minecraft:bedrock",
    "minecraft:barrier",
    "minecraft:end_portal_frame",
    "minecraft:end_portal",
    "minecraft:command_block",
    "minecraft:budding_amethyst",
    "devilfruits:seastone_block",
    "devilfruits:poneglyph_stone",
    "devilfruits:poneglyph_face",
];

/// Blocks that resist high-power `any_hardness` techniques (`DFTags.RESISTS_TECHNIQUES`).
pub const RESISTS_TECHNIQUES_BLOCKS: [&str; 10] = [
    "minecraft:obsidian",
    "minecraft:crying_obsidian",
    "minecraft:reinforced_deepslate",
    "minecraft:ancient_debris",
    "minecraft:netherite_block",
    "minecraft:Lodestone",
    "minecraft:lodestone",
    "minecraft:anvil",
    "minecraft:iron_bars",
    "minecraft:iron_door",
];

/// Returns true if `block_id` is in `#devilfruits:destruction_immune` or `#devilfruits:poneglyph_unbreakable`.
pub fn is_destruction_immune(block_id: &str) -> bool {
    DESTRUCTION_IMMUNE_BLOCKS.contains(&block_id) || block_id == "devilfruits:seastone_block"
}

/// Returns true if `block_id` is in `#devilfruits:resists_techniques`.
pub fn resists_techniques(block_id: &str) -> bool {
    RESISTS_TECHNIQUES_BLOCKS.contains(&block_id) || is_seastone_block(block_id)
}

/// Pending block restoration entry (`BlockRestoreData.Entry`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRestoreEntry {
    pub pos: [i32; 3],
    pub original_block: String,
    pub expected_current_block: Option<String>,
    pub restore_at_tick: i64,
}

/// Flying debris entity spawned when `TerrainDestruction.tearOut` tears out a block with velocity (`DebrisEntity.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct DebrisEntityState {
    pub id: u64,
    pub block_id: String,
    pub position: DVec3,
    pub velocity: DVec3,
    pub spawned_tick: i64,
    pub max_age_ticks: i32,
}

/// Queued VFX / screen shake / block burst event (`ServerFx.java`).
#[derive(Clone, Debug, PartialEq)]
pub struct QueuedVfxEvent {
    pub fx_id: String,
    pub entity_id: i32,
    pub position: DVec3,
    pub direction: DVec3,
    pub power: f32,
    pub seed: i64,
    pub extra: i32,
    pub radius: f64,
}

/// Bomb explosion profile (`ExplosionProfile.java` & `BombExplosionPayload.java`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExplosionProfile {
    pub center: DVec3,
    pub radius: f32,
    pub damage: f32,
    pub knockback: f32,
    pub breaks_blocks: bool,
    pub causes_fire: bool,
}

/// World FX, terrain destruction, and temporary block decay manager
/// (`TerrainDestruction.java`, `BlockRestoreData.java`, and `ServerFx.java`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldFxManager {
    pub restore_entries: BTreeMap<[i32; 3], BlockRestoreEntry>,
    pub active_debris: Vec<DebrisEntityState>,
    pub vfx_queue: Vec<QueuedVfxEvent>,
    next_debris_id: u64,
}

impl WorldFxManager {
    /// Checks whether block destruction is allowed (`TerrainDestruction.destructionAllowed`).
    pub fn destruction_allowed(cfg: &CoreConfig, rules: DFGameRules) -> bool {
        cfg.block_destruction && (!cfg.respect_mob_griefing || rules.mob_griefing)
    }

    /// Checks whether a block at `pos` with `block_id` and `hardness` can be affected (`TerrainDestruction.canAffect`).
    pub fn can_affect(
        block_id: &str,
        hardness: f32,
        has_block_entity: bool,
        any_hardness: bool,
        cfg: &CoreConfig,
    ) -> bool {
        if block_id == "minecraft:air"
            || has_block_entity
            || is_destruction_immune(block_id)
            || hardness < 0.0
        {
            return false;
        }
        if any_hardness {
            !resists_techniques(block_id)
        } else {
            hardness <= cfg.max_block_hardness
        }
    }

    /// Tears out a single block (`TerrainDestruction.tearOut`), scheduling temporary restoration
    /// and optionally spawning a flying debris entity or block burst VFX.
    #[allow(clippy::too_many_arguments)]
    pub fn tear_out(
        &mut self,
        pos: [i32; 3],
        block_id: &str,
        hardness: f32,
        has_block_entity: bool,
        debris_velocity: Option<DVec3>,
        budget: &mut CastBudget,
        restore_jitter_ticks: i64,
        any_hardness: bool,
        emit_burst: bool,
        now: i64,
        cfg: &CoreConfig,
        rules: DFGameRules,
    ) -> bool {
        if !Self::destruction_allowed(cfg, rules)
            || !Self::can_affect(block_id, hardness, has_block_entity, any_hardness, cfg)
            || !budget.try_consume_block()
        {
            return false;
        }

        if cfg.debris_mode == DebrisMode::Temporary {
            let restore_at = now + cfg.restore_delay_ticks() + restore_jitter_ticks;
            self.schedule_restore(pos, block_id, None, restore_at);
        }

        let center = DVec3::new(pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5);
        if let Some(vel) = debris_velocity {
            if budget.try_consume_debris() {
                self.next_debris_id += 1;
                self.active_debris.push(DebrisEntityState {
                    id: self.next_debris_id,
                    block_id: block_id.to_string(),
                    position: center,
                    velocity: vel,
                    spawned_tick: now,
                    max_age_ticks: 60,
                });
            }
        } else if emit_burst {
            self.emit_block_burst(center, 0.6, 0);
        }

        true
    }

    /// Transforms a block into a temporary element block (e.g. Hie-Hie ice, Magu-Magu magma, Doru-Doru wax)
    /// and schedules restoration after `restore_ticks` (`TerrainDestruction.transform`).
    #[allow(clippy::too_many_arguments)]
    pub fn transform_block(
        &mut self,
        pos: [i32; 3],
        original_block: &str,
        target_block: &str,
        hardness: f32,
        has_block_entity: bool,
        budget: &mut CastBudget,
        restore_ticks: i32,
        permanent_revert: Option<&str>,
        now: i64,
        cfg: &CoreConfig,
    ) -> bool {
        if original_block == target_block
            || original_block == "minecraft:air"
            || has_block_entity
            || is_destruction_immune(original_block)
            || hardness < 0.0
            || (original_block != "minecraft:water"
                && original_block != "minecraft:lava"
                && hardness > cfg.max_block_hardness)
            || !budget.try_consume_block()
        {
            return false;
        }
        let restore_at = now + restore_ticks.max(1) as i64;
        if cfg.debris_mode == DebrisMode::Temporary {
            self.schedule_restore(pos, original_block, Some(target_block), restore_at);
        } else if let Some(perm) = permanent_revert {
            self.schedule_restore(pos, perm, Some(target_block), restore_at);
        }
        true
    }

    /// Carves a spherical crater of radius `radius`, tearing out blocks within `budget`.
    #[allow(clippy::too_many_arguments)]
    pub fn carve_spherical_crater<F>(
        &mut self,
        center: DVec3,
        radius: f64,
        budget: &mut CastBudget,
        any_hardness: bool,
        now: i64,
        cfg: &CoreConfig,
        rules: DFGameRules,
        mut block_query: F,
    ) -> Vec<[i32; 3]>
    where
        F: FnMut([i32; 3]) -> (String, f32, bool),
    {
        let mut broken = Vec::new();
        if !Self::destruction_allowed(cfg, rules) || radius <= 0.0 {
            return broken;
        }
        let r_ceil = radius.ceil() as i32;
        let cx = center.x.floor() as i32;
        let cy = center.y.floor() as i32;
        let cz = center.z.floor() as i32;
        let r2 = radius * radius;

        for dy in -r_ceil..=r_ceil {
            for dx in -r_ceil..=r_ceil {
                for dz in -r_ceil..=r_ceil {
                    if budget.blocks_remaining == 0 {
                        return broken;
                    }
                    let pos = [cx + dx, cy + dy, cz + dz];
                    let bcenter =
                        DVec3::new(pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5);
                    let dist2 = bcenter.distance_squared(center);
                    if dist2 <= r2 {
                        let (block_id, hardness, has_be) = block_query(pos);
                        let outward = (bcenter - center).normalize_or_zero() * 0.45
                            + DVec3::new(0.0, 0.25, 0.0);
                        let jitter = ((dx.abs() + dy.abs() + dz.abs()) % 20) as i64;
                        if self.tear_out(
                            pos,
                            &block_id,
                            hardness,
                            has_be,
                            Some(outward),
                            budget,
                            jitter,
                            any_hardness,
                            true,
                            now,
                            cfg,
                            rules,
                        ) {
                            broken.push(pos);
                        }
                    }
                }
            }
        }
        broken
    }

    pub fn schedule_restore(
        &mut self,
        pos: [i32; 3],
        original_block: &str,
        expected_current: Option<&str>,
        restore_at_tick: i64,
    ) {
        if let Some(prev) = self.restore_entries.get_mut(&pos) {
            prev.restore_at_tick = prev.restore_at_tick.max(restore_at_tick);
            prev.expected_current_block = expected_current.map(str::to_string);
        } else {
            self.restore_entries.insert(
                pos,
                BlockRestoreEntry {
                    pos,
                    original_block: original_block.to_string(),
                    expected_current_block: expected_current.map(str::to_string),
                    restore_at_tick,
                },
            );
        }
    }

    /// Ticks block restoration every 5 ticks (`BlockRestoreData.onLevelTick`) and steps debris physics.
    /// Returns all `(pos, original_block)` entries due for restoration.
    pub fn tick(&mut self, now: i64) -> Vec<BlockRestoreEntry> {
        for debris in &mut self.active_debris {
            debris.position += debris.velocity;
            debris.velocity.y -= 0.04;
            debris.velocity *= 0.98;
        }
        self.active_debris
            .retain(|d| (now - d.spawned_tick) < d.max_age_ticks as i64);

        if (now % 5) != 0 || self.restore_entries.is_empty() {
            return Vec::new();
        }
        let due_positions: Vec<[i32; 3]> = self
            .restore_entries
            .iter()
            .filter_map(|(&pos, e)| (e.restore_at_tick <= now).then_some(pos))
            .collect();
        let mut restored = Vec::with_capacity(due_positions.len());
        for pos in due_positions {
            if let Some(entry) = self.restore_entries.remove(&pos) {
                restored.push(entry);
            }
        }
        restored
    }

    /// Queues a screen-shake VFX event (`ServerFx.shake`).
    pub fn emit_shake(&mut self, pos: DVec3, strength: f32, radius: i32) {
        self.vfx_queue.push(QueuedVfxEvent {
            fx_id: core_fx::SHAKE.into(),
            entity_id: -1,
            position: pos,
            direction: DVec3::ZERO,
            power: strength,
            seed: 0,
            extra: radius,
            radius: radius as f64,
        });
    }

    /// Queues a block-burst VFX event (`ServerFx.blockBurst`).
    pub fn emit_block_burst(&mut self, pos: DVec3, intensity: f32, block_state_id: i32) {
        self.vfx_queue.push(QueuedVfxEvent {
            fx_id: core_fx::BLOCK_BURST.into(),
            entity_id: -1,
            position: pos,
            direction: DVec3::ZERO,
            power: intensity,
            seed: 0,
            extra: block_state_id,
            radius: 64.0,
        });
    }

    /// Queues an arbitrary named VFX event (`ServerFx.send` / `sendNear`).
    #[allow(clippy::too_many_arguments)]
    pub fn emit_fx(
        &mut self,
        fx_id: impl Into<String>,
        entity_id: i32,
        position: DVec3,
        direction: DVec3,
        power: f32,
        seed: i64,
        extra: i32,
        radius: f64,
    ) {
        self.vfx_queue.push(QueuedVfxEvent {
            fx_id: fx_id.into(),
            entity_id,
            position,
            direction,
            power,
            seed,
            extra,
            radius,
        });
    }

    pub fn drain_vfx(&mut self) -> Vec<QueuedVfxEvent> {
        std::mem::take(&mut self.vfx_queue)
    }
}
