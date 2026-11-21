//! Pirate mob archetypes (`PirateDeckhand`, `PirateGunner`, `PirateCaptain`),
//! stats, 16 animation clips, AI tick state machines, flintlock/musket ballistics,
//! captain slam/lunge/rally/enrage, bounty rewards, and loot drops.
//!
//! Mirrors `mobs/pirate/{Pirates,PirateConfig,PirateClips,PirateAmbush,PirateFx,
//! entity/PirateFighter,entity/PirateDeckhand,entity/PirateGunner,entity/PirateCaptain,
//! entity/MusketBall,goal/PirateMeleeGoal,goal/PirateRetreatGoal,goal/GunnerGoal,
//! goal/CaptainGoal,item/FlintlockItem}.java`.

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::itemtech::{
    CAPTAIN_CUTLASS_ITEM_ID, CUTLASS_ITEM_ID, FLINTLOCK_PISTOL_ITEM_ID, GOLD_KEY_ITEM_ID,
    MUSKET_BALL_ITEM_ID, MUSKET_ITEM_ID,
};

/// 16 canonical Pirate animation clip IDs (`com.devilfruits.mobs.pirate.PirateClips`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PirateClip {
    Draw = 0,
    Slash1 = 1,
    Slash2 = 2,
    Barge = 3,
    Taunt = 4,
    FirePistol = 5,
    ReloadPistol = 6,
    FireMusket = 7,
    ReloadMusket = 8,
    FlinchRetreat = 9,
    CaptainSlash = 10,
    Slam = 11,
    Lunge = 12,
    Quickdraw = 13,
    Rally = 14,
    Enrage = 15,
}

impl PirateClip {
    pub const NAMES: [&'static str; 16] = [
        "draw",
        "slash_1",
        "slash_2",
        "barge",
        "taunt",
        "fire_pistol",
        "reload_pistol",
        "fire_musket",
        "reload_musket",
        "flinch_retreat",
        "captain_slash",
        "slam",
        "lunge",
        "quickdraw",
        "rally",
        "enrage",
    ];

    /// Clip length in ticks (`PirateClips.LENGTH`).
    pub const LENGTH: [i32; 16] = [10, 12, 12, 16, 30, 12, 100, 16, 160, 12, 18, 40, 20, 16, 32, 30];

    /// Strike/shot impact tick inside the clip (`PirateClips.HIT`).
    pub const HIT: [i32; 16] = [5, 6, 5, 8, -1, 2, 92, 3, 148, -1, 9, 26, 11, 8, 12, 14];

    /// Movement impulse tick inside the clip (`PirateClips.MOVE`).
    pub const MOVE: [i32; 16] = [-1, -1, -1, 4, -1, -1, -1, -1, -1, 3, -1, -1, 6, -1, -1, -1];

    pub const fn length(self) -> i32 {
        Self::LENGTH[self as usize]
    }

    pub const fn hit_tick(self) -> i32 {
        Self::HIT[self as usize]
    }

    pub const fn move_tick(self) -> i32 {
        Self::MOVE[self as usize]
    }

    pub const fn as_str(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "draw" => Some(Self::Draw),
            "slash_1" => Some(Self::Slash1),
            "slash_2" => Some(Self::Slash2),
            "barge" => Some(Self::Barge),
            "taunt" => Some(Self::Taunt),
            "fire_pistol" => Some(Self::FirePistol),
            "reload_pistol" => Some(Self::ReloadPistol),
            "fire_musket" => Some(Self::FireMusket),
            "reload_musket" => Some(Self::ReloadMusket),
            "flinch_retreat" => Some(Self::FlinchRetreat),
            "captain_slash" => Some(Self::CaptainSlash),
            "slam" => Some(Self::Slam),
            "lunge" => Some(Self::Lunge),
            "quickdraw" => Some(Self::Quickdraw),
            "rally" => Some(Self::Rally),
            "enrage" => Some(Self::Enrage),
            _ => None,
        }
    }
}

/// Complete Pirate configuration (`com.devilfruits.mobs.pirate.PirateConfig`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PirateConfig {
    pub spawn_chance: f32,
    pub max_nearby: usize,
    pub captain_crew: bool,
    pub crew_deckhands_min: u32,
    pub crew_deckhands_max: u32,
    pub crew_gunners_min: u32,
    pub crew_gunners_max: u32,
    pub hunt_villagers: bool,
    pub hunt_golems: bool,
    pub hunt_illagers: bool,
    pub friendly_fire: bool,
    pub target_scan_interval: i32,
    pub gear_drop_chance: f32,

    pub deckhand_health: f32,
    pub deckhand_damage: f32,
    pub deckhand_speed: f32,
    pub deckhand_combo_chance: f32,
    pub deckhand_barge_cooldown: i32,
    pub deckhand_retreat_health: f32,

    pub gunner_health: f32,
    pub gunner_speed: f32,
    pub gunner_musket_chance: f32,
    pub gunner_pistol_damage: f32,
    pub gunner_musket_damage: f32,
    pub gunner_min_range: f64,
    pub gunner_max_range: f64,
    pub gunner_aim_ticks: i32,
    pub gunner_inaccuracy: f32,

    pub captain_health: f32,
    pub captain_damage: f32,
    pub captain_armor: f32,
    pub captain_speed: f32,
    pub captain_scale: f32,
    pub captain_boss_bar: bool,
    pub captain_slam_cooldown: i32,
    pub captain_lunge_cooldown: i32,
    pub captain_pistol_cooldown: i32,
    pub captain_rally_cooldown: i32,
    pub captain_slam_radius: f64,
    pub captain_slam_damage: f32,
    pub captain_slam_block_damage: bool,
    pub captain_pistol_damage: f32,
    pub captain_enrage_health: f32,
    pub captain_enrage_cooldown_factor: f32,
    pub rally_radius: f64,
    pub rally_seconds: i32,

    pub pistol_damage: f32,
    pub musket_damage: f32,
    pub pistol_reload_ticks: i32,
    pub musket_reload_ticks: i32,
    pub pistol_cooldown: i32,
    pub musket_cooldown: i32,
    pub pistol_spread: f32,
    pub musket_spread: f32,
    pub ball_speed: f32,
    pub armor_piercing: f32,
    pub gunpowder_per_shot: bool,
}

impl Default for PirateConfig {
    fn default() -> Self {
        Self {
            spawn_chance: 1.0,
            max_nearby: 12,
            captain_crew: true,
            crew_deckhands_min: 2,
            crew_deckhands_max: 4,
            crew_gunners_min: 1,
            crew_gunners_max: 2,
            hunt_villagers: true,
            hunt_golems: true,
            hunt_illagers: false,
            friendly_fire: false,
            target_scan_interval: 10,
            gear_drop_chance: 0.085,
            deckhand_health: 24.0,
            deckhand_damage: 5.0,
            deckhand_speed: 0.26,
            deckhand_combo_chance: 0.6,
            deckhand_barge_cooldown: 120,
            deckhand_retreat_health: 0.25,
            gunner_health: 20.0,
            gunner_speed: 0.30,
            gunner_musket_chance: 0.30,
            gunner_pistol_damage: 6.0,
            gunner_musket_damage: 12.0,
            gunner_min_range: 8.0,
            gunner_max_range: 16.0,
            gunner_aim_ticks: 14,
            gunner_inaccuracy: 2.5,
            captain_health: 150.0,
            captain_damage: 10.0,
            captain_armor: 8.0,
            captain_speed: 0.30,
            captain_scale: 1.35,
            captain_boss_bar: true,
            captain_slam_cooldown: 140,
            captain_lunge_cooldown: 100,
            captain_pistol_cooldown: 90,
            captain_rally_cooldown: 320,
            captain_slam_radius: 4.5,
            captain_slam_damage: 14.0,
            captain_slam_block_damage: false,
            captain_pistol_damage: 9.0,
            captain_enrage_health: 0.5,
            captain_enrage_cooldown_factor: 1.6,
            rally_radius: 12.0,
            rally_seconds: 6,
            pistol_damage: 8.0,
            musket_damage: 15.0,
            pistol_reload_ticks: 40,
            musket_reload_ticks: 70,
            pistol_cooldown: 10,
            musket_cooldown: 20,
            pistol_spread: 1.5,
            musket_spread: 0.4,
            ball_speed: 4.0,
            armor_piercing: 0.25,
            gunpowder_per_shot: true,
        }
    }
}

/// Pirate mob archetype (`PirateDeckhand`, `PirateGunner`, `PirateCaptain`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PirateArchetype {
    Deckhand,
    Gunner,
    Captain,
}

impl PirateArchetype {
    pub const fn entity_id(self) -> &'static str {
        match self {
            Self::Deckhand => "devilfruits:pirate/deckhand",
            Self::Gunner => "devilfruits:pirate/gunner",
            Self::Captain => "devilfruits:pirate/captain",
        }
    }

    pub const fn rank(self) -> u8 {
        match self {
            Self::Deckhand => 0,
            Self::Gunner => 1,
            Self::Captain => 2,
        }
    }

    /// Bounty points / Berry reward (`FactionFighter.bounty()`: Captain = 50, Gunner = 15, Deckhand = 10).
    pub const fn bounty_points(self) -> u32 {
        match self {
            Self::Deckhand => 10,
            Self::Gunner => 15,
            Self::Captain => 50,
        }
    }

    pub const fn xp_reward(self) -> u32 {
        match self {
            Self::Deckhand | Self::Gunner => 8,
            Self::Captain => 40,
        }
    }

    pub const fn variant_count(self) -> u8 {
        match self {
            Self::Deckhand | Self::Gunner => 4,
            Self::Captain => 2,
        }
    }

    pub fn base_health(self, cfg: &PirateConfig) -> f32 {
        match self {
            Self::Deckhand => cfg.deckhand_health,
            Self::Gunner => cfg.gunner_health,
            Self::Captain => cfg.captain_health,
        }
    }

    pub fn base_damage(self, cfg: &PirateConfig) -> f32 {
        match self {
            Self::Deckhand => cfg.deckhand_damage,
            Self::Gunner => 2.0,
            Self::Captain => cfg.captain_damage,
        }
    }

    pub fn base_armor(self, cfg: &PirateConfig) -> f32 {
        match self {
            Self::Deckhand => 2.0,
            Self::Gunner => 1.0,
            Self::Captain => cfg.captain_armor,
        }
    }

    pub fn base_speed(self, cfg: &PirateConfig) -> f32 {
        match self {
            Self::Deckhand => cfg.deckhand_speed,
            Self::Gunner => cfg.gunner_speed,
            Self::Captain => cfg.captain_speed,
        }
    }

    pub fn scale(self, cfg: &PirateConfig) -> f32 {
        match self {
            Self::Deckhand | Self::Gunner => 1.0,
            Self::Captain => cfg.captain_scale,
        }
    }
}

/// Loot drop entry generated on Pirate death.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PirateLootDrop {
    pub item_id: String,
    pub count: u32,
}

/// Events produced by a Pirate entity during its server AI tick.
#[derive(Clone, Debug, PartialEq)]
pub enum PirateAiEvent {
    MeleeStrike {
        damage: f32,
        knockback: f32,
    },
    FireBall {
        musket: bool,
        damage: f32,
        speed: f32,
        armor_piercing: f32,
    },
    CaptainSlam {
        center: DVec3,
        radius: f64,
        damage: f32,
    },
    CaptainRally {
        radius: f64,
        duration_ticks: i32,
    },
    CaptainEnraged,
    DashImpulse(DVec3),
}

/// Runtime state of a spawned Pirate mob (`PirateFighter` + subclasses).
#[derive(Clone, Debug, PartialEq)]
pub struct PirateMobState {
    pub id: u64,
    pub archetype: PirateArchetype,
    pub variant: u8,
    pub position: DVec3,
    pub velocity: DVec3,
    pub yaw: f32,
    pub health: f32,
    pub max_health: f32,
    pub main_hand_item: String,
    pub off_hand_item: Option<String>,
    pub has_musket: bool,
    pub gun_loaded: bool,
    pub enraged: bool,
    pub leader_id: Option<u64>,
    pub active_clip: Option<PirateClip>,
    pub clip_tick: i32,
    pub clip_duration: i32,
    pub barge_cooldown: i32,
    pub slam_cooldown: i32,
    pub lunge_cooldown: i32,
    pub pistol_cooldown: i32,
    pub rally_cooldown: i32,
    pub aim_ticks: i32,
}

impl PirateMobState {
    pub fn spawn(
        id: u64,
        archetype: PirateArchetype,
        variant: u8,
        position: DVec3,
        has_musket: bool,
        cfg: &PirateConfig,
    ) -> Self {
        let max_health = archetype.base_health(cfg);
        let (main_hand_item, off_hand_item) = match archetype {
            PirateArchetype::Deckhand => (CUTLASS_ITEM_ID.into(), None),
            PirateArchetype::Gunner => (
                if has_musket {
                    MUSKET_ITEM_ID.into()
                } else {
                    FLINTLOCK_PISTOL_ITEM_ID.into()
                },
                None,
            ),
            PirateArchetype::Captain => (
                CAPTAIN_CUTLASS_ITEM_ID.into(),
                Some(FLINTLOCK_PISTOL_ITEM_ID.into()),
            ),
        };
        Self {
            id,
            archetype,
            variant: variant % archetype.variant_count().max(1),
            position,
            velocity: DVec3::ZERO,
            yaw: 0.0,
            health: max_health,
            max_health,
            main_hand_item,
            off_hand_item,
            has_musket: archetype == PirateArchetype::Gunner && has_musket,
            gun_loaded: matches!(archetype, PirateArchetype::Gunner | PirateArchetype::Captain),
            enraged: false,
            leader_id: None,
            active_clip: None,
            clip_tick: 0,
            clip_duration: 0,
            barge_cooldown: 0,
            slam_cooldown: 40,
            lunge_cooldown: 20,
            pistol_cooldown: 30,
            rally_cooldown: 60,
            aim_ticks: 0,
        }
    }

    pub fn is_acting(&self) -> bool {
        self.active_clip.is_some()
    }

    pub fn play_action(&mut self, clip: PirateClip) {
        self.active_clip = Some(clip);
        self.clip_tick = 0;
        self.clip_duration = clip.length();
    }

    /// Ticks Pirate AI and active animation clip against an optional target position (`customServerAiStep` + `onActionTick`).
    pub fn tick(&mut self, target_pos: Option<DVec3>, cfg: &PirateConfig) -> Vec<PirateAiEvent> {
        let mut events = Vec::new();
        let cd_step = if self.enraged {
            cfg.captain_enrage_cooldown_factor.round().max(1.0) as i32
        } else {
            1
        };
        self.barge_cooldown = (self.barge_cooldown - cd_step).max(0);
        self.slam_cooldown = (self.slam_cooldown - cd_step).max(0);
        self.lunge_cooldown = (self.lunge_cooldown - cd_step).max(0);
        self.pistol_cooldown = (self.pistol_cooldown - cd_step).max(0);
        self.rally_cooldown = (self.rally_cooldown - cd_step).max(0);

        // Captain enrage check below 50% HP (`PirateCaptain.customServerAiStep`).
        if self.archetype == PirateArchetype::Captain
            && !self.enraged
            && self.health < self.max_health * cfg.captain_enrage_health
            && !self.is_acting()
        {
            self.enraged = true;
            self.play_action(PirateClip::Enrage);
        }

        // Advance active animation clip.
        if let Some(clip) = self.active_clip {
            self.clip_tick += 1;
            let hit = clip.hit_tick();
            let mv = clip.move_tick();

            if self.clip_tick == mv {
                if let Some(tpos) = target_pos {
                    let flat = DVec3::new(tpos.x - self.position.x, 0.0, tpos.z - self.position.z);
                    let dir = flat.normalize_or_zero();
                    match clip {
                        PirateClip::Barge => {
                            let imp = dir * 0.85 + DVec3::new(0.0, 0.12, 0.0);
                            self.velocity = imp;
                            events.push(PirateAiEvent::DashImpulse(imp));
                        }
                        PirateClip::FlinchRetreat => {
                            let imp = -dir * 0.55 + DVec3::new(0.0, 0.32, 0.0);
                            self.velocity = imp;
                            events.push(PirateAiEvent::DashImpulse(imp));
                        }
                        PirateClip::Lunge => {
                            let speed = (0.4 + flat.length() * 0.12).min(1.4);
                            let imp = dir * speed + DVec3::new(0.0, 0.18, 0.0);
                            self.velocity = imp;
                            events.push(PirateAiEvent::DashImpulse(imp));
                        }
                        _ => {}
                    }
                }
            }

            if self.clip_tick == hit {
                match clip {
                    PirateClip::Slash1 => events.push(PirateAiEvent::MeleeStrike {
                        damage: self.archetype.base_damage(cfg),
                        knockback: 0.4,
                    }),
                    PirateClip::Slash2 => events.push(PirateAiEvent::MeleeStrike {
                        damage: self.archetype.base_damage(cfg) * 1.15,
                        knockback: 0.6,
                    }),
                    PirateClip::Barge => events.push(PirateAiEvent::MeleeStrike {
                        damage: self.archetype.base_damage(cfg) * 0.6,
                        knockback: 1.5,
                    }),
                    PirateClip::FirePistol | PirateClip::FireMusket => {
                        self.gun_loaded = false;
                        let musket = clip == PirateClip::FireMusket;
                        events.push(PirateAiEvent::FireBall {
                            musket,
                            damage: if musket {
                                cfg.gunner_musket_damage
                            } else {
                                cfg.gunner_pistol_damage
                            },
                            speed: cfg.ball_speed * if musket { 1.0 } else { 0.85 },
                            armor_piercing: cfg.armor_piercing,
                        });
                    }
                    PirateClip::ReloadPistol | PirateClip::ReloadMusket => {
                        self.gun_loaded = true;
                    }
                    PirateClip::CaptainSlash => events.push(PirateAiEvent::MeleeStrike {
                        damage: cfg.captain_damage,
                        knockback: 1.0,
                    }),
                    PirateClip::Slam => events.push(PirateAiEvent::CaptainSlam {
                        center: self.position,
                        radius: cfg.captain_slam_radius,
                        damage: cfg.captain_slam_damage,
                    }),
                    PirateClip::Lunge => events.push(PirateAiEvent::MeleeStrike {
                        damage: cfg.captain_damage * 1.3,
                        knockback: 1.6,
                    }),
                    PirateClip::Quickdraw => events.push(PirateAiEvent::FireBall {
                        musket: false,
                        damage: cfg.captain_pistol_damage,
                        speed: cfg.ball_speed * 0.85,
                        armor_piercing: cfg.armor_piercing,
                    }),
                    PirateClip::Rally => events.push(PirateAiEvent::CaptainRally {
                        radius: cfg.rally_radius,
                        duration_ticks: cfg.rally_seconds * 20,
                    }),
                    PirateClip::Enrage => events.push(PirateAiEvent::CaptainEnraged),
                    _ => {}
                }
            }

            if self.clip_tick >= self.clip_duration {
                self.active_clip = None;
                self.clip_tick = 0;
                // Enraged Captain or Deckhand combo follow-up (`onActionEnd`).
                if clip == PirateClip::CaptainSlash && self.enraged {
                    self.play_action(PirateClip::Slash2);
                }
            }
            return events;
        }

        // Goal selection when idle and a target is present.
        let Some(tpos) = target_pos else {
            return events;
        };
        let dist = self.position.distance(tpos);

        match self.archetype {
            PirateArchetype::Deckhand => {
                if dist <= 2.6 {
                    self.play_action(PirateClip::Slash1);
                } else if dist <= 6.5 && self.barge_cooldown == 0 {
                    self.barge_cooldown = cfg.deckhand_barge_cooldown;
                    self.play_action(PirateClip::Barge);
                }
            }
            PirateArchetype::Gunner => {
                if !self.gun_loaded {
                    self.play_action(if self.has_musket {
                        PirateClip::ReloadMusket
                    } else {
                        PirateClip::ReloadPistol
                    });
                } else if dist < 3.5 {
                    self.play_action(PirateClip::FlinchRetreat);
                } else if dist <= cfg.gunner_max_range {
                    self.aim_ticks += 1;
                    let req = if self.has_musket {
                        cfg.gunner_aim_ticks * 2
                    } else {
                        cfg.gunner_aim_ticks
                    };
                    if self.aim_ticks >= req {
                        self.aim_ticks = 0;
                        self.play_action(if self.has_musket {
                            PirateClip::FireMusket
                        } else {
                            PirateClip::FirePistol
                        });
                    }
                }
            }
            PirateArchetype::Captain => {
                if dist <= 3.8 && self.slam_cooldown == 0 {
                    self.slam_cooldown = cfg.captain_slam_cooldown;
                    self.play_action(PirateClip::Slam);
                } else if dist <= 3.0 {
                    self.play_action(PirateClip::CaptainSlash);
                } else if (4.0..=9.5).contains(&dist) && self.lunge_cooldown == 0 {
                    self.lunge_cooldown = cfg.captain_lunge_cooldown;
                    self.play_action(PirateClip::Lunge);
                } else if dist <= 16.0 && self.pistol_cooldown == 0 {
                    self.pistol_cooldown = cfg.captain_pistol_cooldown;
                    self.play_action(PirateClip::Quickdraw);
                } else if self.rally_cooldown == 0 {
                    self.rally_cooldown = cfg.captain_rally_cooldown;
                    self.play_action(PirateClip::Rally);
                }
            }
        }

        events
    }

    /// Generates canonical loot drops for this pirate archetype given a deterministic `seed`.
    pub fn roll_loot(&self, seed: u64, cfg: &PirateConfig) -> Vec<PirateLootDrop> {
        let mut drops = Vec::new();
        let unit = ((seed.wrapping_mul(6_364_136_223_846_793_005) >> 32) as f32) / (u32::MAX as f32);

        match self.archetype {
            PirateArchetype::Deckhand => {
                if unit < cfg.gear_drop_chance {
                    drops.push(PirateLootDrop {
                        item_id: CUTLASS_ITEM_ID.into(),
                        count: 1,
                    });
                }
            }
            PirateArchetype::Gunner => {
                drops.push(PirateLootDrop {
                    item_id: MUSKET_BALL_ITEM_ID.into(),
                    count: 2 + ((seed % 4) as u32),
                });
                if unit < cfg.gear_drop_chance {
                    drops.push(PirateLootDrop {
                        item_id: self.main_hand_item.clone(),
                        count: 1,
                    });
                }
            }
            PirateArchetype::Captain => {
                drops.push(PirateLootDrop {
                    item_id: GOLD_KEY_ITEM_ID.into(),
                    count: 1,
                });
                if unit < 0.25 {
                    drops.push(PirateLootDrop {
                        item_id: CAPTAIN_CUTLASS_ITEM_ID.into(),
                        count: 1,
                    });
                }
            }
        }
        drops
    }
}
