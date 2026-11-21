//! All Grand Line operator & debug commands (`/devilfruit`, `/haki`, `/ability`,
//! `/poneglyph`, `/bounty`) with parser and execution metadata.
//!
//! Mirrors `core/command/DevilFruitCommand.java`, `core/haki/HakiCommand.java`,
//! `core/poneglyph/PoneglyphCommand.java`, `core/gacha/CasketCommand.java`,
//! `core/limb/LimbTestCommand.java`, `core/physics/PhysicsCommand.java`,
//! and `mobs/pirate/PirateCommand.java`.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::core::fruit::FruitId;
use crate::core::gacha::GachaTier;
use crate::core::logia::LogiaState;

/// Kind of Poneglyph artifact handed out by `/devilfruit poneglyph give`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoneglyphGiveKind {
    Rubbings,
    Encrypted,
    Deciphered,
}

/// Pirate mob archetype spawned by `/devilfruit mob pirate <type>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PirateSpawnKind {
    Deckhand,
    Gunner,
    Captain,
}

/// All 26 Grand Line command variants across `/devilfruit`, `/haki`, `/ability`, `/poneglyph`, and `/bounty`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrandLineCommand {
    /// `/devilfruit give [target] <fruit>`
    GiveFruit {
        fruit: FruitId,
    },
    /// `/devilfruit remove [target]`
    RemoveFruit,
    /// `/devilfruit form [target] <form>`
    SetForm {
        form: String,
    },
    /// `/devilfruit setmastery [target] <1..=10>`
    SetMastery {
        level: u32,
    },
    /// `/devilfruit reset [target]`
    ResetFruit,
    /// `/devilfruit list`
    ListFruits,
    /// `/devilfruit info [target]`
    FruitInfo,
    /// `/devilfruit cast <ability> [ticks]` or `/ability cast <ability> [ticks]`
    CastAbility {
        ability: String,
        hold_ticks: i32,
    },
    /// `/devilfruit worldfruit release <fruit>`
    ReleaseWorldFruit {
        fruit: FruitId,
    },
    /// `/devilfruit logia state [target] <state> [ticks]`
    LogiaSetState {
        state: LogiaState,
        ticks: i32,
    },
    /// `/devilfruit logia counter [target] <counter> <ticks>`
    LogiaForceCounter {
        counter: String,
        ticks: i32,
    },
    /// `/devilfruit casket give [target] [count]`
    CasketGive {
        count: u32,
    },
    /// `/devilfruit casket key [target] [count]`
    CasketGiveKey {
        count: u32,
    },
    /// `/devilfruit casket roll <rolls>`
    CasketRollSimulation {
        rolls: u32,
    },
    /// `/devilfruit casket preview <tier> [fruit]`
    CasketPreview {
        tier: GachaTier,
        fruit: Option<FruitId>,
    },
    /// `/devilfruit knowledge grant [target] <id>`
    KnowledgeGrant {
        knowledge_id: String,
    },
    /// `/devilfruit knowledge revoke [target] <id>`
    KnowledgeRevoke {
        knowledge_id: String,
    },
    /// `/devilfruit knowledge list [target]`
    KnowledgeList,
    /// `/devilfruit poneglyph give [target] <rubbings|encrypted|deciphered> <id>`
    PoneglyphGive {
        kind: PoneglyphGiveKind,
        poneglyph_id: String,
    },
    /// `/devilfruit poneglyph literacy [target] <0..=5>`
    PoneglyphSetLiteracy {
        level: u32,
    },
    /// `/devilfruit mob pirate <deckhand|gunner|captain> [variant] [crew]`
    PirateSpawn {
        kind: PirateSpawnKind,
        variant: Option<u8>,
        with_crew: bool,
    },
    /// `/devilfruit mob anim <clip>`
    PiratePlayAnim {
        clip: String,
    },
    /// `/devilfruit gomu limbtest <skin> [mode]`
    GomuLimbTest {
        skin: String,
        mode: String,
    },
    /// `/haki unlock [target]`
    HakiUnlock,
    /// `/haki level [target] <1..=10>`
    HakiSetLevel {
        level: u32,
    },
    /// `/haki xp [target] <amount>`
    HakiAddXp {
        amount: f32,
    },
    /// `/haki reset [target]`
    HakiReset,
    /// `/haki info [target]`
    HakiInfo,
    /// `/bounty get` or `/bounty set <amount>` or `/bounty add <amount>`
    BountyGet,
    BountySet {
        berries: u64,
    },
    BountyAdd {
        berries: u64,
    },
}

/// Command execution output returned to caller / chat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
}

impl CommandResponse {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            message: msg.into(),
        }
    }
}

/// Command parser supporting `/devilfruit ...`, `/haki ...`, `/ability ...`, `/poneglyph ...`, and `/bounty ...`.
pub struct CommandParser;

impl CommandParser {
    fn strip_target<'a>(tokens: &'a [&'a str]) -> &'a [&'a str] {
        // If first argument starts with `@` (e.g. `@s`, `@p`, `@a`) or there is an explicit target before known argument,
        // skip it when parsing single-player or targeted command tokens.
        if let Some(&first) = tokens.first() {
            if first.starts_with('@') || first.eq_ignore_ascii_case("self") {
                return &tokens[1..];
            }
        }
        tokens
    }

    pub fn parse(input: &str) -> Result<GrandLineCommand, String> {
        let trimmed = input.trim().trim_start_matches('/');
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let Some((&root, rest)) = tokens.split_first() else {
            return Err("empty command".into());
        };

        match root.to_ascii_lowercase().as_str() {
            "devilfruit" | "df" => Self::parse_devilfruit(rest),
            "haki" => Self::parse_haki(rest),
            "ability" => Self::parse_ability(rest),
            "poneglyph" => Self::parse_poneglyph_sub(rest),
            "bounty" => Self::parse_bounty(rest),
            other => Err(format!("unknown Grand Line command root: /{other}")),
        }
    }

    fn parse_devilfruit(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit <give|remove|form|setmastery|reset|list|info|cast|worldfruit|logia|casket|knowledge|poneglyph|mob|gomu>".into());
        };
        match sub.to_ascii_lowercase().as_str() {
            "give" => {
                let args = Self::strip_target(rest);
                let fruit_str = args.last().ok_or("usage: /devilfruit give [target] <fruit>")?;
                let fruit = FruitId::from_str(fruit_str)?;
                Ok(GrandLineCommand::GiveFruit { fruit })
            }
            "remove" => Ok(GrandLineCommand::RemoveFruit),
            "form" => {
                let args = Self::strip_target(rest);
                let form = args.last().ok_or("usage: /devilfruit form [target] <form>")?;
                Ok(GrandLineCommand::SetForm {
                    form: (*form).to_string(),
                })
            }
            "setmastery" => {
                let args = Self::strip_target(rest);
                let lvl_str = args
                    .last()
                    .ok_or("usage: /devilfruit setmastery [target] <1..=10>")?;
                let level: u32 = lvl_str
                    .parse()
                    .map_err(|_| format!("invalid mastery level: {lvl_str}"))?;
                if !(1..=10).contains(&level) {
                    return Err("mastery level must be between 1 and 10".into());
                }
                Ok(GrandLineCommand::SetMastery { level })
            }
            "reset" => Ok(GrandLineCommand::ResetFruit),
            "list" => Ok(GrandLineCommand::ListFruits),
            "info" => Ok(GrandLineCommand::FruitInfo),
            "cast" => Self::parse_cast(rest),
            "worldfruit" => {
                if rest.first().copied() != Some("release") {
                    return Err("usage: /devilfruit worldfruit release <fruit>".into());
                }
                let fruit_str = rest
                    .get(1)
                    .ok_or("usage: /devilfruit worldfruit release <fruit>")?;
                let fruit = FruitId::from_str(fruit_str)?;
                Ok(GrandLineCommand::ReleaseWorldFruit { fruit })
            }
            "logia" => Self::parse_logia(rest),
            "casket" => Self::parse_casket(rest),
            "knowledge" => Self::parse_knowledge(rest),
            "poneglyph" => Self::parse_poneglyph_sub(rest),
            "mob" => Self::parse_mob(rest),
            "gomu" => {
                if rest.first().copied() != Some("limbtest") {
                    return Err("usage: /devilfruit gomu limbtest <skin> [mode]".into());
                }
                let skin = rest.get(1).copied().unwrap_or("own").to_string();
                let mode = rest.get(2).copied().unwrap_or("10").to_string();
                Ok(GrandLineCommand::GomuLimbTest { skin, mode })
            }
            other => Err(format!("unknown /devilfruit subcommand: {other}")),
        }
    }

    fn parse_cast(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let ability = tokens
            .first()
            .ok_or("usage: /devilfruit cast <ability> [ticks]")?;
        let hold_ticks = tokens
            .get(1)
            .map(|s| s.parse::<i32>().map_err(|_| format!("invalid ticks: {s}")))
            .transpose()?
            .unwrap_or(40);
        Ok(GrandLineCommand::CastAbility {
            ability: (*ability).to_string(),
            hold_ticks,
        })
    }

    fn parse_logia(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit logia <state|counter> ...".into());
        };
        let args = Self::strip_target(rest);
        match sub {
            "state" => {
                let state_str = args
                    .first()
                    .ok_or("usage: /devilfruit logia state [target] <state> [ticks]")?;
                let state = match *state_str {
                    "solid" => LogiaState::Solid,
                    "partial" => LogiaState::Partial,
                    "elemental_body" | "elemental" => LogiaState::ElementalBody,
                    "reforming" => LogiaState::Reforming,
                    "suppressed" => LogiaState::Suppressed,
                    other => return Err(format!("unknown logia state: {other}")),
                };
                let ticks = args
                    .get(1)
                    .map(|s| s.parse::<i32>().map_err(|_| format!("invalid ticks: {s}")))
                    .transpose()?
                    .unwrap_or(200);
                Ok(GrandLineCommand::LogiaSetState { state, ticks })
            }
            "counter" => {
                let counter = args
                    .first()
                    .ok_or("usage: /devilfruit logia counter [target] <counter> <ticks>")?;
                let ticks = args
                    .get(1)
                    .ok_or("usage: /devilfruit logia counter [target] <counter> <ticks>")?
                    .parse::<i32>()
                    .map_err(|_| "invalid counter ticks".to_string())?;
                Ok(GrandLineCommand::LogiaForceCounter {
                    counter: (*counter).to_string(),
                    ticks,
                })
            }
            other => Err(format!("unknown logia subcommand: {other}")),
        }
    }

    fn parse_casket(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit casket <give|key|roll|preview>".into());
        };
        let args = Self::strip_target(rest);
        match sub {
            "give" => {
                let count = args
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(1)
                    .clamp(1, 64);
                Ok(GrandLineCommand::CasketGive { count })
            }
            "key" => {
                let count = args
                    .last()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(1)
                    .clamp(1, 64);
                Ok(GrandLineCommand::CasketGiveKey { count })
            }
            "roll" => {
                let rolls: u32 = args
                    .first()
                    .ok_or("usage: /devilfruit casket roll <rolls>")?
                    .parse()
                    .map_err(|_| "invalid roll count".to_string())?;
                Ok(GrandLineCommand::CasketRollSimulation {
                    rolls: rolls.clamp(1, 1_000_000),
                })
            }
            "preview" => {
                let tier_str = args
                    .first()
                    .ok_or("usage: /devilfruit casket preview <tier> [fruit]")?;
                let tier = match tier_str.to_ascii_lowercase().as_str() {
                    "legendary" => GachaTier::Legendary,
                    "rare" => GachaTier::Rare,
                    "uncommon" => GachaTier::Uncommon,
                    "common" => GachaTier::Common,
                    other => return Err(format!("unknown gacha tier: {other}")),
                };
                let fruit = args.get(1).map(|s| FruitId::from_str(s)).transpose()?;
                Ok(GrandLineCommand::CasketPreview { tier, fruit })
            }
            other => Err(format!("unknown casket subcommand: {other}")),
        }
    }

    fn parse_knowledge(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit knowledge <grant|revoke|list>".into());
        };
        let args = Self::strip_target(rest);
        match sub {
            "grant" => {
                let id = args
                    .last()
                    .ok_or("usage: /devilfruit knowledge grant [target] <id>")?;
                Ok(GrandLineCommand::KnowledgeGrant {
                    knowledge_id: (*id).to_string(),
                })
            }
            "revoke" => {
                let id = args
                    .last()
                    .ok_or("usage: /devilfruit knowledge revoke [target] <id>")?;
                Ok(GrandLineCommand::KnowledgeRevoke {
                    knowledge_id: (*id).to_string(),
                })
            }
            "list" => Ok(GrandLineCommand::KnowledgeList),
            other => Err(format!("unknown knowledge subcommand: {other}")),
        }
    }

    fn parse_poneglyph_sub(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit poneglyph <give|literacy>".into());
        };
        let args = Self::strip_target(rest);
        match sub {
            "give" => {
                let kind_str = args
                    .first()
                    .ok_or("usage: /devilfruit poneglyph give [target] <rubbings|encrypted|deciphered> <id>")?;
                let kind = match *kind_str {
                    "rubbings" => PoneglyphGiveKind::Rubbings,
                    "encrypted" => PoneglyphGiveKind::Encrypted,
                    "deciphered" => PoneglyphGiveKind::Deciphered,
                    other => return Err(format!("unknown poneglyph item kind: {other}")),
                };
                let id = args
                    .get(1)
                    .ok_or("usage: /devilfruit poneglyph give [target] <kind> <id>")?;
                Ok(GrandLineCommand::PoneglyphGive {
                    kind,
                    poneglyph_id: (*id).to_string(),
                })
            }
            "literacy" => {
                let lvl_str = args
                    .last()
                    .ok_or("usage: /devilfruit poneglyph literacy [target] <0..=5>")?;
                let level: u32 = lvl_str
                    .parse()
                    .map_err(|_| format!("invalid literacy level: {lvl_str}"))?;
                if level > 5 {
                    return Err("literacy level must be 0..=5".into());
                }
                Ok(GrandLineCommand::PoneglyphSetLiteracy { level })
            }
            other => Err(format!("unknown poneglyph subcommand: {other}")),
        }
    }

    fn parse_mob(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /devilfruit mob <pirate|anim>".into());
        };
        match sub {
            "pirate" => {
                let kind_str = rest
                    .first()
                    .ok_or("usage: /devilfruit mob pirate <deckhand|gunner|captain> [variant] [crew]")?;
                let kind = match *kind_str {
                    "deckhand" => PirateSpawnKind::Deckhand,
                    "gunner" => PirateSpawnKind::Gunner,
                    "captain" => PirateSpawnKind::Captain,
                    other => return Err(format!("unknown pirate type: {other}")),
                };
                let variant = rest.get(1).and_then(|s| s.parse::<u8>().ok());
                let with_crew = rest.contains(&"crew");
                Ok(GrandLineCommand::PirateSpawn {
                    kind,
                    variant,
                    with_crew,
                })
            }
            "anim" => {
                let clip = rest
                    .first()
                    .ok_or("usage: /devilfruit mob anim <clip>")?;
                Ok(GrandLineCommand::PiratePlayAnim {
                    clip: (*clip).to_string(),
                })
            }
            other => Err(format!("unknown mob subcommand: {other}")),
        }
    }

    fn parse_haki(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /haki <unlock|level|xp|reset|info>".into());
        };
        let args = Self::strip_target(rest);
        match sub.to_ascii_lowercase().as_str() {
            "unlock" => Ok(GrandLineCommand::HakiUnlock),
            "level" => {
                let lvl_str = args.last().ok_or("usage: /haki level [target] <1..=10>")?;
                let level: u32 = lvl_str
                    .parse()
                    .map_err(|_| format!("invalid haki level: {lvl_str}"))?;
                if !(1..=10).contains(&level) {
                    return Err("haki level must be between 1 and 10".into());
                }
                Ok(GrandLineCommand::HakiSetLevel { level })
            }
            "xp" => {
                let amt_str = args.last().ok_or("usage: /haki xp [target] <amount>")?;
                let amount: f32 = amt_str
                    .parse()
                    .map_err(|_| format!("invalid haki xp: {amt_str}"))?;
                Ok(GrandLineCommand::HakiAddXp { amount })
            }
            "reset" => Ok(GrandLineCommand::HakiReset),
            "info" => Ok(GrandLineCommand::HakiInfo),
            other => Err(format!("unknown /haki subcommand: {other}")),
        }
    }

    fn parse_ability(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Err("usage: /ability <cast|reset>".into());
        };
        match sub {
            "cast" => Self::parse_cast(rest),
            "reset" => Ok(GrandLineCommand::ResetFruit),
            _ => Self::parse_cast(tokens),
        }
    }

    fn parse_bounty(tokens: &[&str]) -> Result<GrandLineCommand, String> {
        let Some((&sub, rest)) = tokens.split_first() else {
            return Ok(GrandLineCommand::BountyGet);
        };
        let args = Self::strip_target(rest);
        match sub {
            "get" | "info" => Ok(GrandLineCommand::BountyGet),
            "set" => {
                let berries: u64 = args
                    .last()
                    .ok_or("usage: /bounty set <amount>")?
                    .parse()
                    .map_err(|_| "invalid bounty amount".to_string())?;
                Ok(GrandLineCommand::BountySet { berries })
            }
            "add" => {
                let berries: u64 = args
                    .last()
                    .ok_or("usage: /bounty add <amount>")?
                    .parse()
                    .map_err(|_| "invalid bounty amount".to_string())?;
                Ok(GrandLineCommand::BountyAdd { berries })
            }
            other => Err(format!("unknown /bounty subcommand: {other}")),
        }
    }
}
