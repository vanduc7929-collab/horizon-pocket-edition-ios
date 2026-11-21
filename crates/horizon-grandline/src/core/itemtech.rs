//! Item technique framework (weapon combos & held techniques without needing a
//! Devil Fruit) plus special Grand Line item behaviors (Seastone Jitte, Pirate
//! Cutlasses, Flintlock Pistol & Musket, Devil Fruit Casket & Gold Key, Haki
//! Scroll, Ancient Script Primer, Viva Card, Log Pose, Eternal Pose, Devil
//! Fruit Encyclopedia, and Bounty Poster).
//!
//! Mirrors `core/itemtech/{ItemTechnique,ItemTechniqueConfig,ItemTechniqueManager,
//! ItemTechniques,WeaponCombo,ItemTechniquePayload}.java`, `core/seastone/JitteTechniques.java`,
//! `core/haki/HakiScrollItem.java`, and `mobs/pirate/item/FlintlockItem.java`.

use std::collections::BTreeMap;

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::rules::PlayerUuid;
use crate::core::seastone::SEASTONE_JITTE_ID;

/// Client-prediction slack window in ticks (`ItemTechniqueManager.SLACK = 5`).
pub const ITEM_TECHNIQUE_SLACK_TICKS: i64 = 5;

/// Canonical Grand Line special item IDs.
pub const CASKET_ITEM_ID: &str = "devilfruits:casket";
pub const GOLD_KEY_ITEM_ID: &str = "devilfruits:gold_key";
pub const HAKI_SCROLL_ITEM_ID: &str = "devilfruits:haki_scroll";
pub const ANCIENT_PRIMER_ITEM_ID: &str = "devilfruits:ancient_script_primer";
pub const CUTLASS_ITEM_ID: &str = "devilfruits:pirate/cutlass";
pub const CAPTAIN_CUTLASS_ITEM_ID: &str = "devilfruits:pirate/captain_cutlass";
pub const FLINTLOCK_PISTOL_ITEM_ID: &str = "devilfruits:pirate/flintlock_pistol";
pub const MUSKET_ITEM_ID: &str = "devilfruits:pirate/musket";
pub const MUSKET_BALL_ITEM_ID: &str = "devilfruits:pirate/musket_ball";
pub const VIVA_CARD_ITEM_ID: &str = "devilfruits:viva_card";
pub const LOG_POSE_ITEM_ID: &str = "devilfruits:log_pose";
pub const ETERNAL_POSE_ITEM_ID: &str = "devilfruits:eternal_pose";
pub const ENCYCLOPEDIA_ITEM_ID: &str = "devilfruits:devil_fruit_encyclopedia";
pub const BOUNTY_POSTER_ITEM_ID: &str = "devilfruits:bounty_poster";

/// Technique activation kind (`ItemTechnique.Kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemTechniqueKind {
    Combo,
    Held,
}

/// Input action sent in `ItemTechniquePayload` (`0 = ATTACK`, `1 = USE`, `2 = RELEASE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemTechniqueAction {
    Attack = 0,
    Use = 1,
    Release = 2,
}

impl ItemTechniqueAction {
    pub const fn from_wire(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Attack),
            1 => Some(Self::Use),
            2 => Some(Self::Release),
            _ => None,
        }
    }
}

/// Static declaration of an item technique (`ItemTechnique.java` + `WeaponCombo.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemTechniqueDef {
    pub id: String,
    pub item_id: String,
    pub kind: ItemTechniqueKind,
    pub enabled: bool,
    pub cooldown_ticks: i32,
    pub window_ticks: i32,
    pub max_hold_ticks: i32,
    pub clip_prefix: String,
    /// Per-step hit tick offsets (`WeaponCombo.hits`).
    pub step_hits: Vec<Vec<i32>>,
    /// Per-step busy duration in ticks (`WeaponCombo.busy`).
    pub step_busy: Vec<i32>,
    /// Extra tunable parameters (`ItemTechnique.extras`).
    pub extras: BTreeMap<String, f64>,
}

impl ItemTechniqueDef {
    pub fn steps(&self) -> usize {
        self.step_hits.len().max(1)
    }

    pub fn hit_ticks(&self, step: usize) -> &[i32] {
        self.step_hits
            .get(step.min(self.step_hits.len().saturating_sub(1)))
            .map(Vec::as_slice)
            .unwrap_or(&[0])
    }

    pub fn busy_ticks(&self, step: usize) -> i32 {
        self.step_busy
            .get(step.min(self.step_busy.len().saturating_sub(1)))
            .copied()
            .unwrap_or(10)
    }

    pub fn clip(&self, step: usize) -> Option<String> {
        if self.clip_prefix.is_empty() {
            None
        } else {
            Some(format!("devilfruits:{}{}", self.clip_prefix, step + 1))
        }
    }

    /// Built-in Seastone Jitte 4-hit combo (`JitteTechniques.COMBO`).
    pub fn seastone_jitte_combo() -> Self {
        let mut extras = BTreeMap::new();
        extras.insert("jabShare".into(), 0.72);
        extras.insert("backhandShare".into(), 0.72);
        extras.insert("thrustShare".into(), 0.80);
        extras.insert("smashShare".into(), 1.05);
        extras.insert("reach".into(), 3.3);
        extras.insert("arcDegrees".into(), 70.0);
        extras.insert("sealTicks".into(), 40.0);
        extras.insert("knockdownTicks".into(), 20.0);
        Self {
            id: "devilfruits:seastone/jitte_combo".into(),
            item_id: SEASTONE_JITTE_ID.into(),
            kind: ItemTechniqueKind::Combo,
            enabled: true,
            cooldown_ticks: 10,
            window_ticks: 10,
            max_hold_ticks: 0,
            clip_prefix: "items/jitte_m1_".into(),
            step_hits: vec![vec![4], vec![4], vec![5], vec![7]],
            step_busy: vec![7, 7, 8, 15],
            extras,
        }
    }

    /// Built-in Seastone Jitte Pin held technique (`JitteTechniques.PIN`).
    pub fn seastone_jitte_pin() -> Self {
        let mut extras = BTreeMap::new();
        extras.insert("reach".into(), 3.0);
        extras.insert("leash".into(), 3.5);
        extras.insert("seconds".into(), 4.0);
        extras.insert("damagePerSecond".into(), 1.0);
        extras.insert("durabilityPerSecond".into(), 2.0);
        extras.insert("maxStruggleCut".into(), 0.5);
        extras.insert("strugglePresses".into(), 16.0);
        Self {
            id: "devilfruits:seastone/jitte_pin".into(),
            item_id: SEASTONE_JITTE_ID.into(),
            kind: ItemTechniqueKind::Held,
            enabled: true,
            cooldown_ticks: 160,
            window_ticks: 10,
            max_hold_ticks: 82,
            clip_prefix: "items/jitte_pin_".into(),
            step_hits: Vec::new(),
            step_busy: Vec::new(),
            extras,
        }
    }

    /// Built-in Pirate Cutlass 3-step slash combo.
    pub fn pirate_cutlass_combo(item_id: &str) -> Self {
        let mut extras = BTreeMap::new();
        extras.insert("slash1Share".into(), 0.85);
        extras.insert("slash2Share".into(), 0.95);
        extras.insert("finisherShare".into(), 1.25);
        extras.insert("reach".into(), 3.4);
        extras.insert("arcDegrees".into(), 80.0);
        Self {
            id: format!("{item_id}_combo"),
            item_id: item_id.to_string(),
            kind: ItemTechniqueKind::Combo,
            enabled: true,
            cooldown_ticks: 12,
            window_ticks: 10,
            max_hold_ticks: 0,
            clip_prefix: "items/cutlass_m1_".into(),
            step_hits: vec![vec![4], vec![4], vec![6]],
            step_busy: vec![7, 7, 14],
            extras,
        }
    }

    /// Returns all canonical built-in item techniques.
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::seastone_jitte_combo(),
            Self::seastone_jitte_pin(),
            Self::pirate_cutlass_combo(CUTLASS_ITEM_ID),
            Self::pirate_cutlass_combo(CAPTAIN_CUTLASS_ITEM_ID),
        ]
    }
}

/// Active weapon combo state for a player (`ItemTechniqueManager.Combo`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveItemCombo {
    pub technique_id: String,
    pub item_id: String,
    pub step: usize,
    pub last_press_tick: i64,
    pub busy_until_tick: i64,
    pub pending_hits: Vec<i32>,
    pub next_hit_index: usize,
}

/// Active held item technique state (`ItemTechniqueManager.Held`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveItemHold {
    pub technique_id: String,
    pub item_id: String,
    pub started_tick: i64,
    pub max_ticks: i32,
}

/// Events emitted by `ItemTechniqueRuntime`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemTechniqueEvent {
    ComboStepStarted {
        technique_id: String,
        step: usize,
        clip: Option<String>,
    },
    ComboStrike {
        technique_id: String,
        step: usize,
        hit_index: usize,
    },
    HoldStarted {
        technique_id: String,
    },
    HoldTick {
        technique_id: String,
        elapsed_ticks: i32,
    },
    HoldEnded {
        technique_id: String,
        elapsed_ticks: i32,
        interrupted: bool,
    },
}

/// Per-player item technique state machine (`com.devilfruits.core.itemtech.ItemTechniqueManager`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemTechniqueRuntime {
    pub combo: Option<ActiveItemCombo>,
    pub held: Option<ActiveItemHold>,
    pub cooldowns: BTreeMap<String, i64>,
}

impl ItemTechniqueRuntime {
    pub fn is_on_cooldown(&self, technique_id: &str, now: i64) -> bool {
        self.cooldowns
            .get(technique_id)
            .is_some_and(|&until| until > now)
    }

    pub fn cooldown_remaining(&self, technique_id: &str, now: i64) -> i32 {
        self.cooldowns
            .get(technique_id)
            .map(|&until| (until - now).max(0) as i32)
            .unwrap_or(0)
    }

    /// Handles an attack combo input (`ItemTechniqueManager.attack`).
    pub fn attack(
        &mut self,
        def: &ItemTechniqueDef,
        predicted_step: Option<usize>,
        free_to_act: bool,
        now: i64,
    ) -> (Option<usize>, Vec<ItemTechniqueEvent>) {
        let mut events = Vec::new();
        if !def.enabled || def.kind != ItemTechniqueKind::Combo || !free_to_act {
            return (None, events);
        }
        let slack = if predicted_step.is_some() {
            ITEM_TECHNIQUE_SLACK_TICKS
        } else {
            0
        };

        let mut next_step = 0usize;
        if let Some(c) = &self.combo {
            if c.technique_id == def.id {
                if now < c.busy_until_tick - slack {
                    return (None, events);
                }
                let continues = c.step + 1 < def.steps()
                    && now <= c.busy_until_tick + def.window_ticks as i64 + slack;
                if continues && predicted_step.is_none_or(|p| p == c.step + 1) {
                    next_step = c.step + 1;
                } else if predicted_step.is_some_and(|p| p > 0) {
                    return (None, events);
                } else {
                    if c.step + 1 >= def.steps() && def.cooldown_ticks > 0 {
                        self.cooldowns
                            .insert(def.id.clone(), now + def.cooldown_ticks as i64);
                    }
                    self.combo = None;
                }
            }
        }

        if next_step == 0 && self.is_on_cooldown(&def.id, now + slack) {
            return (None, events);
        }

        let hits = def.hit_ticks(next_step).to_vec();
        let busy = def.busy_ticks(next_step);
        let mut active = ActiveItemCombo {
            technique_id: def.id.clone(),
            item_id: def.item_id.clone(),
            step: next_step,
            last_press_tick: now,
            busy_until_tick: now + busy as i64,
            pending_hits: hits,
            next_hit_index: 0,
        };

        events.push(ItemTechniqueEvent::ComboStepStarted {
            technique_id: def.id.clone(),
            step: next_step,
            clip: def.clip(next_step),
        });

        while active.next_hit_index < active.pending_hits.len()
            && active.last_press_tick + active.pending_hits[active.next_hit_index] as i64 <= now
        {
            let hit_idx = active.next_hit_index;
            active.next_hit_index += 1;
            events.push(ItemTechniqueEvent::ComboStrike {
                technique_id: def.id.clone(),
                step: next_step,
                hit_index: hit_idx,
            });
        }

        self.combo = Some(active);
        (Some(next_step), events)
    }

    /// Starts a held item technique (`ItemTechniqueManager.use`).
    pub fn begin_hold(
        &mut self,
        def: &ItemTechniqueDef,
        free_to_act: bool,
        now: i64,
    ) -> (bool, Vec<ItemTechniqueEvent>) {
        let mut events = Vec::new();
        if !def.enabled
            || def.kind != ItemTechniqueKind::Held
            || self.held.is_some()
            || !free_to_act
            || self.is_on_cooldown(&def.id, now)
        {
            return (false, events);
        }
        self.held = Some(ActiveItemHold {
            technique_id: def.id.clone(),
            item_id: def.item_id.clone(),
            started_tick: now,
            max_ticks: def.max_hold_ticks.max(1),
        });
        events.push(ItemTechniqueEvent::HoldStarted {
            technique_id: def.id.clone(),
        });
        (true, events)
    }

    /// Ends a held item technique (`ItemTechniqueManager.release` / `interrupt`).
    pub fn end_hold(
        &mut self,
        cooldown_ticks: i32,
        interrupted: bool,
        now: i64,
    ) -> Option<ItemTechniqueEvent> {
        let h = self.held.take()?;
        let elapsed = (now - h.started_tick).max(0) as i32;
        if cooldown_ticks > 0 {
            self.cooldowns
                .insert(h.technique_id.clone(), now + cooldown_ticks as i64);
        }
        Some(ItemTechniqueEvent::HoldEnded {
            technique_id: h.technique_id,
            elapsed_ticks: elapsed,
            interrupted,
        })
    }

    /// Interrupts any active combo or held technique (`ItemTechniqueManager.interrupt`).
    pub fn interrupt(&mut self, cooldown_ticks: i32, now: i64) -> Vec<ItemTechniqueEvent> {
        let mut events = Vec::new();
        if let Some(ev) = self.end_hold(cooldown_ticks, true, now) {
            events.push(ev);
        }
        if let Some(c) = self.combo.take() {
            if cooldown_ticks > 0 {
                self.cooldowns
                    .insert(c.technique_id, now + cooldown_ticks as i64);
            }
        }
        events
    }

    /// Ticks pending combo strikes, combo window expiration, and held technique duration (`ItemTechniqueManager.tickPlayer`).
    pub fn tick(
        &mut self,
        held_item_id: Option<&str>,
        free_to_act: bool,
        def_lookup: impl Fn(&str) -> Option<ItemTechniqueDef>,
        now: i64,
    ) -> Vec<ItemTechniqueEvent> {
        let mut events = Vec::new();

        if let Some(mut c) = self.combo.take() {
            let def = def_lookup(&c.technique_id);
            let same_item = held_item_id.is_some_and(|id| crate::core::fruit::df_id(id) == c.item_id);
            while c.next_hit_index < c.pending_hits.len()
                && now >= c.last_press_tick + c.pending_hits[c.next_hit_index] as i64
            {
                let hit_idx = c.next_hit_index;
                c.next_hit_index += 1;
                if same_item && free_to_act {
                    events.push(ItemTechniqueEvent::ComboStrike {
                        technique_id: c.technique_id.clone(),
                        step: c.step,
                        hit_index: hit_idx,
                    });
                }
            }

            let all_hits_done = c.next_hit_index >= c.pending_hits.len();
            let steps = def.as_ref().map(|d| d.steps()).unwrap_or(1);
            let window = def.as_ref().map(|d| d.window_ticks).unwrap_or(10) as i64;
            let cd = def.as_ref().map(|d| d.cooldown_ticks).unwrap_or(10);

            let expired = all_hits_done
                && if c.step + 1 >= steps {
                    now >= c.busy_until_tick
                } else {
                    now > c.busy_until_tick + window + ITEM_TECHNIQUE_SLACK_TICKS
                };

            if expired {
                if c.step + 1 >= steps && cd > 0 {
                    self.cooldowns
                        .insert(c.technique_id.clone(), now + cd as i64);
                }
            } else {
                self.combo = Some(c);
            }
        }

        if let Some(h) = self.held.clone() {
            let def = def_lookup(&h.technique_id);
            let cd = def.as_ref().map(|d| d.cooldown_ticks).unwrap_or(160);
            let same_item = held_item_id.is_some_and(|id| crate::core::fruit::df_id(id) == h.item_id);
            let elapsed = (now - h.started_tick).max(0) as i32;
            if !same_item || !free_to_act {
                if let Some(ev) = self.end_hold(cd, true, now) {
                    events.push(ev);
                }
            } else if elapsed >= h.max_ticks {
                if let Some(ev) = self.end_hold(cd, false, now) {
                    events.push(ev);
                }
            } else {
                events.push(ItemTechniqueEvent::HoldTick {
                    technique_id: h.technique_id,
                    elapsed_ticks: elapsed,
                });
            }
        }

        events
    }
}

// ---------------------------------------------------------------------------
// Special Grand Line item behaviors (Viva Card, Log Pose, Eternal Pose,
// Devil Fruit Encyclopedia, Bounty Poster, Flintlock / Musket)
// ---------------------------------------------------------------------------

/// Viva Card (Vivre Card) bound to a target entity's life force and position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VivaCardData {
    pub owner: PlayerUuid,
    pub owner_position: DVec3,
    pub owner_health_fraction: f32,
}

impl VivaCardData {
    /// Direction unit vector pointing on the horizontal plane from `holder_pos` toward `owner_position`.
    pub fn direction_from(&self, holder_pos: DVec3) -> DVec3 {
        let delta = DVec3::new(
            self.owner_position.x - holder_pos.x,
            0.0,
            self.owner_position.z - holder_pos.z,
        );
        delta.normalize_or_zero()
    }

    /// Smolder/burn fraction (`0.0` = healthy, `1.0` = dead/consumed) as owner's health drops below 50%.
    pub fn burn_fraction(&self) -> f32 {
        if self.owner_health_fraction >= 0.5 {
            0.0
        } else {
            (1.0 - self.owner_health_fraction * 2.0).clamp(0.0, 1.0)
        }
    }
}

/// Grand Line magnetic compass (`LogPose` adapts to new islands over time; `EternalPose` is permanently locked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompassPoseData {
    pub target_name: String,
    pub target_pos: [i32; 3],
    pub is_eternal: bool,
    pub lock_progress_ticks: i32,
    pub required_lock_ticks: i32,
}

impl CompassPoseData {
    pub fn eternal(target_name: impl Into<String>, target_pos: [i32; 3]) -> Self {
        Self {
            target_name: target_name.into(),
            target_pos,
            is_eternal: true,
            lock_progress_ticks: 1200,
            required_lock_ticks: 1200,
        }
    }

    pub fn log_pose(target_name: impl Into<String>, target_pos: [i32; 3], lock_ticks: i32) -> Self {
        Self {
            target_name: target_name.into(),
            target_pos,
            is_eternal: false,
            lock_progress_ticks: 0,
            required_lock_ticks: lock_ticks.max(1),
        }
    }

    pub fn is_locked(&self) -> bool {
        self.is_eternal || self.lock_progress_ticks >= self.required_lock_ticks
    }

    pub fn tick_adaptation(&mut self) -> bool {
        if !self.is_eternal && self.lock_progress_ticks < self.required_lock_ticks {
            self.lock_progress_ticks += 1;
        }
        self.is_locked()
    }

    /// Horizontal needle angle in radians from `pos` to `target_pos`.
    pub fn needle_angle_rad(&self, pos: DVec3) -> f64 {
        let dx = (self.target_pos[0] as f64 + 0.5) - pos.x;
        let dz = (self.target_pos[2] as f64 + 0.5) - pos.z;
        dz.atan2(dx)
    }
}

/// Devil Fruit Encyclopedia tracking discovered & identified fruits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevilFruitEncyclopedia {
    pub discovered: Vec<FruitId>,
}

impl DevilFruitEncyclopedia {
    pub fn record(&mut self, fruit: FruitId) -> bool {
        if self.discovered.contains(&fruit) {
            false
        } else {
            self.discovered.push(fruit);
            true
        }
    }

    pub fn knows(&self, fruit: FruitId) -> bool {
        self.discovered.contains(&fruit)
    }

    pub fn completion_fraction(&self) -> f32 {
        self.discovered.len() as f32 / FruitId::ALL.len() as f32
    }
}

/// Bounty Poster item data (`DEAD OR ALIVE`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BountyPosterData {
    pub target_id: PlayerUuid,
    pub target_name: String,
    pub berry_amount: u64,
    pub only_alive: bool,
}

impl BountyPosterData {
    pub fn new(target_id: PlayerUuid, target_name: impl Into<String>, berry_amount: u64) -> Self {
        Self {
            target_id,
            target_name: target_name.into(),
            berry_amount,
            only_alive: false,
        }
    }
}
