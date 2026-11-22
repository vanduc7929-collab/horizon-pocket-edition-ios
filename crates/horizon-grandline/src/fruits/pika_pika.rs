//! Pika Pika no Mi (Light Logia) — Grand Line 0.4.0 1:1 port.
//!
//! Mirrors `fruits/pika_pika/{PikaPika,PikaAbilities,PikaEvents,PikaFx,PikaLight}.java`
//! and `fruits/pika_pika/ability/{FingerBeamAbility,FootBeamAbility,AmaterasuAbility,
//! AmaNoMurakumoAbility,MurakumoSword,MurakumoSlashAbility,MurakumoParryAbility,
//! LightFlightAbility,LightFlight,LightClonesAbility,YasakaniAbility,LightBlast,LightRay}.java`.

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastBudget, CastResult, CastType,
};
use crate::core::combat::{CombatTargeting, GrandLineDamageType};
use crate::core::form::{FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::logia::{LogiaState, limbs};
use crate::core::rules::PlayerFruitData;
use crate::fruits::{AbilityExecutionResult, AbilityHitEffect, DevilFruitModule, FruitContext};

// ---------------------------------------------------------------------------
// Canonical Pika-Pika constants (`PikaPika.java`, `PikaLight.java`, abilities)
// ---------------------------------------------------------------------------

/// `PikaLight.java` brightness thresholds for Logia environment regeneration.
pub const PIKA_BRIGHT_BLOCK_LIGHT: i32 = 13;
pub const PIKA_DARK_LIGHT: i32 = 3;

/// `FingerBeamAbility.java` (`finger_beam` / `laser` / `light_beam`) constants.
pub const FINGER_BEAM_ENERGY: f32 = 4.0;
pub const FINGER_BEAM_COOLDOWN_TICKS: i32 = 6;
pub const FINGER_BEAM_UNLOCK_MASTERY: u32 = 1;
pub const FINGER_BEAM_DAMAGE: f32 = 3.0;
pub const FINGER_BEAM_RANGE: f32 = 96.0;
pub const FINGER_BEAM_KNOCKBACK: f32 = 0.25;
pub const FINGER_BEAM_FIRE_TICK: i32 = 15;
pub const FINGER_BEAM_REPEAT_FIRE_TICK: i32 = 2;
pub const FINGER_BEAM_HOLD_TICKS: i32 = 9;
pub const FINGER_BEAM_MAX_TARGETS: usize = 2;
pub const FINGER_BEAM_BOLT_RADIUS: f64 = 0.25;

/// `FootBeamAbility.java` (`foot_beam` / `light_kick`) constants.
pub const FOOT_BEAM_ENERGY: f32 = 28.0;
pub const FOOT_BEAM_COOLDOWN_TICKS: i32 = 200;
pub const FOOT_BEAM_UNLOCK_MASTERY: u32 = 2;
pub const FOOT_BEAM_DAMAGE: f32 = 16.0;
pub const FOOT_BEAM_RANGE: f32 = 64.0;
pub const FOOT_BEAM_KNOCKBACK: f32 = 1.6;
pub const FOOT_BEAM_FIRE_TICK: i32 = 14;
pub const FOOT_BEAM_HOLD_TICKS: i32 = 10;
pub const FOOT_BEAM_RADIUS: f64 = 1.2;
pub const FOOT_BEAM_BURST_RADIUS: f64 = 3.5;
pub const FOOT_BEAM_BURST_DAMAGE: f32 = 8.0;

/// `AmaterasuAbility.java` constants.
pub const AMATERASU_ENERGY: f32 = 22.0;
pub const AMATERASU_COOLDOWN_TICKS: i32 = 360;
pub const AMATERASU_UNLOCK_MASTERY: u32 = 3;
pub const AMATERASU_DAMAGE: f32 = 2.0;
pub const AMATERASU_FIRE_TICK: i32 = 12;
pub const AMATERASU_FLASH_RADIUS: f64 = 14.0;
pub const AMATERASU_BLIND_TICKS: i32 = 80; // 4.0s * 20
pub const AMATERASU_SLOW_TICKS: i32 = 40; // 2.0s * 20

/// `AmaNoMurakumoAbility.java` constants.
pub const AMA_NO_MURAKUMO_ENERGY: f32 = 10.0;
pub const AMA_NO_MURAKUMO_DRAIN_PER_SECOND: f32 = 1.5;
pub const AMA_NO_MURAKUMO_COOLDOWN_TICKS: i32 = 40;
pub const AMA_NO_MURAKUMO_UNLOCK_MASTERY: u32 = 4;
pub const AMA_NO_MURAKUMO_SUMMON_TICKS: i32 = 22;

/// `MurakumoSlashAbility.java` constants.
pub const MURAKUMO_SLASH_STRIKES: [i32; 5] = [5, 5, 6, 5, 7];
pub const MURAKUMO_SLASH_POWER: [f32; 5] = [1.0, 1.0, 1.1, 1.15, 1.4];
pub const MURAKUMO_SLASH_ENERGY: f32 = 1.0;
pub const MURAKUMO_SLASH_FINISHER_ENERGY: f32 = 6.0;
pub const MURAKUMO_SLASH_COOLDOWN_TICKS: i32 = 16;
pub const MURAKUMO_SLASH_DAMAGE: f32 = 9.0;
pub const MURAKUMO_SLASH_RANGE: f32 = 3.8;
pub const MURAKUMO_SLASH_KNOCKBACK: f32 = 0.45;
pub const MURAKUMO_SLASH_ARC_DEGREES: f64 = 110.0;
pub const MURAKUMO_SLASH_CRESCENT_RANGE: f64 = 24.0;
pub const MURAKUMO_SLASH_CRESCENT_DAMAGE: f32 = 12.0;
pub const MURAKUMO_SLASH_CRESCENT_SPAN: f64 = 3.2;
pub const MURAKUMO_SLASH_CRESCENT_BLOCKS: usize = 40;

/// `MurakumoParryAbility.java` constants.
pub const MURAKUMO_PARRY_ENERGY: f32 = 3.0;
pub const MURAKUMO_PARRY_COOLDOWN_TICKS: i32 = 20;
pub const MURAKUMO_PARRY_KNOCKBACK: f32 = 0.7;
pub const MURAKUMO_PARRY_WINDOW_TICKS: i32 = 8;

/// `LightFlightAbility.java` & `LightFlight.java` (`light_flight` / `yata_no_kagami`) constants.
pub const LIGHT_FLIGHT_ENERGY: f32 = 12.0;
pub const LIGHT_FLIGHT_COOLDOWN_TICKS: i32 = 60;
pub const LIGHT_FLIGHT_UNLOCK_MASTERY: u32 = 5;
pub const LIGHT_FLIGHT_SPEED: f64 = 2.0;
pub const LIGHT_FLIGHT_BOOST_SPEED: f64 = 4.0;
pub const LIGHT_FLIGHT_HARD_CAP: f64 = 9.5;
pub const LIGHT_FLIGHT_ACCEL_TICKS: i32 = 12;
pub const LIGHT_FLIGHT_DRAIN_PER_SECOND: f32 = 5.0;
pub const LIGHT_FLIGHT_MAX_SECONDS: f64 = 40.0;
pub const LIGHT_FLIGHT_TURN_ENERGY: f32 = 3.0;
pub const LIGHT_FLIGHT_TURN_GAP_TICKS: i32 = 5;
pub const LIGHT_FLIGHT_KICK_DAMAGE: f32 = 6.0;
pub const LIGHT_FLIGHT_KICK_PER_SPEED: f32 = 1.5;

/// `LightClonesAbility.java` constants.
pub const LIGHT_CLONES_ENERGY: f32 = 45.0;
pub const LIGHT_CLONES_COOLDOWN_TICKS: i32 = 900;
pub const LIGHT_CLONES_UNLOCK_MASTERY: u32 = 7;
pub const LIGHT_CLONES_BODIES: usize = 7;
pub const LIGHT_CLONES_HP_FRACTION: f32 = 0.25;
pub const LIGHT_CLONES_LIFE_TICKS: i32 = 400; // 20.0s * 20
pub const LIGHT_CLONES_DAMAGE_FRACTION: f32 = 0.5;
pub const LIGHT_CLONES_RING_RADIUS: f64 = 5.0;

/// `YasakaniAbility.java` (`yasakani` / `yasakani_no_magatama`) constants.
pub const YASAKANI_ENERGY: f32 = 90.0;
pub const YASAKANI_COOLDOWN_TICKS: i32 = 2400;
pub const YASAKANI_UNLOCK_MASTERY: u32 = 9;
pub const YASAKANI_DAMAGE: f32 = 12.0;
pub const YASAKANI_RANGE: f32 = 80.0;
pub const YASAKANI_KNOCKBACK: f32 = 0.9;
pub const YASAKANI_JUMP_TICKS: i32 = 12;
pub const YASAKANI_STAR_TICKS: i32 = 12;
pub const YASAKANI_ATTACK_TICKS: i32 = 80;
pub const YASAKANI_BEAMS_PER_SECOND: f64 = 14.0;
pub const YASAKANI_SPREAD_RADIUS: f64 = 3.0;
pub const YASAKANI_AIM_TURN_RATE_DEG: f32 = 4.0;
pub const YASAKANI_BLAST_RADIUS: f64 = 2.5;
pub const YASAKANI_MAX_BLOCKS: usize = 600;
pub const YASAKANI_JUMP_SPEED: f64 = 1.25;

/// Canonical Pika-Pika VFX resource IDs (`PikaFx.java`).
pub mod pika_fx {
    pub const AWAKEN: &str = "devilfruits:pika_pika/awaken";
    pub const LEVEL_UP: &str = "devilfruits:pika_pika/level_up";
    pub const REINCARNATE: &str = "devilfruits:pika_pika/reincarnate";
    pub const FINGER: &str = "devilfruits:pika_pika/finger_beam";
    pub const FINGER_AIM: &str = "devilfruits:pika_pika/finger_beam_aim";
    pub const FOOT_AIM: &str = "devilfruits:pika_pika/foot_beam_aim";
    pub const FOOT: &str = "devilfruits:pika_pika/foot_beam";
    pub const AMATERASU: &str = "devilfruits:pika_pika/amaterasu";
    pub const AMATERASU_BLIND: &str = "devilfruits:pika_pika/amaterasu_blind";
    pub const SWORD_SUMMON: &str = "devilfruits:pika_pika/murakumo_summon";
    pub const SWORD_SWING: &str = "devilfruits:pika_pika/murakumo_swing";
    pub const SWORD_CRESCENT: &str = "devilfruits:pika_pika/murakumo_crescent";
    pub const SWORD_CRESCENT_END: &str = "devilfruits:pika_pika/murakumo_crescent_end";
    pub const SWORD_PARRY: &str = "devilfruits:pika_pika/murakumo_parry";
    pub const SWORD_DISMISS: &str = "devilfruits:pika_pika/murakumo_dismiss";
    pub const FLIGHT_START: &str = "devilfruits:pika_pika/flight_start";
    pub const MIRROR: &str = "devilfruits:pika_pika/flight_mirror";
    pub const FLIGHT_KICK: &str = "devilfruits:pika_pika/flight_kick";
    pub const FLIGHT_END: &str = "devilfruits:pika_pika/flight_end";
    pub const CLONE_BEAM: &str = "devilfruits:pika_pika/clone_beam";
    pub const CLONE_SPLIT: &str = "devilfruits:pika_pika/clone_split";
    pub const CLONE_BURST: &str = "devilfruits:pika_pika/clone_burst";
    pub const ULT_JUMP: &str = "devilfruits:pika_pika/yasakani_jump";
    pub const ULT_STAR: &str = "devilfruits:pika_pika/yasakani_star";
    pub const ULT_BOLT: &str = "devilfruits:pika_pika/yasakani_bolt";
    pub const ULT_END: &str = "devilfruits:pika_pika/yasakani_end";
}

/// Canonical Pika-Pika animation clip IDs.
pub mod pika_anim {
    pub const FINGER_SHOT: &str = "devilfruits:pika_pika/finger_shot";
    pub const FINGER_SHOT_REPEAT: &str = "devilfruits:pika_pika/finger_shot_repeat";
    pub const FOOT_BEAM: &str = "devilfruits:pika_pika/foot_beam";
    pub const AMATERASU: &str = "devilfruits:pika_pika/amaterasu";
    pub const SWORD_SUMMON: &str = "devilfruits:pika_pika/sword_summon";
    pub const SWORD_PARRY: &str = "devilfruits:pika_pika/sword_parry";
    pub const CLONES_CAST: &str = "devilfruits:pika_pika/clones_cast";
    pub const ULT_JUMP: &str = "devilfruits:pika_pika/ult_jump";
    pub const ULT_HOLD: &str = "devilfruits:pika_pika/ult_hold";
    pub const ULT_END: &str = "devilfruits:pika_pika/ult_end";
}

/// Pika Pika no Mi Devil Fruit module (`PikaPika.java`).
#[derive(Clone, Copy, Debug, Default)]
pub struct PikaPikaModule;

impl PikaPikaModule {
    pub const fn new() -> Self {
        Self
    }

    /// Exact 32-bit packed yaw/pitch direction encoding (`PikaFx.packDir`).
    pub fn pack_dir(d: DVec3) -> i32 {
        let yaw = d.z.atan2(d.x);
        let pitch = d.y.clamp(-1.0, 1.0).asin();
        let y = (((yaw / (std::f64::consts::PI * 2.0) + 0.5) * 65535.0).round() as i32) & 0xFFFF;
        let p = (((pitch / std::f64::consts::PI + 0.5) * 65535.0).round() as i32) & 0xFFFF;
        (y << 16) | p
    }

    /// Exact 32-bit packed yaw/pitch direction decoding (`PikaFx.unpackDir`).
    pub fn unpack_dir(packed: i32) -> DVec3 {
        let yaw = (((packed as u32 >> 16) & 0xFFFF) as f64 / 65535.0 - 0.5)
            * 2.0
            * std::f64::consts::PI;
        let pitch = ((packed & 0xFFFF) as f64 / 65535.0 - 0.5) * std::f64::consts::PI;
        let c = pitch.cos();
        DVec3::new(yaw.cos() * c, pitch.sin(), yaw.sin() * c)
    }

    /// Exact mirror reflection vector (`LightFlight.reflect(d, n)`): `d - 2 * dot(d, n) * n`.
    pub fn mirror_reflect(d: DVec3, n: DVec3) -> DVec3 {
        d - n * (2.0 * d.dot(n))
    }

    /// Exact mirror normal between incoming and outgoing beam directions (`LightFlight.mirrorNormal`).
    pub fn mirror_normal(in_dir: DVec3, out_dir: DVec3) -> DVec3 {
        let n = out_dir - in_dir;
        if n.length_squared() < 1e-9 {
            -in_dir
        } else {
            n.normalize()
        }
    }

    /// Computes a multi-bounce Yata no Kagami mirror reflection raycast path.
    pub fn trace_mirror_path(origin: DVec3, initial_dir: DVec3, normals: &[DVec3], segment_len: f64) -> Vec<DVec3> {
        let mut points = Vec::with_capacity(normals.len() + 2);
        let mut cur_pos = origin;
        let mut cur_dir = initial_dir.normalize_or_zero();
        points.push(cur_pos);
        for &n in normals {
            cur_pos += cur_dir * segment_len;
            points.push(cur_pos);
            cur_dir = Self::mirror_reflect(cur_dir, n.normalize_or_zero()).normalize_or_zero();
        }
        cur_pos += cur_dir * segment_len;
        points.push(cur_pos);
        points
    }

    /// Normalizes ability alias names so both `FruitMeta` names (`"laser"`, `"yata_no_kagami"`, `"yasakani"`)
    /// and Java / prompt names (`"finger_beam"`, `"light_beam"`, `"foot_beam"`, `"light_kick"`,
    /// `"light_flight"`, `"yasakani_no_magatama"`) resolve identically.
    fn canonical_ability_name(name: &str) -> &str {
        let stripped = name
            .split(':')
            .next_back()
            .unwrap_or(name)
            .split('/')
            .next_back()
            .unwrap_or(name);
        match stripped {
            "finger_beam" | "light_beam" => "laser",
            "foot_beam" => "light_kick",
            "light_flight" => "yata_no_kagami",
            "yasakani_no_magatama" => "yasakani",
            other => other,
        }
    }
}

impl DevilFruitModule for PikaPikaModule {
    fn fruit_id(&self) -> FruitId {
        FruitId::PikaPika
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let allowed = vec!["base".to_string()];
        vec![
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "laser"),
                display_name: "Finger Beam".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: FINGER_BEAM_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: FINGER_BEAM_COOLDOWN_TICKS,
                    unlock_mastery: FINGER_BEAM_UNLOCK_MASTERY,
                    damage: FINGER_BEAM_DAMAGE,
                    range: FINGER_BEAM_RANGE,
                    knockback: FINGER_BEAM_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "light_kick"),
                display_name: "Light Kick Beam".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: FOOT_BEAM_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: FOOT_BEAM_COOLDOWN_TICKS,
                    unlock_mastery: FOOT_BEAM_UNLOCK_MASTERY,
                    damage: FOOT_BEAM_DAMAGE,
                    range: FOOT_BEAM_RANGE,
                    knockback: FOOT_BEAM_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "amaterasu"),
                display_name: "Amaterasu".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: AMATERASU_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: AMATERASU_COOLDOWN_TICKS,
                    unlock_mastery: AMATERASU_UNLOCK_MASTERY,
                    damage: AMATERASU_DAMAGE,
                    range: AMATERASU_FLASH_RADIUS as f32,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "ama_no_murakumo"),
                display_name: "Ama no Murakumo".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: AMA_NO_MURAKUMO_ENERGY,
                    energy_per_second: AMA_NO_MURAKUMO_DRAIN_PER_SECOND,
                    cooldown_ticks: AMA_NO_MURAKUMO_COOLDOWN_TICKS,
                    unlock_mastery: AMA_NO_MURAKUMO_UNLOCK_MASTERY,
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
                id: AbilityId::new(FruitId::PikaPika, "murakumo_slash"),
                display_name: "Ama no Murakumo Slash".into(),
                cast_type: CastType::Combo,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: MURAKUMO_SLASH_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: MURAKUMO_SLASH_COOLDOWN_TICKS,
                    unlock_mastery: 4,
                    damage: MURAKUMO_SLASH_DAMAGE,
                    range: MURAKUMO_SLASH_RANGE,
                    knockback: MURAKUMO_SLASH_KNOCKBACK,
                    max_combo_steps: 5,
                    combo_window_ticks: 18,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "murakumo_parry"),
                display_name: "Ama no Murakumo Parry".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: MURAKUMO_PARRY_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: MURAKUMO_PARRY_COOLDOWN_TICKS,
                    unlock_mastery: 4,
                    damage: 0.0,
                    range: 0.0,
                    knockback: MURAKUMO_PARRY_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "yata_no_kagami"),
                display_name: "Yata no Kagami".into(),
                cast_type: CastType::Toggle,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: LIGHT_FLIGHT_ENERGY,
                    energy_per_second: LIGHT_FLIGHT_DRAIN_PER_SECOND,
                    cooldown_ticks: LIGHT_FLIGHT_COOLDOWN_TICKS,
                    unlock_mastery: LIGHT_FLIGHT_UNLOCK_MASTERY,
                    damage: LIGHT_FLIGHT_KICK_DAMAGE,
                    range: 32.0,
                    knockback: 0.9,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "light_clones"),
                display_name: "Light Clones".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed.clone(),
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: LIGHT_CLONES_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: LIGHT_CLONES_COOLDOWN_TICKS,
                    unlock_mastery: LIGHT_CLONES_UNLOCK_MASTERY,
                    damage: 0.0,
                    range: LIGHT_CLONES_RING_RADIUS as f32,
                    knockback: 0.0,
                    ..AbilityStats::default()
                },
                is_ultimate: false,
                stops_movement_while_charging: false,
                allows_cancel_on_hurt: false,
            },
            AbilityDef {
                id: AbilityId::new(FruitId::PikaPika, "yasakani"),
                display_name: "Yasakani no Magatama".into(),
                cast_type: CastType::Instant,
                allowed_forms: allowed,
                stats: AbilityStats {
                    enabled: true,
                    energy_cost: YASAKANI_ENERGY,
                    energy_per_second: 0.0,
                    cooldown_ticks: YASAKANI_COOLDOWN_TICKS,
                    unlock_mastery: YASAKANI_UNLOCK_MASTERY,
                    damage: YASAKANI_DAMAGE,
                    range: YASAKANI_RANGE,
                    knockback: YASAKANI_KNOCKBACK,
                    ..AbilityStats::default()
                },
                is_ultimate: true,
                stops_movement_while_charging: true,
                allows_cancel_on_hurt: false,
            },
        ]
    }

    fn ability(&self, name: &str) -> Option<AbilityDef> {
        let canonical = Self::canonical_ability_name(name);
        self.abilities()
            .into_iter()
            .find(|a| a.id.name == canonical || a.id.full_key() == name)
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(GrandLineDamageType::PikaLight),
            ..AbilityExecutionResult::default()
        };

        let raw_name = match event {
            AbilityEvent::CastInstant { ability, .. }
            | AbilityEvent::ChargeStarted { ability, .. }
            | AbilityEvent::ChargeReleased { ability, .. }
            | AbilityEvent::ChargeCancelled { ability }
            | AbilityEvent::ChannelStarted { ability }
            | AbilityEvent::ChannelTick { ability, .. }
            | AbilityEvent::ChannelEnded { ability, .. }
            | AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::ToggleTick { ability }
            | AbilityEvent::ComboStep { ability, .. }
            | AbilityEvent::FormSwitchRequested { ability } => ability.as_str(),
        };
        let ability_name = Self::canonical_ability_name(raw_name);
        let Some(def) = self.ability(ability_name) else {
            return res;
        };

        let now = ctx.ability_ctx.tick;
        let eye = ctx.ability_ctx.eye_pos;
        let look = ctx.ability_ctx.look_dir.normalize_or_zero();
        let horiz_look = DVec3::new(look.x, 0.0, look.z).normalize_or_zero();
        let base_scaled_dmg = def.stats.scaled_damage(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        );
        let scaled_range = def.stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) as f64;

        match ability_name {
            "laser" => {
                // `FingerBeamAbility`: pierces up to 2 targets along `range = 96.0`, `boltRadius = 0.25`.
                res.animation_clip = Some(pika_anim::FINGER_SHOT.into());
                res.spawned_projectiles = 1;
                let from = eye + look * 0.62;
                let end = from + look * scaled_range;

                ctx.world_fx.emit_fx(
                    pika_fx::FINGER_AIM,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    now,
                    FINGER_BEAM_FIRE_TICK,
                    64.0,
                );

                let mut hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    from,
                    end,
                    FINGER_BEAM_BOLT_RADIUS,
                );
                hits.sort_by(|a, b| {
                    let da = (a.center - from).dot(look);
                    let db = (b.center - from).dot(look);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                });
                let struck: Vec<_> = hits.into_iter().take(FINGER_BEAM_MAX_TARGETS).collect();
                let beam_end = if struck.len() == FINGER_BEAM_MAX_TARGETS {
                    let last = struck[struck.len() - 1];
                    let along = (last.center - from).dot(look).max(0.0);
                    from + look * along
                } else {
                    end
                };

                let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.08, 0.0);
                for t in &struck {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::PikaLight,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                let flags = if struck.is_empty() { 1 } else { 2 };
                ctx.world_fx.emit_fx(
                    pika_fx::FINGER,
                    0,
                    from,
                    beam_end,
                    FINGER_BEAM_BOLT_RADIUS as f32,
                    now,
                    flags,
                    128.0,
                );
            }
            "light_kick" => {
                // `FootBeamAbility`: heavy light kick beam (`range = 64.0`, `beamRadius = 1.2`) + `LightBlast.detonate` (`burstRadius = 3.5`).
                res.animation_clip = Some(pika_anim::FOOT_BEAM.into());
                res.spawned_projectiles = 1;
                let from = eye + DVec3::new(0.0, -0.8, 0.0) + look * 0.8;
                let end = from + look * scaled_range;

                ctx.world_fx.emit_fx(
                    pika_fx::FOOT_AIM,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    now,
                    FOOT_BEAM_FIRE_TICK,
                    64.0,
                );
                ctx.world_fx.emit_fx(
                    pika_fx::FOOT,
                    0,
                    from,
                    end,
                    FOOT_BEAM_BURST_RADIUS as f32,
                    now,
                    1,
                    128.0,
                );

                // Beam capsule hits.
                let beam_hits = CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    from,
                    end,
                    FOOT_BEAM_RADIUS,
                );
                let push = look * (def.stats.knockback as f64) + DVec3::new(0.0, 0.3, 0.0);
                for t in beam_hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::PikaLight,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                // Terminal light blast detonation (`LightBlast.detonate`).
                let burst_at = ctx.ability_ctx.aim_point.unwrap_or(end - look * 0.3);
                let mut burst_budget = CastBudget::new(
                    (4.2 * FOOT_BEAM_BURST_RADIUS.powi(3)).ceil() as usize,
                    12,
                );
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    burst_at,
                    FOOT_BEAM_BURST_RADIUS,
                    &mut burst_budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );

                let burst_hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    burst_at,
                    FOOT_BEAM_BURST_RADIUS + 0.8,
                );
                let burst_base = (FOOT_BEAM_BURST_DAMAGE / FOOT_BEAM_DAMAGE) * base_scaled_dmg;
                for t in burst_hits {
                    if !res.hits.iter().any(|h| h.target_id == t.id) {
                        let d = t.center.distance(burst_at);
                        let k = 1.0 - 0.5 * (d / FOOT_BEAM_BURST_RADIUS).clamp(0.0, 1.0);
                        let out = if d < 0.001 {
                            DVec3::Y
                        } else {
                            (t.center - burst_at) / d
                        };
                        let kb = out * (def.stats.knockback as f64 * 0.8 * k)
                            + DVec3::new(0.0, 0.25 * k, 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: burst_base * (k as f32),
                            damage_type: GrandLineDamageType::PikaLight,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                kb,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                    }
                }
            }
            "amaterasu" => {
                res.animation_clip = Some(pika_anim::AMATERASU.into());
                let flash_at = eye + look * 0.82;
                let hits = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    flash_at,
                    AMATERASU_FLASH_RADIUS,
                );
                let caught_count = hits.len() as i32;
                for t in hits {
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::PikaLight,
                        knockback_impulse: DVec3::ZERO,
                        stun_ticks: AMATERASU_SLOW_TICKS,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
                ctx.world_fx.emit_fx(
                    pika_fx::AMATERASU,
                    0,
                    flash_at,
                    eye,
                    AMATERASU_FLASH_RADIUS as f32,
                    now,
                    caught_count,
                    64.0,
                );
            }
            "ama_no_murakumo" => {
                if let AbilityEvent::Toggled { enabled, .. } = event {
                    res.cast_result = Some(CastResult::Toggled);
                    if *enabled {
                        res.animation_clip = Some(pika_anim::SWORD_SUMMON.into());
                        ctx.world_fx.emit_fx(
                            pika_fx::SWORD_SUMMON,
                            0,
                            eye,
                            DVec3::ZERO,
                            0.0,
                            now,
                            AMA_NO_MURAKUMO_SUMMON_TICKS,
                            64.0,
                        );
                    } else {
                        ctx.world_fx.emit_fx(
                            pika_fx::SWORD_DISMISS,
                            0,
                            eye,
                            DVec3::ZERO,
                            0.0,
                            now,
                            0,
                            64.0,
                        );
                    }
                }
            }
            "murakumo_slash" => {
                let step = match event {
                    AbilityEvent::ComboStep { step, .. } => (*step).clamp(1, 5) - 1,
                    _ => ctx.ability_ctx.combo_step.clamp(1, 5) - 1,
                } as usize;
                res.cast_result = Some(CastResult::ComboStep);
                res.animation_clip = Some(format!("devilfruits:pika_pika/sword_m1_{}", step + 1));

                let slash_dmg = base_scaled_dmg * MURAKUMO_SLASH_POWER[step];
                let half_angle = (MURAKUMO_SLASH_ARC_DEGREES * 0.5).to_radians();
                let cone_hits = CombatTargeting::in_cone(
                    ctx.nearby_targets,
                    ctx.player_id,
                    eye,
                    look,
                    scaled_range,
                    half_angle,
                );
                let kb_scale = def.stats.knockback as f64 * if step == 4 { 1.6 } else { 1.0 };
                let push_vec = horiz_look * kb_scale + DVec3::new(0.0, 0.12, 0.0);
                let mut any_struck = false;
                for t in cone_hits {
                    any_struck = true;
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: slash_dmg,
                        damage_type: GrandLineDamageType::PikaLight,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push_vec,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }

                ctx.world_fx.emit_fx(
                    pika_fx::SWORD_SWING,
                    0,
                    eye,
                    look,
                    scaled_range as f32,
                    now,
                    (step as i32) | if any_struck { 8 } else { 0 },
                    64.0,
                );

                // 5th combo cut launches a light crescent (`MurakumoSlashAbility.crescent`).
                if step == 4 {
                    res.spawned_projectiles = 1;
                    let crescent_dmg =
                        (MURAKUMO_SLASH_CRESCENT_DAMAGE / MURAKUMO_SLASH_DAMAGE) * base_scaled_dmg;
                    let crescent_half = MURAKUMO_SLASH_CRESCENT_SPAN * 0.5;
                    let crescent_end = eye + look * MURAKUMO_SLASH_CRESCENT_RANGE;
                    ctx.world_fx.emit_fx(
                        pika_fx::SWORD_CRESCENT,
                        0,
                        eye + look * 1.2 + DVec3::new(0.0, -0.35, 0.0),
                        look * 1.6,
                        crescent_half as f32,
                        now,
                        15,
                        96.0,
                    );
                    let crescent_hits = CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        crescent_end,
                        crescent_half,
                    );
                    for t in crescent_hits {
                        if !res.hits.iter().any(|h| h.target_id == t.id) {
                            res.hits.push(AbilityHitEffect {
                                target_id: t.id,
                                damage: crescent_dmg,
                                damage_type: GrandLineDamageType::PikaLight,
                                knockback_impulse: CombatTargeting::launch_velocity(
                                    look * (def.stats.knockback as f64 * 1.4)
                                        + DVec3::new(0.0, 0.25, 0.0),
                                    t.knockback_resistance,
                                ),
                                stun_ticks: 0,
                                chill_stacks_added: 0,
                                chill_burst: None,
                                freeze_ticks: 0,
                                set_fire_seconds: 0,
                            });
                        }
                    }
                    ctx.world_fx.emit_fx(
                        pika_fx::SWORD_CRESCENT_END,
                        0,
                        crescent_end,
                        look * 1.6,
                        crescent_half as f32,
                        now,
                        0,
                        96.0,
                    );
                }
            }
            "murakumo_parry" => {
                res.animation_clip = Some(pika_anim::SWORD_PARRY.into());
                ctx.world_fx.emit_fx(
                    pika_fx::SWORD_PARRY,
                    0,
                    eye,
                    look,
                    0.0,
                    now,
                    MURAKUMO_PARRY_WINDOW_TICKS,
                    64.0,
                );
            }
            "yata_no_kagami" => {
                // `LightFlightAbility` / `LightFlight.java`: mirror reflection light beam flight & teleport raycast.
                let enabled = match event {
                    AbilityEvent::Toggled { enabled, .. } => *enabled,
                    _ => true,
                };
                res.cast_result = Some(CastResult::Toggled);
                ctx.form_runtime.set_flag(form_flags::FLYING, enabled);

                if enabled {
                    if let Some(logia) = ctx.logia.as_deref_mut() {
                        logia.state = LogiaState::ElementalBody;
                        logia.dissolved_limbs = limbs::ALL;
                    }
                    let dest = ctx
                        .ability_ctx
                        .aim_point
                        .unwrap_or(eye + look * scaled_range);
                    let out_dir = (dest - eye).normalize_or_zero();
                    let speed = LIGHT_FLIGHT_SPEED;
                    res.caster_velocity_impulse = Some(out_dir * speed);

                    let packed_flight = LIGHT_FLIGHT_ACCEL_TICKS
                        | (((LIGHT_FLIGHT_BOOST_SPEED * 10.0).round() as i32) << 8);
                    ctx.world_fx.emit_fx(
                        pika_fx::FLIGHT_START,
                        0,
                        eye,
                        look,
                        speed as f32,
                        now,
                        packed_flight,
                        96.0,
                    );

                    // Emit mirror reflection turn VFX (`PikaFx.MIRROR` with `PikaFx.packDir(out)`).
                    let mirror_vertex = eye + look * (speed * 2.0);
                    ctx.world_fx.emit_fx(
                        pika_fx::MIRROR,
                        0,
                        mirror_vertex,
                        look,
                        2.0,
                        now,
                        Self::pack_dir(out_dir),
                        96.0,
                    );

                    // `LightFlight.kicks`: creatures in the beam path take `kickDamage + kickPerSpeed * speed` (`6.0 + 1.5 * 2.0 = 9.0`).
                    let kick_dmg =
                        base_scaled_dmg + LIGHT_FLIGHT_KICK_PER_SPEED * (speed as f32);
                    let path_hits = CombatTargeting::in_capsule(
                        ctx.nearby_targets,
                        ctx.player_id,
                        eye,
                        dest,
                        1.0,
                    );
                    for t in path_hits {
                        let push = out_dir * 0.5 + DVec3::new(0.0, 0.45, 0.0);
                        res.hits.push(AbilityHitEffect {
                            target_id: t.id,
                            damage: kick_dmg,
                            damage_type: GrandLineDamageType::PikaLight,
                            knockback_impulse: CombatTargeting::launch_velocity(
                                push,
                                t.knockback_resistance,
                            ),
                            stun_ticks: 0,
                            chill_stacks_added: 0,
                            chill_burst: None,
                            freeze_ticks: 0,
                            set_fire_seconds: 0,
                        });
                        ctx.world_fx.emit_fx(
                            pika_fx::FLIGHT_KICK,
                            0,
                            t.center,
                            out_dir,
                            speed as f32,
                            now,
                            0,
                            64.0,
                        );
                    }
                } else {
                    ctx.world_fx.emit_fx(
                        pika_fx::FLIGHT_END,
                        0,
                        eye,
                        look,
                        1.0,
                        now,
                        0,
                        96.0,
                    );
                }
            }
            "light_clones" => {
                res.animation_clip = Some(pika_anim::CLONES_CAST.into());
                res.spawned_projectiles = LIGHT_CLONES_BODIES;
                res.caster_velocity_impulse = Some(DVec3::new(0.0, 0.5, 0.0));
                if let Some(logia) = ctx.logia.as_deref_mut() {
                    logia.state = LogiaState::ElementalBody;
                    logia.dissolved_limbs = limbs::ALL;
                }
                ctx.world_fx.emit_fx(
                    pika_fx::CLONE_SPLIT,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    now,
                    10,
                    96.0,
                );
                for i in 0..LIGHT_CLONES_BODIES {
                    let angle = (i as f64) * std::f64::consts::TAU / (LIGHT_CLONES_BODIES as f64);
                    let spot = eye
                        + DVec3::new(
                            angle.cos() * LIGHT_CLONES_RING_RADIUS,
                            0.0,
                            angle.sin() * LIGHT_CLONES_RING_RADIUS,
                        );
                    ctx.world_fx.emit_fx(
                        pika_fx::CLONE_BEAM,
                        0,
                        eye,
                        spot,
                        10.0,
                        now + i as i64,
                        i as i32,
                        96.0,
                    );
                }
            }
            "yasakani" => {
                // `YasakaniAbility.java`: jumps upward (`1.25`), hovers in star posture,
                // and fires `ceil((14.0 / 20.0) * 80) = 56` light jewel bolts with `blastRadius = 2.5`.
                res.animation_clip = Some(pika_anim::ULT_JUMP.into());
                res.caster_velocity_impulse = Some(DVec3::new(0.0, YASAKANI_JUMP_SPEED, 0.0));
                let total_bolts =
                    ((YASAKANI_BEAMS_PER_SECOND / 20.0) * (YASAKANI_ATTACK_TICKS as f64)).ceil()
                        as usize;
                res.spawned_projectiles = total_bolts;

                let star_pos = eye + DVec3::new(0.0, -0.35, 0.0) + look * 1.4;
                let impact_center = ctx
                    .ability_ctx
                    .aim_point
                    .unwrap_or(star_pos + look * scaled_range);

                ctx.world_fx.emit_fx(
                    pika_fx::ULT_JUMP,
                    0,
                    eye,
                    DVec3::ZERO,
                    0.0,
                    now,
                    YASAKANI_JUMP_TICKS,
                    96.0,
                );
                ctx.world_fx.emit_fx(
                    pika_fx::ULT_STAR,
                    0,
                    eye,
                    DVec3::ZERO,
                    12.0,
                    now,
                    YASAKANI_STAR_TICKS + YASAKANI_ATTACK_TICKS,
                    160.0,
                );
                ctx.world_fx.emit_fx(
                    pika_fx::ULT_BOLT,
                    0,
                    star_pos,
                    impact_center,
                    YASAKANI_BLAST_RADIUS as f32,
                    now,
                    0,
                    160.0,
                );
                ctx.world_fx.emit_shake(impact_center, 0.85, 32);
                res.screen_shake = Some((0.85, 32));

                let mut volley_budget = CastBudget::new(YASAKANI_MAX_BLOCKS, 40);
                res.destroyed_blocks = ctx.world_fx.carve_spherical_crater(
                    impact_center,
                    YASAKANI_BLAST_RADIUS,
                    &mut volley_budget,
                    false,
                    now,
                    ctx.config,
                    ctx.game_rules,
                    |_| ("minecraft:stone".into(), 1.5, false),
                );

                // Targets within `spreadRadius + blastRadius` of the aimed hail zone or along the ray capsule are hit.
                let hail_radius = YASAKANI_SPREAD_RADIUS + YASAKANI_BLAST_RADIUS;
                let mut hit_targets = CombatTargeting::in_radius(
                    ctx.nearby_targets,
                    ctx.player_id,
                    impact_center,
                    hail_radius,
                );
                for cap_t in CombatTargeting::in_capsule(
                    ctx.nearby_targets,
                    ctx.player_id,
                    star_pos,
                    impact_center,
                    YASAKANI_SPREAD_RADIUS,
                ) {
                    if !hit_targets.iter().any(|existing| existing.id == cap_t.id) {
                        hit_targets.push(cap_t);
                    }
                }

                for t in hit_targets {
                    let out = (t.center - impact_center).normalize_or_zero();
                    let push = out * (def.stats.knockback as f64) + DVec3::new(0.0, 0.25, 0.0);
                    res.hits.push(AbilityHitEffect {
                        target_id: t.id,
                        damage: base_scaled_dmg,
                        damage_type: GrandLineDamageType::PikaLight,
                        knockback_impulse: CombatTargeting::launch_velocity(
                            push,
                            t.knockback_resistance,
                        ),
                        stun_ticks: 0,
                        chill_stacks_added: 0,
                        chill_burst: None,
                        freeze_ticks: 0,
                        set_fire_seconds: 0,
                    });
                }
            }
            _ => {}
        }

        res.vfx_events = ctx.world_fx.drain_vfx();
        res
    }

    fn on_tick(&self, ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        if ctx.suppressed || ctx.submerged_in_water {
            ctx.form_runtime.set_flag(form_flags::FLYING, false);
        }
        AbilityExecutionResult::default()
    }

    fn body_modifiers(
        &self,
        player: &PlayerFruitData,
        form_runtime: &FormRuntime,
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
        if form_runtime.has_flag(form_flags::FLYING) {
            mods.reset_above_ground_ticks = true;
            mods.passes_through_leaves = true;
        }
        mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ability::{AbilityContext, AbilityModifiers, TechniqueHeat};
    use crate::core::combat::TargetCandidate;
    use crate::core::gear::GearState;
    use crate::core::haki::PlayerHakiData;
    use crate::core::logia::LogiaData;
    use crate::core::rules::{CoreConfig, DFGameRules};
    use crate::core::world_fx::WorldFxManager;

    fn make_target(id_byte: u8, pos: DVec3) -> TargetCandidate {
        TargetCandidate {
            id: [id_byte; 16],
            center: pos,
            width: 0.6,
            height: 1.8,
            knockback_resistance: 0.0,
        }
    }

    #[test]
    fn pika_pika_abilities_and_logia_profile_match_java() {
        let module = PikaPikaModule::new();
        assert_eq!(module.fruit_id(), FruitId::PikaPika);
        assert_eq!(module.primary_damage_type(), GrandLineDamageType::PikaLight);

        let profile = module.logia_profile().expect("Pika-Pika is a Logia");
        assert_eq!(profile.element, "light");
        assert_eq!(profile.elemental_drain_per_second, 3.0);
        assert_eq!(profile.dash_range, 32.0);
        assert_eq!(profile.dash_speed, 4.5);

        // Verify all required abilities & aliases (`light_beam`/`laser`, `light_kick`/`foot_beam`,
        // `ama_no_murakumo`, `yata_no_kagami`/`light_flight`, `yasakani_no_magatama`/`yasakani`).
        let beam = module.ability("light_beam").unwrap();
        assert_eq!(beam.id.name, "laser");
        assert_eq!(beam.stats.energy_cost, 4.0);
        assert_eq!(beam.stats.cooldown_ticks, 6);
        assert_eq!(beam.stats.damage, 3.0);
        assert_eq!(beam.stats.range, 96.0);

        let kick = module.ability("light_kick").unwrap();
        assert_eq!(kick.stats.energy_cost, 28.0);
        assert_eq!(kick.stats.cooldown_ticks, 200);
        assert_eq!(kick.stats.unlock_mastery, 2);
        assert_eq!(kick.stats.damage, 16.0);
        assert_eq!(kick.stats.range, 64.0);

        let sword = module.ability("ama_no_murakumo").unwrap();
        assert_eq!(sword.cast_type, CastType::Toggle);
        assert_eq!(sword.stats.energy_cost, 10.0);
        assert_eq!(sword.stats.energy_per_second, 1.5);
        assert_eq!(sword.stats.cooldown_ticks, 40);
        assert_eq!(sword.stats.unlock_mastery, 4);

        let yata = module.ability("yata_no_kagami").unwrap();
        assert_eq!(yata.cast_type, CastType::Toggle);
        assert_eq!(yata.stats.energy_cost, 12.0);
        assert_eq!(yata.stats.energy_per_second, 5.0);
        assert_eq!(yata.stats.cooldown_ticks, 60);
        assert_eq!(yata.stats.unlock_mastery, 5);

        let yasakani = module.ability("yasakani_no_magatama").unwrap();
        assert!(yasakani.is_ultimate);
        assert_eq!(yasakani.stats.energy_cost, 90.0);
        assert_eq!(yasakani.stats.cooldown_ticks, 2400);
        assert_eq!(yasakani.stats.unlock_mastery, 9);
        assert_eq!(yasakani.stats.damage, 12.0);
        assert_eq!(yasakani.stats.range, 80.0);
    }

    #[test]
    fn pika_pika_mirror_reflection_and_pack_dir_math() {
        let dir = DVec3::new(0.6, 0.0, 0.8).normalize();
        let packed = PikaPikaModule::pack_dir(dir);
        let unpacked = PikaPikaModule::unpack_dir(packed);
        assert!(dir.distance(unpacked) < 1e-3);

        // 90° mirror bounce off a diagonal normal.
        let in_dir = DVec3::Z;
        let out_dir = DVec3::X;
        let normal = PikaPikaModule::mirror_normal(in_dir, out_dir);
        let reflected = PikaPikaModule::mirror_reflect(in_dir, normal);
        assert!(reflected.distance(out_dir) < 1e-6);

        let path = PikaPikaModule::trace_mirror_path(DVec3::ZERO, in_dir, &[normal], 10.0);
        assert_eq!(path.len(), 3);
        assert!(path[1].distance(DVec3::new(0.0, 0.0, 10.0)) < 1e-6);
        assert!(path[2].distance(DVec3::new(10.0, 0.0, 10.0)) < 1e-6);
    }

    #[test]
    fn pika_pika_execute_all_abilities_and_multi_volley_yasakani() {
        let module = PikaPikaModule::new();
        let cfg = CoreConfig::default();
        let mut player = PlayerFruitData {
            fruit_id: Some(FruitId::PikaPika),
            mastery: 10,
            energy: 271.0,
            active_form: "base".into(),
            ..PlayerFruitData::default()
        };
        let mut form_rt = FormRuntime::default();
        let mut gear = GearState::default();
        let mut haki = PlayerHakiData::default();
        let mut logia = LogiaData::default();
        let mut heat = TechniqueHeat::default();
        let mut world_fx = WorldFxManager::default();
        let targets = vec![
            make_target(2, DVec3::new(0.0, 64.0, 5.0)),
            make_target(3, DVec3::new(0.0, 64.0, 10.0)),
            make_target(4, DVec3::new(0.0, 64.0, 15.0)),
        ];

        let mut ctx = FruitContext {
            player_id: [1; 16],
            player: &mut player,
            form_runtime: &mut form_rt,
            gear_state: &mut gear,
            haki: &mut haki,
            logia: Some(&mut logia),
            heat: &mut heat,
            world_fx: &mut world_fx,
            budget: CastBudget::from_config(&cfg),
            ability_ctx: AbilityContext {
                caster: [1; 16],
                fruit: FruitId::PikaPika,
                ability_name: "laser".into(),
                mastery: 10,
                charge: 1.0,
                combo_step: 1,
                channel_ticks: 0,
                tick: 100,
                eye_pos: DVec3::new(0.0, 64.0, 0.0),
                look_dir: DVec3::Z,
                aim_point: Some(DVec3::new(0.0, 64.0, 10.0)),
                modifiers: AbilityModifiers::IDENTITY,
            },
            nearby_targets: &targets,
            config: &cfg,
            game_rules: DFGameRules::vanilla_default(),
            creative: false,
            submerged_in_water: false,
            suppressed: false,
            on_ground: true,
            in_lava: false,
        };

        // 1. Finger Beam (`laser` / `light_beam`) pierces at most 2 targets and stops in the 2nd.
        let r_beam = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "light_beam".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_beam.hits.len(), FINGER_BEAM_MAX_TARGETS);

        // 2. Light Kick (`light_kick` / `foot_beam`) pierces all targets and detonates a crater at the end.
        let r_kick = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "light_kick".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_kick.hits.len(), 3);
        assert!(!r_kick.destroyed_blocks.is_empty());

        // 3. Yata no Kagami (`yata_no_kagami`) enters ElementalBody, emits mirror VFX, and kicks targets along the beam.
        let r_yata = module.execute_ability(
            &mut ctx,
            &AbilityEvent::Toggled {
                ability: "yata_no_kagami".into(),
                enabled: true,
            },
        );
        assert_eq!(r_yata.cast_result, Some(CastResult::Toggled));
        assert!(r_yata.caster_velocity_impulse.is_some());
        assert_eq!(ctx.logia.as_ref().unwrap().state, LogiaState::ElementalBody);
        assert!(r_yata.vfx_events.iter().any(|e| e.fx_id == pika_fx::MIRROR));
        assert_eq!(r_yata.hits.len(), 2);

        let mods = module.body_modifiers(ctx.player, ctx.form_runtime, false, 0.0);
        assert!(mods.reset_above_ground_ticks);
        assert!(mods.passes_through_leaves);

        // 4. Yasakani no Magatama launches 56 light jewel bolts (`14/sec * 4s`), jumps upward, and carves craters.
        let r_ult = module.execute_ability(
            &mut ctx,
            &AbilityEvent::CastInstant {
                ability: "yasakani_no_magatama".into(),
                mastery: 10,
            },
        );
        assert_eq!(r_ult.spawned_projectiles, 56);
        assert_eq!(
            r_ult.caster_velocity_impulse,
            Some(DVec3::new(0.0, YASAKANI_JUMP_SPEED, 0.0))
        );
        assert!(!r_ult.hits.is_empty());
        assert!(!r_ult.destroyed_blocks.is_empty());
    }
}
