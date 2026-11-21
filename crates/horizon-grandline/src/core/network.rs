//! All Grand Line custom network payloads (C2S and S2C, protocol `"13"`) with
//! deterministic binary & JSON serialization.
//!
//! Mirrors `core/network/{DFNetwork,AbilityInputPayload,FormInputPayload,
//! LogiaInputPayload,DashInputPayload,ItemTechniquePayload,AimPayload,
//! JumpInputPayload,HakiInputPayload,FxPayload,AnimationPayload,
//! AbilityTablePayload,ChargePayload,FormStatePayload,LogiaStatePayload,
//! LogiaHitPayload}.java`, `core/gear/GearSwitchPayload.java`,
//! `core/cold/StrugglePayload.java`, `core/combat/RestraintPayload.java`,
//! `core/gacha/CasketRevealPayload.java`, `core/poneglyph/PoneglyphNetwork.java`,
//! and `mobs/pirate/FireGunPayload.java`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::gacha::GachaTier;
use crate::core::itemtech::ItemTechniqueAction;
use crate::core::logia::LogiaState;
use crate::core::rules::Cooldown;

/// Grand Line network protocol version string (`DFNetwork.PROTOCOL = "13"`).
pub const DF_NETWORK_PROTOCOL: &str = "13";

/// Action in `AbilityInputPayload` (`PRESS`, `RELEASE`, `SELECT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilityInputAction {
    Press,
    Release,
    Select,
}

/// Action in `FormInputPayload` (`CYCLE`, `REVERT`, `SET`, `BOOST_START`, `BOOST_STOP`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormInputAction {
    Cycle,
    Revert,
    Set,
    BoostStart,
    BoostStop,
}

/// Action in `LogiaInputPayload` (`ELEMENTAL_ON`, `ELEMENTAL_OFF`, `ELEMENTAL_TOGGLE`, `DASH`, `INTANGIBILITY_TOGGLE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogiaInputAction {
    ElementalOn,
    ElementalOff,
    ElementalToggle,
    Dash,
    IntangibilityToggle,
}

/// Action in `HakiInputPayload` (`TOGGLE`, `SELECT`, `RYUO_START`, `RYUO_RELEASE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HakiInputAction {
    Toggle,
    Select,
    RyuoStart,
    RyuoRelease,
}

/// Action in `JumpInputPayload` (`START`, `RELEASE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JumpInputAction {
    Start,
    Release,
}

/// Action on the Decipher Desk (`PoneglyphNetwork` + `DecipherDeskMenu.clickMenuButton`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecipherDeskAction {
    PickOption(u8),
    UseHint,
    UseReveal,
}

/// Poneglyph summary entry in `PoneglyphSyncPayload`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoneglyphSyncInfo {
    pub id: String,
    pub knowledge: String,
    pub lore_key: String,
    pub carving: i32,
}

/// All Client-to-Server (C2S) payloads registered in `DFNetwork`, `PoneglyphNetwork`, and `Pirates`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum C2SPayload {
    /// `devilfruits:ability_input` — ability cast press, hold release, or slot selection.
    AbilityInput {
        action: AbilityInputAction,
        ability: String,
    },
    /// `devilfruits:form_input` — transformation cycle, revert, explicit set, or flight boost.
    FormInput {
        action: FormInputAction,
        form: String,
    },
    /// `devilfruits:gear_switch` — Gear wheel selection (`gear_1`..`gear_5` + optional sub-gear).
    GearSwitch {
        gear_id: String,
        sub_gear: Option<String>,
    },
    /// `devilfruits:haki_input` — Armament Haki coating toggle, 9-part mask select, or Ryuo charge/release.
    HakiInput {
        action: HakiInputAction,
        mask: u16,
    },
    /// `devilfruits:logia_input` — Logia elemental body toggle, elemental dash, or intangibility reflex toggle.
    LogiaInput {
        action: LogiaInputAction,
    },
    /// `devilfruits:dash_input` — non-Logia form directional dash (`dx`, `dz`).
    DashInput {
        dx: f32,
        dz: f32,
    },
    /// `devilfruits:jump_input` — form charged jump press/release.
    JumpInput {
        action: JumpInputAction,
    },
    /// `devilfruits:aim` — client crosshair world hit point for server `AimHelper`.
    Aim {
        point: [f64; 3],
    },
    /// `devilfruits:item_technique` — weapon combo M1 attack, M2 hold start, or M2 release.
    ItemTechnique {
        action: ItemTechniqueAction,
        predicted_step: i32,
    },
    /// `devilfruits:struggle` — mashing key to break free of Frozen or Restraint holds.
    Struggle,
    /// `devilfruits:poneglyph/rub_held` — heartbeat while holding Paper + Ink Sac on a Poneglyph face.
    PoneglyphRubHeld {
        pos: [i32; 3],
    },
    /// `devilfruits:poneglyph/desk_action` — word pick, Hint (100), or Reveal (101) in Decipher Desk.
    DecipherDesk {
        container_id: i32,
        action: DecipherDeskAction,
    },
    /// `devilfruits:pirate/fire_gun` — firing a loaded Flintlock Pistol or Musket.
    FireGun {
        offhand: bool,
    },
}

/// All Server-to-Client (S2C) payloads registered in `DFNetwork` and `PoneglyphNetwork`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum S2CPayload {
    /// `devilfruits:player_state_sync` — full player fruit, mastery, energy, cooldown, form, gear, and Haki sync.
    PlayerStateSync {
        fruit: Option<FruitId>,
        mastery: u32,
        xp: f32,
        energy: f32,
        max_energy: f32,
        active_form: String,
        selected_ability: Option<String>,
        cooldowns: BTreeMap<String, Cooldown>,
        haki_unlocked: bool,
        haki_level: u32,
        haki_xp: f32,
        haki_spirit: f32,
        haki_active: bool,
        haki_parts: u16,
    },
    /// `devilfruits:fx` — server-triggered particle/sound/screen-shake VFX event (`FxPayload`).
    Fx {
        fx_id: String,
        entity_id: i32,
        pos: [f64; 3],
        dir: [f64; 3],
        power: f32,
        seed: i64,
        extra: i32,
    },
    /// `devilfruits:animation` — first/third-person player animation clip trigger (`AnimationPayload`).
    Animation {
        entity_id: i32,
        clip: String,
        stop: bool,
    },
    /// `devilfruits:ability_table` — synced ability stats & unlock table (`AbilityTablePayload`).
    AbilityTable {
        fruit: FruitId,
        abilities: Vec<String>,
    },
    /// `devilfruits:charge` — charging ability start/stop broadcast (`ChargePayload`).
    Charge {
        entity_id: i32,
        ability_id: String,
        active: bool,
        duration: f32,
        seed: i64,
        released: bool,
    },
    /// `devilfruits:form_state` — active form & synced bitflags broadcast (`FormStatePayload`).
    FormState {
        entity_id: i32,
        fruit: Option<FruitId>,
        form_name: String,
        flags: u32,
    },
    /// `devilfruits:logia_state` — Logia body state, dissolved limbs, and SolidBody plating (`LogiaStatePayload`).
    LogiaState {
        entity_id: i32,
        state: LogiaState,
        dissolved_limbs: u8,
        solid_cracks: u32,
        solid_shattered: u8,
    },
    /// `devilfruits:logia_hit` — carved capsule hole / projectile tunnel (`LogiaHitPayload`).
    LogiaHit {
        entity_id: i32,
        entry_local: [f32; 3],
        exit_local: [f32; 3],
        radius: f32,
        limb_index: u8,
    },
    /// `devilfruits:restraint` — active restraint hold progress or release (`RestraintPayload`).
    Restraint {
        entity_id: i32,
        kind: String,
        ticks_left: i32,
        struggle_progress: f32,
        end_reason: i32,
    },
    /// `devilfruits:casket_reveal` — Devil Fruit Casket gacha roll reveal (`CasketRevealPayload`).
    CasketReveal {
        entity_id: i32,
        tier: GachaTier,
        fruit: FruitId,
        seed: i64,
        reveal_ticks: i32,
        cinematic: bool,
        preview_only: bool,
    },
    /// `devilfruits:poneglyph/rub_state` — rubbing progress bar start/done/cancel (`RubStatePayload`).
    PoneglyphRubState {
        pos: [i32; 3],
        cell: u8,
        carving: i32,
        duration: i32,
        state: u8,
    },
    /// `devilfruits:poneglyph/desk_state` — Decipher Desk puzzle round & literacy state (`DeskStatePayload`).
    PoneglyphDeskState {
        container_id: i32,
        status: i32,
        round: usize,
        rounds: usize,
        clue: String,
        options: Vec<String>,
        mistakes: i32,
        max_mistakes: i32,
        lock_ticks: i32,
        hint: String,
        reveal: i32,
        perks: i32,
        literacy: u32,
        xp: i32,
        xp_from: i32,
        xp_to: i32,
        event: i32,
        event_index: i32,
    },
    /// `devilfruits:poneglyph/sync` — world cipher permutation & registered poneglyph catalog (`PoneglyphSyncPayload`).
    PoneglyphSync {
        worldbound: bool,
        permutation: Vec<u8>,
        poneglyphs: Vec<PoneglyphSyncInfo>,
    },
    /// `devilfruits:poneglyph/knowledge_toast` — recipe knowledge learned toast (`KnowledgeToastPayload`).
    KnowledgeToast {
        knowledge: String,
        name_key: String,
        icon: String,
    },
}

/// Unified network envelope supporting both deterministic length-prefixed binary framing and JSON wire encoding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "direction", content = "payload", rename_all = "snake_case")]
pub enum GrandLinePacket {
    C2S(C2SPayload),
    S2C(S2CPayload),
}

impl GrandLinePacket {
    /// Magic 4-byte frame header (`"GL13"` = Grand Line protocol 13).
    pub const MAGIC: [u8; 4] = *b"GL13";

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Deterministic binary serialization: `b"GL13" || u32_be(len) || utf8_json_bytes`.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let body = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        let len = u32::try_from(body.len()).map_err(|_| "packet payload too large".to_string())?;
        let mut out = Vec::with_capacity(8 + body.len());
        out.extend_from_slice(&Self::MAGIC);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Decodes a packet produced by [`GrandLinePacket::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("truncated packet header".into());
        }
        if bytes[0..4] != Self::MAGIC {
            return Err("invalid Grand Line protocol magic (expected GL13)".into());
        }
        let len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        if bytes.len() != 8 + len {
            return Err(format!(
                "packet length mismatch: header declared {len} bytes, got {}",
                bytes.len() - 8
            ));
        }
        serde_json::from_slice(&bytes[8..]).map_err(|e| e.to_string())
    }
}
