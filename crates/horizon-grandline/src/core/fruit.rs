//! Fruit identifiers, categories, energy rules, themes, and canonical metadata
//! for all 16 playable Devil Fruits in Grand Line 0.4.0.
//!
//! Mirrors `DevilFruits.java`, `BuiltinFruits.java`, `core/fruit/DevilFruit.java`,
//! `DevilFruitType.java`, `EnergyRules.java`, and `FruitTheme.java`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::core::gacha::GachaTier;

/// Mod namespace matching `DevilFruits.MOD_ID`.
pub const MOD_ID: &str = "devilfruits";

/// Canonical `devilfruits:<path>` identifier formatting.
pub fn df_id(path: &str) -> String {
    if path.contains(':') {
        path.to_string()
    } else {
        format!("{MOD_ID}:{path}")
    }
}

/// Exact Devil Fruit taxonomy (`com.devilfruits.core.fruit.DevilFruitType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevilFruitType {
    Paramecia,
    Zoan,
    AncientZoan,
    MythicalZoan,
    Logia,
}

impl DevilFruitType {
    /// Serialization name matching `DevilFruitType.serializedName()`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Paramecia => "paramecia",
            Self::Zoan => "zoan",
            Self::AncientZoan => "ancient_zoan",
            Self::MythicalZoan => "mythical_zoan",
            Self::Logia => "logia",
        }
    }

    /// Translation key matching `DevilFruitType.translationKey()`.
    pub const fn translation_key(self) -> &'static str {
        match self {
            Self::Paramecia => "devilfruits.type.paramecia",
            Self::Zoan => "devilfruits.type.zoan",
            Self::AncientZoan => "devilfruits.type.ancient_zoan",
            Self::MythicalZoan => "devilfruits.type.mythical_zoan",
            Self::Logia => "devilfruits.type.logia",
        }
    }

    /// Default rarity color RGB matching `DevilFruitType.color()`.
    pub const fn color(self) -> u32 {
        match self {
            Self::Paramecia => 0xD4_87_FF,
            Self::Zoan => 0x5F_D6_6B,
            Self::AncientZoan => 0xD6_9A_3B,
            Self::MythicalZoan => 0xFF_D7_5F,
            Self::Logia => 0x5F_C8_FF,
        }
    }

    /// Default gacha tier mapping (`GachaTier.of(DevilFruitType)`).
    pub const fn default_gacha_tier(self) -> GachaTier {
        match self {
            Self::MythicalZoan => GachaTier::Legendary,
            Self::Logia | Self::AncientZoan => GachaTier::Rare,
            Self::Paramecia | Self::Zoan => GachaTier::Common,
        }
    }

    pub const fn is_zoan(self) -> bool {
        matches!(self, Self::Zoan | Self::AncientZoan | Self::MythicalZoan)
    }

    pub const fn is_logia(self) -> bool {
        matches!(self, Self::Logia)
    }
}

/// Per-fruit energy pool and regen curve (`com.devilfruits.core.fruit.EnergyRules`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnergyRules {
    pub base_max: f32,
    pub per_level: f32,
    pub regen_combat: f32,
    pub regen_out_of_combat: f32,
}

impl EnergyRules {
    /// Default fallback rules (`EnergyRules.DEFAULT`).
    pub const DEFAULT: Self = Self {
        base_max: 100.0,
        per_level: 10.0,
        regen_combat: 2.0,
        regen_out_of_combat: 5.0,
    };

    pub const fn new(
        base_max: f32,
        per_level: f32,
        regen_combat: f32,
        regen_out_of_combat: f32,
    ) -> Self {
        Self {
            base_max,
            per_level,
            regen_combat,
            regen_out_of_combat,
        }
    }

    /// Exact max energy formula: `base_max + per_level * (mastery.max(1) - 1)`.
    pub fn max_energy(self, mastery: u32) -> f32 {
        let m = mastery.max(1) as f32;
        self.base_max + self.per_level * (m - 1.0)
    }

    /// Energy regenerated per server tick (20 ticks/sec).
    pub fn regen_per_tick(self, in_combat: bool) -> f32 {
        let per_second = if in_combat {
            self.regen_combat
        } else {
            self.regen_out_of_combat
        };
        per_second / 20.0
    }
}

impl Default for EnergyRules {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 6-color UI/VFX palette (`com.devilfruits.core.fruit.FruitTheme`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FruitTheme {
    pub title: u32,
    pub primary: u32,
    pub accent: u32,
    pub mastery: u32,
    pub energy_low: u32,
    pub energy_high: u32,
}

impl FruitTheme {
    pub const fn new(
        title: u32,
        primary: u32,
        accent: u32,
        mastery: u32,
        energy_low: u32,
        energy_high: u32,
    ) -> Self {
        Self {
            title,
            primary,
            accent,
            mastery,
            energy_low,
            energy_high,
        }
    }
}

/// All 16 playable Devil Fruits in Grand Line 0.4.0 (`BuiltinFruits.java`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FruitId {
    GuraGura,
    UoUoSeiryu,
    SunaSuna,
    KiloKilo,
    BomuBomu,
    DoruDoru,
    SupaSupa,
    KageKage,
    YomiYomi,
    HoroHoro,
    SukeSuke,
    GomuGomu,
    PikaPika,
    HieHie,
    MaguMagu,
    MokuMoku,
}

impl FruitId {
    /// Complete ordered list of all 16 playable Devil Fruits.
    pub const ALL: [Self; 16] = [
        Self::GuraGura,
        Self::UoUoSeiryu,
        Self::SunaSuna,
        Self::KiloKilo,
        Self::BomuBomu,
        Self::DoruDoru,
        Self::SupaSupa,
        Self::KageKage,
        Self::YomiYomi,
        Self::HoroHoro,
        Self::SukeSuke,
        Self::GomuGomu,
        Self::PikaPika,
        Self::HieHie,
        Self::MaguMagu,
        Self::MokuMoku,
    ];

    /// Path component under `devilfruits:` namespace.
    pub const fn path(self) -> &'static str {
        match self {
            Self::GuraGura => "gura_gura",
            Self::UoUoSeiryu => "uo_uo_seiryu",
            Self::SunaSuna => "suna_suna",
            Self::KiloKilo => "kilo_kilo",
            Self::BomuBomu => "bomu_bomu",
            Self::DoruDoru => "doru_doru",
            Self::SupaSupa => "supa_supa",
            Self::KageKage => "kage_kage",
            Self::YomiYomi => "yomi_yomi",
            Self::HoroHoro => "horo_horo",
            Self::SukeSuke => "suke_suke",
            Self::GomuGomu => "gomu_gomu",
            Self::PikaPika => "pika_pika",
            Self::HieHie => "hie_hie",
            Self::MaguMagu => "magu_magu",
            Self::MokuMoku => "moku_moku",
        }
    }

    /// Full `devilfruits:<path>` resource identifier.
    pub fn resource_id(self) -> String {
        format!("{MOD_ID}:{}", self.path())
    }

    /// Static canonical metadata extracted directly from each fruit's Java class.
    pub fn meta(self) -> FruitMeta {
        fruit_meta(self)
    }

    /// Returns the Logia element key if this fruit is one of the 5 Logias.
    pub const fn logia_element(self) -> Option<&'static str> {
        match self {
            Self::SunaSuna => Some("sand"),
            Self::PikaPika => Some("light"),
            Self::HieHie => Some("ice"),
            Self::MaguMagu => Some("magma"),
            Self::MokuMoku => Some("smoke"),
            _ => None,
        }
    }

    pub const fn is_logia(self) -> bool {
        self.logia_element().is_some()
    }
}

impl fmt::Display for FruitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{MOD_ID}:{}", self.path())
    }
}

impl FromStr for FruitId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let stripped = s.strip_prefix("devilfruits:").unwrap_or(s);
        match stripped {
            "gura_gura" => Ok(Self::GuraGura),
            "uo_uo_seiryu" => Ok(Self::UoUoSeiryu),
            "suna_suna" => Ok(Self::SunaSuna),
            "kilo_kilo" => Ok(Self::KiloKilo),
            "bomu_bomu" => Ok(Self::BomuBomu),
            "doru_doru" => Ok(Self::DoruDoru),
            "supa_supa" => Ok(Self::SupaSupa),
            "kage_kage" => Ok(Self::KageKage),
            "yomi_yomi" => Ok(Self::YomiYomi),
            "horo_horo" => Ok(Self::HoroHoro),
            "suke_suke" => Ok(Self::SukeSuke),
            "gomu_gomu" => Ok(Self::GomuGomu),
            "pika_pika" => Ok(Self::PikaPika),
            "hie_hie" => Ok(Self::HieHie),
            "magu_magu" => Ok(Self::MaguMagu),
            "moku_moku" => Ok(Self::MokuMoku),
            other => Err(format!("unknown devil fruit: {other}")),
        }
    }
}

/// Complete static descriptor of a Devil Fruit (`DevilFruit.java` + subclass specs).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FruitMeta {
    pub id: FruitId,
    pub display_name: &'static str,
    pub fruit_type: DevilFruitType,
    pub energy: EnergyRules,
    pub loot_weight: u32,
    pub gacha_tier: GachaTier,
    pub theme: FruitTheme,
    pub default_form: &'static str,
    pub forms: &'static [&'static str],
    pub base_abilities: &'static [&'static str],
    pub ultimate_ability: Option<&'static str>,
    pub logia_element: Option<&'static str>,
    pub has_gears: bool,
}

/// Returns canonical metadata for a given `FruitId` matching Grand Line 0.4.0 1:1.
pub fn fruit_meta(id: FruitId) -> FruitMeta {
    match id {
        FruitId::GuraGura => FruitMeta {
            id,
            display_name: "Gura Gura no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(200.0, 20.0, 2.5, 7.0),
            loot_weight: 10,
            gacha_tier: GachaTier::Uncommon,
            theme: FruitTheme::new(15_984_318, 12_578_815, 8_382_719, 16_766_282, 8_382_719, 12_580_095),
            default_form: "base",
            forms: &["base"],
            base_abilities: &[
                "kaishin",
                "gekishin",
                "kabutowari",
                "kaisho",
                "shindo",
                "quake_infusion",
                "shima_yurashi",
            ],
            ultimate_ability: Some("shima_yurashi"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::UoUoSeiryu => FruitMeta {
            id,
            display_name: "Uo Uo no Mi, Model: Seiryu",
            fruit_type: DevilFruitType::MythicalZoan,
            energy: EnergyRules::new(150.0, 15.0, 3.0, 8.0),
            loot_weight: 4,
            gacha_tier: GachaTier::Legendary,
            theme: FruitTheme::new(16_769_162, 12_576_511, 7_325_951, 16_766_282, 9_427_199, 16_771_496),
            default_form: "human",
            forms: &["human", "hybrid", "dragon"],
            base_abilities: &["hybrid_form", "dragon_form", "bolo_breath", "tatsumaki", "kaifu"],
            ultimate_ability: Some("dragon_form"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::SunaSuna => FruitMeta {
            id,
            display_name: "Suna Suna no Mi",
            fruit_type: DevilFruitType::Logia,
            energy: EnergyRules::new(140.0, 14.0, 3.0, 8.0),
            loot_weight: 4,
            gacha_tier: GachaTier::Rare,
            theme: FruitTheme::new(15_915_424, 15_918_003, 15_254_378, 16_766_282, 15_918_003, 16_771_248),
            default_form: "base",
            forms: &["base"],
            base_abilities: &[
                "desert_spada",
                "barchan",
                "desert_girasole",
                "ground_secco",
                "sables",
            ],
            ultimate_ability: Some("sables"),
            logia_element: Some("sand"),
            has_gears: false,
        },
        FruitId::KiloKilo => FruitMeta {
            id,
            display_name: "Kilo Kilo no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 15.0, 2.5, 7.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(14_472_942, 15_131_378, 12_893_912, 16_766_282, 15_131_378, 15_657_718),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["one_kilo", "press"],
            ultimate_ability: Some("press"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::BomuBomu => FruitMeta {
            id,
            display_name: "Bomu Bomu no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(140.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(15_764_076, 16_174_526, 15_037_263, 16_766_282, 16_174_526, 16_757_866),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["punch", "cannon", "shot", "self_destruct"],
            ultimate_ability: Some("self_destruct"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::DoruDoru => FruitMeta {
            id,
            display_name: "Doru Doru no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(16_774_108, 15_918_016, 15_254_138, 16_766_282, 15_918_016, 16_769_184),
            default_form: "base",
            forms: &["base", "champion"],
            base_abilities: &["tools", "atelier", "wall", "lance", "service", "champion"],
            ultimate_ability: Some("champion"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::SupaSupa => FruitMeta {
            id,
            display_name: "Supa Supa no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 12.0, 3.0, 8.0),
            loot_weight: 5,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(15_398_143, 13_624_063, 10_275_071, 16_770_976, 13_624_063, 12_573_951),
            default_form: "base",
            forms: &["base"],
            base_abilities: &[
                "spar_claw",
                "steel_body",
                "blade_skates",
                "spiral_hollow",
                "super_slash",
            ],
            ultimate_ability: Some("super_slash"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::KageKage => FruitMeta {
            id,
            display_name: "Kage Kage no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Uncommon,
            theme: FruitTheme::new(12_868_576, 15_189_234, 11_554_768, 14_708_991, 15_189_234, 14_708_991),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["brick_bat", "spear", "collection", "trap", "hand", "doppelman"],
            ultimate_ability: Some("doppelman"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::YomiYomi => FruitMeta {
            id,
            display_name: "Yomi Yomi no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Uncommon,
            theme: FruitTheme::new(8_191_802, 14_221_248, 8_191_802, 13_172_586, 14_221_248, 13_172_586),
            default_form: "base",
            forms: &["base", "human", "corpse", "skeleton", "soul", "soul_king"],
            base_abilities: &[
                "soul_projection",
                "cold_slash",
                "hanauta_sancho",
                "party_music",
                "soul_king",
            ],
            ultimate_ability: Some("soul_king"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::HoroHoro => FruitMeta {
            id,
            display_name: "Horo Horo no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(16_758_741, 16_775_163, 16_740_264, 13_219_583, 16_775_163, 16_758_741),
            default_form: "base",
            forms: &["base", "ghost"],
            base_abilities: &["negative_hollow", "mini_hollow", "astral_projection", "special_hollow"],
            ultimate_ability: Some("special_hollow"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::SukeSuke => FruitMeta {
            id,
            display_name: "Suke Suke no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(150.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Common,
            theme: FruitTheme::new(12_578_784, 14_546_376, 8_378_568, 16_777_215, 14_546_376, 12_578_784),
            default_form: "base",
            forms: &["base"],
            base_abilities: &[
                "invisibility",
                "hidden_weapon",
                "group_invisibility",
                "ship_invisibility",
            ],
            ultimate_ability: Some("ship_invisibility"),
            logia_element: None,
            has_gears: false,
        },
        FruitId::GomuGomu => FruitMeta {
            id,
            display_name: "Gomu Gomu no Mi",
            fruit_type: DevilFruitType::Paramecia,
            energy: EnergyRules::new(140.0, 10.0, 3.0, 8.0),
            loot_weight: 6,
            gacha_tier: GachaTier::Legendary,
            theme: FruitTheme::new(15_194_367, 16_773_598, 12_098_559, 16_765_803, 16_773_598, 16_165_064),
            default_form: "base",
            forms: &["base", "gear_2"],
            base_abilities: &["pistol", "bazooka", "rocket", "axe", "rifle", "gatling"],
            ultimate_ability: Some("gatling"),
            logia_element: None,
            has_gears: true,
        },
        FruitId::PikaPika => FruitMeta {
            id,
            display_name: "Pika Pika no Mi",
            fruit_type: DevilFruitType::Logia,
            energy: EnergyRules::new(100.0, 19.0, 3.0, 9.0),
            loot_weight: 3,
            gacha_tier: GachaTier::Rare,
            theme: FruitTheme::new(16_774_856, 16_773_808, 16_766_282, 16_775_392, 16_771_488, 16_776_166),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["laser", "ama_no_murakumo", "yata_no_kagami", "yasakani"],
            ultimate_ability: Some("yasakani"),
            logia_element: Some("light"),
            has_gears: false,
        },
        FruitId::HieHie => FruitMeta {
            id,
            display_name: "Hie Hie no Mi",
            fruit_type: DevilFruitType::Logia,
            energy: EnergyRules::new(100.0, 20.0, 3.0, 9.0),
            loot_weight: 3,
            gacha_tier: GachaTier::Rare,
            theme: FruitTheme::new(14_546_175, 12_576_511, 7_321_840, 15_399_167, 10_474_746, 15_924_223),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["ice_ball", "partisan", "pheasant_beak", "ice_saber", "ice_age"],
            ultimate_ability: Some("ice_age"),
            logia_element: Some("ice"),
            has_gears: false,
        },
        FruitId::MaguMagu => FruitMeta {
            id,
            display_name: "Magu Magu no Mi",
            fruit_type: DevilFruitType::Logia,
            energy: EnergyRules::new(100.0, 20.0, 3.0, 9.0),
            loot_weight: 3,
            gacha_tier: GachaTier::Rare,
            theme: FruitTheme::new(16_742_958, 16_761_466, 16_734_750, 16_769_162, 16_751_184, 16_765_562),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["dai_funka", "meigo", "inugami_guren", "ryusei_kazan"],
            ultimate_ability: Some("ryusei_kazan"),
            logia_element: Some("magma"),
            has_gears: false,
        },
        FruitId::MokuMoku => FruitMeta {
            id,
            display_name: "Moku Moku no Mi",
            fruit_type: DevilFruitType::Logia,
            energy: EnergyRules::new(110.0, 20.0, 3.0, 9.0),
            loot_weight: 4,
            gacha_tier: GachaTier::Rare,
            theme: FruitTheme::new(15_265_008, 16_054_008, 11_121_598, 16_777_215, 13_226_456, 15_790_836),
            default_form: "base",
            forms: &["base"],
            base_abilities: &["white_blow", "white_launcher", "white_snake", "white_vine", "white_out"],
            ultimate_ability: Some("white_out"),
            logia_element: Some("smoke"),
            has_gears: false,
        },
    }
}

/// Returns metadata for all 16 playable Devil Fruits.
pub fn all_fruits() -> Vec<FruitMeta> {
    FruitId::ALL.iter().copied().map(fruit_meta).collect()
}
