//! Poneglyph monuments, 9-cell rubbings, blueprint assembly, Ancient Script
//! cipher (Standard & Worldbound), Literacy progression (levels 0..=5),
//! Decipher Desk puzzle state machine, and recipe knowledge unlocking.
//!
//! Mirrors `core/poneglyph/{Poneglyphs,PoneglyphConfig,PoneglyphData,BlueprintData,
//! BlueprintAssemblyRecipe,Cipher,Literacy,Knowledge,RubbingService,PoneglyphPuzzle,
//! DecipherDeskBlockEntity,DecipherDeskMenu}.java`.

use std::collections::{BTreeMap, BTreeSet};

use horizon_world::grandline_hooks::{CraftingGateResult, check_crafting_gate};
use serde::{Deserialize, Serialize};

use crate::core::rules::PlayerUuid;

/// Maximum Poneglyph Literacy level (`Literacy.MAX_LEVEL = 5`).
pub const LITERACY_MAX_LEVEL: u32 = 5;

/// Number of carved cells on a 3×3 Poneglyph face (`0..=8`).
pub const PONEGLYPH_CELL_COUNT: usize = 9;

/// Decipher Desk menu button IDs (`DecipherDeskMenu.BUTTON_HINT = 100`, `BUTTON_REVEAL = 101`).
pub const DESK_BUTTON_HINT: i32 = 100;
pub const DESK_BUTTON_REVEAL: i32 = 101;

/// Decipher Desk status codes (`DecipherDeskMenu.STATUS_*`).
pub const DESK_STATUS_EMPTY: i32 = 0;
pub const DESK_STATUS_ACTIVE: i32 = 1;
pub const DESK_STATUS_OUTPUT_FULL: i32 = 2;
pub const DESK_STATUS_TOO_HARD: i32 = 3;
pub const DESK_STATUS_UNREADABLE: i32 = 4;

/// Decipher Desk event codes (`DecipherDeskMenu.EV_*`).
pub const DESK_EV_NONE: i32 = 0;
pub const DESK_EV_CORRECT: i32 = 1;
pub const DESK_EV_WRONG: i32 = 2;
pub const DESK_EV_FORGIVEN: i32 = 3;
pub const DESK_EV_RESET: i32 = 4;
pub const DESK_EV_SUCCESS: i32 = 5;
pub const DESK_EV_SMUDGED: i32 = 6;
pub const DESK_EV_HINT: i32 = 7;
pub const DESK_EV_REVEAL: i32 = 8;
pub const DESK_EV_LOCKED: i32 = 9;

/// Blueprint progress bitflags (`BlueprintData.Progress`).
pub const PROGRESS_HINT_USED: i32 = 1;
pub const PROGRESS_REVEAL_USED: i32 = 2;
pub const PROGRESS_FORGIVE_USED: i32 = 4;

/// Cipher mode (`PoneglyphConfig.CipherMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CipherMode {
    #[default]
    Standard,
    Worldbound,
}

/// Complete Poneglyph subsystem configuration (`com.devilfruits.core.poneglyph.PoneglyphConfig`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoneglyphConfig {
    pub rub_ticks: i32,
    pub consume_paper: bool,
    pub consume_ink: bool,
    pub cell_cooldown_seconds: i32,
    pub rub_noise_ambush: bool,
    pub ambush_radius: i32,

    pub cipher_mode: CipherMode,
    pub hard_mode: bool,
    pub rounds: usize,
    pub options: usize,
    pub max_mistakes: i32,
    pub lockout_ticks: i32,

    pub xp_base: i32,
    pub xp_growth: f64,
    pub xp_rub: i32,
    pub xp_round: i32,
    pub xp_decipher: i32,
    pub l1_fewer_decoys: usize,
    pub l3_lockout_factor: f64,
    pub l3_rub_factor: f64,

    pub gate_recipes: bool,
    pub ungated_knowledge: Vec<String>,
    pub lead_map_chance: f64,
}

impl Default for PoneglyphConfig {
    fn default() -> Self {
        Self {
            rub_ticks: 60,
            consume_paper: true,
            consume_ink: true,
            cell_cooldown_seconds: 600,
            rub_noise_ambush: false,
            ambush_radius: 24,
            cipher_mode: CipherMode::Standard,
            hard_mode: false,
            rounds: 3,
            options: 10,
            max_mistakes: 3,
            lockout_ticks: 100,
            xp_base: 40,
            xp_growth: 1.4,
            xp_rub: 3,
            xp_round: 8,
            xp_decipher: 50,
            l1_fewer_decoys: 2,
            l3_lockout_factor: 0.5,
            l3_rub_factor: 0.7,
            gate_recipes: true,
            ungated_knowledge: Vec::new(),
            lead_map_chance: 0.5,
        }
    }
}

/// Static knowledge branch definition (`PoneglyphData.Knowledge`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeDef {
    pub id: String,
    pub recipes: Vec<String>,
    pub requires: Vec<String>,
    pub icon: String,
    pub name_key: String,
}

/// Static Poneglyph monument definition (`PoneglyphData.Poneglyph`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoneglyphDef {
    pub id: String,
    pub knowledge: String,
    pub words: Vec<String>,
    pub carving_variant: i32,
    pub lore_key: String,
    pub min_literacy: u32,
    pub weight: u32,
}

/// Built-in Poneglyph & Knowledge registry (`PoneglyphData.java`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoneglyphRegistry {
    pub poneglyphs: BTreeMap<String, PoneglyphDef>,
    pub knowledge: BTreeMap<String, KnowledgeDef>,
}

impl Default for PoneglyphRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl PoneglyphRegistry {
    pub fn builtin() -> Self {
        let mut knowledge = BTreeMap::new();
        let mut poneglyphs = BTreeMap::new();

        let branches = [
            (
                "devilfruits:seastone_tools",
                vec![
                    "devilfruits:seastone_pickaxe",
                    "devilfruits:seastone_axe",
                    "devilfruits:seastone_shovel",
                    "devilfruits:seastone_hoe",
                ],
                vec![],
                0,
                vec!["stone", "ocean", "prism", "carve", "depths", "chisel", "anchor", "tide"],
            ),
            (
                "devilfruits:seastone_weapons",
                vec![
                    "devilfruits:seastone_sword",
                    "devilfruits:seastone_jitte",
                    "devilfruits:seastone_arrow",
                ],
                vec!["devilfruits:seastone_tools"],
                1,
                vec!["blade", "silence", "strike", "forged", "harbor", "marine", "quench", "ward"],
            ),
            (
                "devilfruits:seastone_armor",
                vec![
                    "devilfruits:seastone_helmet",
                    "devilfruits:seastone_chestplate",
                    "devilfruits:seastone_leggings",
                    "devilfruits:seastone_boots",
                    "devilfruits:seastone_cuffs",
                ],
                vec!["devilfruits:seastone_weapons"],
                2,
                vec!["shackle", "mantle", "bastion", "chain", "weight", "aegis", "subdue", "vow"],
            ),
        ];

        for (id, recipes, reqs, min_lit, words) in branches {
            knowledge.insert(
                id.to_string(),
                KnowledgeDef {
                    id: id.to_string(),
                    recipes: recipes.into_iter().map(str::to_string).collect(),
                    requires: reqs.into_iter().map(str::to_string).collect(),
                    icon: "minecraft:paper".into(),
                    name_key: format!("knowledge.{}", id.replace(':', ".")),
                },
            );
            poneglyphs.insert(
                id.to_string(),
                PoneglyphDef {
                    id: id.to_string(),
                    knowledge: id.to_string(),
                    words: words.into_iter().map(str::to_string).collect(),
                    carving_variant: min_lit as i32,
                    lore_key: format!("poneglyph.{}.lore", id.replace(':', ".")),
                    min_literacy: min_lit,
                    weight: 10,
                },
            );
        }

        Self {
            poneglyphs,
            knowledge,
        }
    }
}

/// 26-letter Ancient Script substitution cipher (`com.devilfruits.core.poneglyph.Cipher`).
pub struct Cipher;

impl Cipher {
    pub const IDENTITY: [u8; 26] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25,
    ];

    /// Deterministic world-seed permutation (`Cipher.permutation(worldSeed)`).
    pub fn permutation(world_seed: i64) -> [u8; 26] {
        let mut p = Self::IDENTITY;
        let mut state = (world_seed as u64) ^ 6_819_981_342_808_085_044u64;
        for i in (1..26).rev() {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            let j = (z as usize) % i;
            p.swap(i, j);
        }
        p
    }

    pub fn active(mode: CipherMode, world_seed: i64) -> [u8; 26] {
        match mode {
            CipherMode::Standard => Self::IDENTITY,
            CipherMode::Worldbound => Self::permutation(world_seed),
        }
    }

    pub fn encode(text: &str, perm: &[u8; 26]) -> String {
        text.chars()
            .map(|ch| {
                let lower = ch.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    let idx = (lower as u8 - b'a') as usize;
                    (b'a' + perm[idx]) as char
                } else {
                    ch
                }
            })
            .collect()
    }

    pub fn is_bijective(perm: &[u8]) -> bool {
        if perm.len() != 26 {
            return false;
        }
        let mut seen = [false; 26];
        for &v in perm {
            if v >= 26 || seen[v as usize] {
                return false;
            }
            seen[v as usize] = true;
        }
        true
    }
}

/// Poneglyph Literacy progression & perks (`com.devilfruits.core.poneglyph.Literacy`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoneglyphLiteracy {
    pub level: u32,
    pub xp: i32,
}

impl PoneglyphLiteracy {
    /// Exact cumulative XP required for `level` (`0..=5`):
    /// `sum_{n=0}^{level-1} xp_base * xp_growth^n`.
    pub fn xp_for(level: u32, cfg: &PoneglyphConfig) -> i32 {
        let mut total = 0.0f64;
        for n in 0..level.min(LITERACY_MAX_LEVEL) {
            total += (cfg.xp_base as f64) * cfg.xp_growth.powi(n as i32);
        }
        total.round() as i32
    }

    pub fn level_for(xp: i32, cfg: &PoneglyphConfig) -> u32 {
        let mut lvl = 0u32;
        while lvl < LITERACY_MAX_LEVEL && xp >= Self::xp_for(lvl + 1, cfg) {
            lvl += 1;
        }
        lvl
    }

    pub fn add_xp(&mut self, amount: i32, cfg: &PoneglyphConfig) -> u32 {
        if amount <= 0 {
            return 0;
        }
        let old = self.level;
        self.xp += amount;
        self.level = self.level.max(Self::level_for(self.xp, cfg));
        self.level.saturating_sub(old)
    }

    pub fn set_level(&mut self, level: u32, cfg: &PoneglyphConfig) {
        let clamped = level.min(LITERACY_MAX_LEVEL);
        self.level = clamped;
        self.xp = Self::xp_for(clamped, cfg);
    }

    pub fn decoy_reduction(self, cfg: &PoneglyphConfig) -> usize {
        if self.level >= 1 {
            cfg.l1_fewer_decoys
        } else {
            0
        }
    }

    pub const fn has_hint(self) -> bool {
        self.level >= 2
    }

    pub fn lockout_factor(self, cfg: &PoneglyphConfig) -> f64 {
        if self.level >= 3 {
            cfg.l3_lockout_factor
        } else {
            1.0
        }
    }

    pub fn rub_factor(self, cfg: &PoneglyphConfig) -> f64 {
        if self.level >= 3 {
            cfg.l3_rub_factor
        } else {
            1.0
        }
    }

    pub const fn can_forgive(self) -> bool {
        self.level >= 4
    }

    pub const fn has_reveal(self) -> bool {
        self.level >= 5
    }
}

/// Per-player learned Poneglyph knowledge (`PoneglyphPlayerData.PlayerKnowledge` & `Knowledge.java`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerKnowledge {
    pub learned: BTreeSet<String>,
}

/// Outcome of learning a knowledge branch (`Knowledge.Result`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLearnResult {
    Learned,
    Already,
    Unknown,
}

impl PlayerKnowledge {
    pub fn knows(&self, id: &str) -> bool {
        let key = crate::core::fruit::df_id(id);
        self.learned.contains(&key)
    }

    /// Learns `id` and recursively learns all prerequisite branches in `def.requires` (`Knowledge.learn`).
    pub fn learn(&mut self, id: &str, registry: &PoneglyphRegistry) -> KnowledgeLearnResult {
        let key = crate::core::fruit::df_id(id);
        let Some(def) = registry.knowledge.get(&key) else {
            return KnowledgeLearnResult::Unknown;
        };
        if self.learned.contains(&key) {
            return KnowledgeLearnResult::Already;
        }
        self.learned.insert(key);
        let reqs = def.requires.clone();
        for req in reqs {
            self.learn(&req, registry);
        }
        KnowledgeLearnResult::Learned
    }

    pub fn revoke(&mut self, id: &str) -> bool {
        let key = crate::core::fruit::df_id(id);
        self.learned.remove(&key)
    }

    /// Evaluates the crafting gate for a locked recipe (`Knowledge.allows` via `horizon-world`).
    pub fn allows_recipe(
        &self,
        has_player: bool,
        recipe_knowledge: Option<&str>,
        cfg: &PoneglyphConfig,
    ) -> CraftingGateResult {
        let Some(req) = recipe_knowledge else {
            return CraftingGateResult::Allowed;
        };
        let key = crate::core::fruit::df_id(req);
        let ungated = cfg.ungated_knowledge.iter().map(String::as_str);
        let knows_fn = |id: &str| self.knows(id);
        let player_cb: Option<&dyn Fn(&str) -> bool> = if has_player {
            Some(&knows_fn)
        } else {
            None
        };
        check_crafting_gate(Some(key.as_str()), cfg.gate_recipes, ungated, player_cb)
    }
}

/// Single Poneglyph rubbing fragment (`BlueprintData.Rubbing`, cell `0..=8`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RubbingFragment {
    pub poneglyph: String,
    pub cell: u8,
}

impl RubbingFragment {
    /// Assembles 9 unique cells (`0..=8`) of the same Poneglyph into an [`EncryptedBlueprint`]
    /// (`BlueprintAssemblyRecipe.java`).
    pub fn assemble_blueprint(
        fragments: &[Self],
        seed: i64,
    ) -> Result<EncryptedBlueprint, &'static str> {
        if fragments.len() != PONEGLYPH_CELL_COUNT {
            return Err("requires all 9 poneglyph cells (0..=8)");
        }
        let first_id = &fragments[0].poneglyph;
        let mut mask = 0u16;
        for frag in fragments {
            if &frag.poneglyph != first_id {
                return Err("all 9 rubbings must come from the same poneglyph");
            }
            if frag.cell >= 9 {
                return Err("cell index out of range 0..=8");
            }
            mask |= 1u16 << frag.cell;
        }
        if mask != 0b1_1111_1111 {
            return Err("duplicate or missing cell in 3x3 rubbing set");
        }
        Ok(EncryptedBlueprint {
            poneglyph: first_id.clone(),
            seed,
            progress: BlueprintProgress::default(),
        })
    }
}

/// Progress state stored on an [`EncryptedBlueprint`] (`BlueprintData.Progress`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintProgress {
    pub owner: i32,
    pub draw: i32,
    pub round: usize,
    pub mistakes: i32,
    pub lock_until: i64,
    pub flags: i32,
    pub shuffle: i32,
}

impl BlueprintProgress {
    pub const fn has_flag(self, flag: i32) -> bool {
        (self.flags & flag) != 0
    }

    pub const fn with_flag(mut self, flag: i32) -> Self {
        self.flags |= flag;
        self
    }
}

/// Encrypted Blueprint item component (`BlueprintData.Encrypted`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBlueprint {
    pub poneglyph: String,
    pub seed: i64,
    pub progress: BlueprintProgress,
}

/// Deciphered Blueprint item component (`BlueprintData.Deciphered`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecipheredBlueprint {
    pub knowledge: String,
    pub poneglyph: String,
}

/// Single puzzle round generated on the Decipher Desk (`PoneglyphPuzzle.Round`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PuzzleRound {
    pub clue: String,
    pub options: Vec<String>,
    pub correct: usize,
}

/// Outcome of picking an option on the Decipher Desk (`PoneglyphPuzzle.Outcome`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleOutcome {
    Locked,
    Correct,
    Solved,
    Wrong,
    Forgiven,
    Reset,
    Destroyed,
}

/// Deterministic puzzle generator and step evaluator (`com.devilfruits.core.poneglyph.PoneglyphPuzzle`).
pub struct PoneglyphPuzzle;

impl PoneglyphPuzzle {
    /// Exact SplitMix64 mixing function from `PoneglyphPuzzle.mix(seed, player, draw)`.
    pub const fn mix(seed: i64, player: i64, draw: i32) -> i64 {
        let mut z = (seed as u64)
            ^ (player as u64).wrapping_mul(11_400_714_819_323_198_485u64)
            ^ ((draw as i64 as u64).wrapping_mul(14_029_467_366_897_019_727u64));
        z = (z ^ (z >> 30)).wrapping_mul(13_787_848_793_156_543_929u64);
        z = (z ^ (z >> 27)).wrapping_mul(10_723_151_780_598_845_931u64);
        (z ^ (z >> 31)) as i64
    }

    /// Generates `rounds` deterministic puzzle rounds with decoys (`PoneglyphPuzzle.generate`).
    pub fn generate(
        seed: i64,
        player_key: i64,
        draw: i32,
        words: &[String],
        other_words: &[String],
        rounds: usize,
        option_count: usize,
    ) -> Vec<PuzzleRound> {
        let mut rng = Self::mix(seed, player_key, draw) as u64;
        let mut next_u32 = || -> usize {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (rng >> 32) as usize
        };

        let mut clues = words.to_vec();
        if clues.is_empty() {
            clues.push("stone".into());
        }
        for i in (1..clues.len()).rev() {
            let j = next_u32() % (i + 1);
            clues.swap(i, j);
        }

        let mut out = Vec::with_capacity(rounds);
        let needed_decoys = option_count.saturating_sub(1).max(1);
        for r in 0..rounds {
            let clue = clues[r % clues.len()].clone();
            let mut options = Vec::with_capacity(needed_decoys + 1);

            // Adjacent character swaps
            let chars: Vec<char> = clue.chars().collect();
            for i in 0..chars.len().saturating_sub(1) {
                let mut c = chars.clone();
                c.swap(i, i + 1);
                let s: String = c.into_iter().collect();
                if s != clue && !options.contains(&s) && options.len() < needed_decoys {
                    options.push(s);
                }
            }

            // Same-length & other words
            for w in other_words.iter().chain(words.iter()) {
                if options.len() >= needed_decoys {
                    break;
                }
                if w != &clue && !options.contains(w) {
                    options.push(w.clone());
                }
            }

            // Synthetic letter mutations if still short
            let alphabet = b"aeiourstnlcdmpbg";
            let mut guard = 0usize;
            while options.len() < needed_decoys && guard < 128 {
                guard += 1;
                let mut c = chars.clone();
                if !c.is_empty() {
                    let pos = next_u32() % c.len();
                    let ch = alphabet[next_u32() % alphabet.len()] as char;
                    c[pos] = ch;
                }
                let s: String = c.into_iter().collect();
                if s != clue && !options.contains(&s) {
                    options.push(s);
                }
            }

            options.push(clue.clone());
            for i in (1..options.len()).rev() {
                let j = next_u32() % (i + 1);
                options.swap(i, j);
            }
            let correct = options.iter().position(|w| w == &clue).unwrap_or(0);
            out.push(PuzzleRound {
                clue,
                options,
                correct,
            });
        }
        out
    }

    /// Evaluates a player's word choice in the Decipher Desk (`PoneglyphPuzzle.pick`).
    #[allow(clippy::too_many_arguments)]
    pub fn pick(
        p: BlueprintProgress,
        right: bool,
        rounds: usize,
        now: i64,
        lock_ticks: i32,
        max_mistakes: i32,
        can_forgive: bool,
        hard_mode: bool,
    ) -> (PuzzleOutcome, BlueprintProgress) {
        if now < p.lock_until {
            return (PuzzleOutcome::Locked, p);
        }
        if right {
            let next_round = p.round + 1;
            let next = BlueprintProgress {
                owner: p.owner,
                draw: p.draw,
                round: next_round,
                mistakes: p.mistakes,
                lock_until: 0,
                flags: p.flags,
                shuffle: 0,
            };
            let outcome = if next_round >= rounds {
                PuzzleOutcome::Solved
            } else {
                PuzzleOutcome::Correct
            };
            return (outcome, next);
        }
        if can_forgive && !p.has_flag(PROGRESS_FORGIVE_USED) {
            return (
                PuzzleOutcome::Forgiven,
                BlueprintProgress {
                    owner: p.owner,
                    draw: p.draw,
                    round: p.round,
                    mistakes: p.mistakes,
                    lock_until: 0,
                    flags: p.flags | PROGRESS_FORGIVE_USED,
                    shuffle: p.shuffle + 1,
                },
            );
        }
        let mistakes = p.mistakes + 1;
        if mistakes >= max_mistakes {
            if hard_mode {
                (PuzzleOutcome::Destroyed, p)
            } else {
                (
                    PuzzleOutcome::Reset,
                    BlueprintProgress {
                        owner: p.owner,
                        draw: p.draw + 1,
                        round: 0,
                        mistakes: 0,
                        lock_until: now + lock_ticks as i64,
                        flags: 0,
                        shuffle: 0,
                    },
                )
            }
        } else {
            (
                PuzzleOutcome::Wrong,
                BlueprintProgress {
                    owner: p.owner,
                    draw: p.draw,
                    round: p.round,
                    mistakes,
                    lock_until: now + lock_ticks as i64,
                    flags: p.flags,
                    shuffle: p.shuffle + 1,
                },
            )
        }
    }
}

/// Active rubbing session & per-cell cooldowns (`RubbingService.java` & `PoneglyphPlayerData.RubbingProgress`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRubbingState {
    pub cell_ready_at: BTreeMap<(String, u8), i64>,
    pub active_pos: Option<[i32; 3]>,
    pub active_poneglyph: Option<String>,
    pub active_cell: u8,
    pub progress_ticks: i32,
    pub duration_ticks: i32,
    pub last_heartbeat_tick: i64,
}

impl PlayerRubbingState {
    pub fn is_cell_on_cooldown(&self, poneglyph: &str, cell: u8, now: i64) -> bool {
        self.cell_ready_at
            .get(&(poneglyph.to_string(), cell))
            .is_some_and(|&ready| now < ready)
    }

    /// Starts or advances a rubbing heartbeat (`RubbingService.heartbeat` + `tick`).
    #[allow(clippy::too_many_arguments)]
    pub fn step_rubbing(
        &mut self,
        pos: [i32; 3],
        poneglyph_id: &str,
        cell: u8,
        literacy: &mut PoneglyphLiteracy,
        knowledge: &PlayerKnowledge,
        registry: &PoneglyphRegistry,
        creative: bool,
        now: i64,
        cfg: &PoneglyphConfig,
    ) -> Result<Option<RubbingFragment>, &'static str> {
        let key = crate::core::fruit::df_id(poneglyph_id);
        let Some(def) = registry.poneglyphs.get(&key) else {
            return Err("unreadable poneglyph");
        };
        if knowledge.knows(&def.knowledge) {
            return Err("knowledge already learned");
        }
        if literacy.level < def.min_literacy {
            return Err("poneglyph script too ancient for current literacy");
        }
        if !creative && self.is_cell_on_cooldown(&key, cell, now) {
            return Err("cell on personal cooldown");
        }

        let duration = ((cfg.rub_ticks as f64) * literacy.rub_factor(cfg))
            .round()
            .max(5.0) as i32;

        if self.active_pos != Some(pos) || self.active_poneglyph.as_deref() != Some(&key) {
            self.active_pos = Some(pos);
            self.active_poneglyph = Some(key.clone());
            self.active_cell = cell;
            self.progress_ticks = 0;
            self.duration_ticks = duration;
        }
        self.last_heartbeat_tick = now;
        self.progress_ticks += 1;

        if self.progress_ticks >= self.duration_ticks {
            self.active_pos = None;
            self.active_poneglyph = None;
            self.progress_ticks = 0;
            let cooldown_ticks = (cfg.cell_cooldown_seconds.max(0) as i64) * 20;
            self.cell_ready_at.insert((key.clone(), cell), now + cooldown_ticks);
            literacy.add_xp(cfg.xp_rub, cfg);
            Ok(Some(RubbingFragment {
                poneglyph: key,
                cell,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Interactive Decipher Desk container state machine (`DecipherDeskMenu.java` + `DecipherDeskBlockEntity.java`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecipherDeskSession {
    pub container_id: i32,
    pub reader: Option<PlayerUuid>,
    pub input: Option<EncryptedBlueprint>,
    pub has_primer: bool,
    pub output: Option<DecipheredBlueprint>,
    pub rounds: Vec<PuzzleRound>,
    pub last_event: i32,
    pub last_event_index: i32,
}

impl DecipherDeskSession {
    pub fn status(&self, literacy: PoneglyphLiteracy, registry: &PoneglyphRegistry) -> i32 {
        let Some(enc) = &self.input else {
            return DESK_STATUS_EMPTY;
        };
        let Some(def) = registry.poneglyphs.get(&enc.poneglyph) else {
            return DESK_STATUS_UNREADABLE;
        };
        if self.output.is_some() {
            return DESK_STATUS_OUTPUT_FULL;
        }
        if literacy.level < def.min_literacy {
            return DESK_STATUS_TOO_HARD;
        }
        DESK_STATUS_ACTIVE
    }

    pub fn ensure_rounds(
        &mut self,
        player_key: i64,
        literacy: PoneglyphLiteracy,
        registry: &PoneglyphRegistry,
        cfg: &PoneglyphConfig,
    ) {
        let Some(enc) = &self.input else {
            self.rounds.clear();
            return;
        };
        let Some(def) = registry.poneglyphs.get(&enc.poneglyph) else {
            self.rounds.clear();
            return;
        };
        if self.rounds.is_empty() {
            let options = cfg
                .options
                .saturating_sub(literacy.decoy_reduction(cfg))
                .clamp(3, 12);
            let mut others = Vec::new();
            for (k, p) in &registry.poneglyphs {
                if k != &enc.poneglyph {
                    others.extend(p.words.iter().cloned());
                }
            }
            self.rounds = PoneglyphPuzzle::generate(
                enc.seed,
                player_key,
                enc.progress.draw,
                &def.words,
                &others,
                cfg.rounds,
                options,
            );
        }
    }

    /// Handles a button click on the Decipher Desk (`0..options` word pick, `100` Hint, `101` Reveal).
    pub fn click_button(
        &mut self,
        button_id: i32,
        player_key: i64,
        literacy: &mut PoneglyphLiteracy,
        registry: &PoneglyphRegistry,
        now: i64,
        cfg: &PoneglyphConfig,
    ) -> Option<PuzzleOutcome> {
        if self.status(*literacy, registry) != DESK_STATUS_ACTIVE {
            return None;
        }
        self.ensure_rounds(player_key, *literacy, registry, cfg);
        let enc = self.input.as_mut()?;
        let def = registry.poneglyphs.get(&enc.poneglyph)?;
        let round_idx = enc.progress.round.min(self.rounds.len().saturating_sub(1));
        let round = self.rounds.get(round_idx)?.clone();

        if button_id == DESK_BUTTON_HINT {
            if literacy.has_hint() && !enc.progress.has_flag(PROGRESS_HINT_USED) {
                enc.progress = enc.progress.with_flag(PROGRESS_HINT_USED);
                self.last_event = DESK_EV_HINT;
                self.last_event_index = -1;
            }
            return None;
        }
        if button_id == DESK_BUTTON_REVEAL {
            if literacy.has_reveal() && !enc.progress.has_flag(PROGRESS_REVEAL_USED) {
                enc.progress = enc.progress.with_flag(PROGRESS_REVEAL_USED);
                self.last_event = DESK_EV_REVEAL;
                self.last_event_index = round.correct as i32;
            }
            return None;
        }
        if button_id < 0 || (button_id as usize) >= round.options.len() {
            return None;
        }

        let lock_ticks = ((cfg.lockout_ticks as f64) * literacy.lockout_factor(cfg)).round() as i32;
        let is_right = (button_id as usize) == round.correct;
        let (outcome, next_prog) = PoneglyphPuzzle::pick(
            enc.progress,
            is_right,
            self.rounds.len(),
            now,
            lock_ticks,
            cfg.max_mistakes,
            literacy.can_forgive(),
            cfg.hard_mode,
        );

        match outcome {
            PuzzleOutcome::Locked => {
                self.last_event = DESK_EV_LOCKED;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Correct => {
                enc.progress = next_prog;
                literacy.add_xp(cfg.xp_round, cfg);
                self.last_event = DESK_EV_CORRECT;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Solved => {
                let poneglyph_id = enc.poneglyph.clone();
                let knowledge_id = def.knowledge.clone();
                self.input = None;
                self.output = Some(DecipheredBlueprint {
                    knowledge: knowledge_id,
                    poneglyph: poneglyph_id,
                });
                self.rounds.clear();
                literacy.add_xp(cfg.xp_round + cfg.xp_decipher, cfg);
                self.last_event = DESK_EV_SUCCESS;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Wrong => {
                enc.progress = next_prog;
                self.last_event = DESK_EV_WRONG;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Forgiven => {
                enc.progress = next_prog;
                self.last_event = DESK_EV_FORGIVEN;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Reset => {
                enc.progress = next_prog;
                self.rounds.clear();
                self.last_event = DESK_EV_RESET;
                self.last_event_index = button_id;
            }
            PuzzleOutcome::Destroyed => {
                self.input = None;
                self.rounds.clear();
                self.last_event = DESK_EV_SMUDGED;
                self.last_event_index = button_id;
            }
        }
        Some(outcome)
    }
}
