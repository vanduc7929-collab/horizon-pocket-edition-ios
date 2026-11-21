//! All 30 Grand Line damage types, knockback & hitstop, stun/exhaustion/restraint
//! state machines, projectile/beam/cone/capsule AoE targeting, and Haki/Logia
//! damage routing.
//!
//! Mirrors `registry/DFDamageTypes.java`, `datagen/DFTagProviders.java`,
//! `core/combat/{AbilityProjectiles,Exhaustion,Interrupt,Restraint,Stun,WeaponStrike}.java`,
//! and `core/util/Targeting.java`.

use std::collections::BTreeMap;

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::core::haki::{HakiCombat, HakiConfig, HakiHitResult, PlayerHakiData};
use crate::core::logia::{
    DamageCategory, LogiaBypassResult, LogiaConfig, LogiaDamageOutcome, LogiaData, LogiaManager,
    LogiaProfile,
};
use crate::core::rules::{CoreConfig, Mastery, PlayerFruitData, PlayerUuid};
use crate::core::seastone::{SeastoneCombat, SeastoneCombatConfig, SeastoneHandcuffs};

/// All 30 canonical Grand Line 0.4.0 damage types (`DFDamageTypes.java` + all 16 fruit modules).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrandLineDamageType {
    /// `devilfruits:devil_fruit`
    DevilFruit,
    /// `devilfruits:second_fruit`
    SecondFruit,
    /// `devilfruits:haki_ryuo`
    HakiRyuo,
    /// `devilfruits:bomu_bomu/explosion`
    BomuExplosion,
    /// `devilfruits:doru_doru/wax`
    DoruWax,
    /// `devilfruits:gura_gura/quake`
    GuraQuake,
    /// `devilfruits:hie_hie/ice`
    HieIce,
    /// `devilfruits:hie_hie/frost`
    HieFrost,
    /// `devilfruits:hie_hie/shatter`
    HieShatter,
    /// `devilfruits:horo_horo/hollow`
    HoroHollow,
    /// `devilfruits:horo_horo/blast`
    HoroBlast,
    /// `devilfruits:kage_kage/shadow`
    KageShadow,
    /// `devilfruits:kilo_kilo/press`
    KiloPress,
    /// `devilfruits:magu_magu/magma`
    MaguMagma,
    /// `devilfruits:magu_magu/scald`
    MaguScald,
    /// `devilfruits:magu_magu/eruption`
    MaguEruption,
    /// `devilfruits:magu_magu/meigo`
    MaguMeigo,
    /// `devilfruits:moku_moku/smoke`
    MokuSmoke,
    /// `devilfruits:moku_moku/choke`
    MokuChoke,
    /// `devilfruits:pika_pika/light`
    PikaLight,
    /// `devilfruits:suna_suna/sand`
    SunaSand,
    /// `devilfruits:suna_suna/dehydration`
    SunaDehydration,
    /// `devilfruits:suna_suna/quicksand`
    SunaQuicksand,
    /// `devilfruits:supa_supa/blade`
    SupaBlade,
    /// `devilfruits:uo_uo_seiryu/thunder`
    SeiryuThunder,
    /// `devilfruits:uo_uo_seiryu/flame`
    SeiryuFlame,
    /// `devilfruits:uo_uo_seiryu/dragon`
    SeiryuDragon,
    /// `devilfruits:yomi_yomi/soul`
    YomiSoul,
    /// `devilfruits:yomi_yomi/soul_king_roar`
    YomiSoulKingRoar,
    /// `devilfruits:yomi_yomi/hellfire_cold`
    YomiHellfireCold,
}

impl GrandLineDamageType {
    /// Complete array of all 30 Grand Line damage types.
    pub const ALL: [Self; 30] = [
        Self::DevilFruit,
        Self::SecondFruit,
        Self::HakiRyuo,
        Self::BomuExplosion,
        Self::DoruWax,
        Self::GuraQuake,
        Self::HieIce,
        Self::HieFrost,
        Self::HieShatter,
        Self::HoroHollow,
        Self::HoroBlast,
        Self::KageShadow,
        Self::KiloPress,
        Self::MaguMagma,
        Self::MaguScald,
        Self::MaguEruption,
        Self::MaguMeigo,
        Self::MokuSmoke,
        Self::MokuChoke,
        Self::PikaLight,
        Self::SunaSand,
        Self::SunaDehydration,
        Self::SunaQuicksand,
        Self::SupaBlade,
        Self::SeiryuThunder,
        Self::SeiryuFlame,
        Self::SeiryuDragon,
        Self::YomiSoul,
        Self::YomiSoulKingRoar,
        Self::YomiHellfireCold,
    ];

    /// Exact resource key (`devilfruits:<path>`).
    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::DevilFruit => "devilfruits:devil_fruit",
            Self::SecondFruit => "devilfruits:second_fruit",
            Self::HakiRyuo => "devilfruits:haki_ryuo",
            Self::BomuExplosion => "devilfruits:bomu_bomu/explosion",
            Self::DoruWax => "devilfruits:doru_doru/wax",
            Self::GuraQuake => "devilfruits:gura_gura/quake",
            Self::HieIce => "devilfruits:hie_hie/ice",
            Self::HieFrost => "devilfruits:hie_hie/frost",
            Self::HieShatter => "devilfruits:hie_hie/shatter",
            Self::HoroHollow => "devilfruits:horo_horo/hollow",
            Self::HoroBlast => "devilfruits:horo_horo/blast",
            Self::KageShadow => "devilfruits:kage_kage/shadow",
            Self::KiloPress => "devilfruits:kilo_kilo/press",
            Self::MaguMagma => "devilfruits:magu_magu/magma",
            Self::MaguScald => "devilfruits:magu_magu/scald",
            Self::MaguEruption => "devilfruits:magu_magu/eruption",
            Self::MaguMeigo => "devilfruits:magu_magu/meigo",
            Self::MokuSmoke => "devilfruits:moku_moku/smoke",
            Self::MokuChoke => "devilfruits:moku_moku/choke",
            Self::PikaLight => "devilfruits:pika_pika/light",
            Self::SunaSand => "devilfruits:suna_suna/sand",
            Self::SunaDehydration => "devilfruits:suna_suna/dehydration",
            Self::SunaQuicksand => "devilfruits:suna_suna/quicksand",
            Self::SupaBlade => "devilfruits:supa_supa/blade",
            Self::SeiryuThunder => "devilfruits:uo_uo_seiryu/thunder",
            Self::SeiryuFlame => "devilfruits:uo_uo_seiryu/flame",
            Self::SeiryuDragon => "devilfruits:uo_uo_seiryu/dragon",
            Self::YomiSoul => "devilfruits:yomi_yomi/soul",
            Self::YomiSoulKingRoar => "devilfruits:yomi_yomi/soul_king_roar",
            Self::YomiHellfireCold => "devilfruits:yomi_yomi/hellfire_cold",
        }
    }

    /// True if tagged `#devilfruits:devil_fruit_attacks`.
    pub const fn is_devil_fruit_attack(self) -> bool {
        !matches!(self, Self::SecondFruit | Self::HakiRyuo)
    }

    /// True if tagged `#devilfruits:heat` (triggers Hie-Hie heat counter & thaws Frozen entities).
    pub const fn is_heat(self) -> bool {
        matches!(
            self,
            Self::MaguMagma
                | Self::MaguScald
                | Self::MaguEruption
                | Self::MaguMeigo
                | Self::SeiryuFlame
                | Self::BomuExplosion
        )
    }

    /// True if tagged `#minecraft:bypasses_armor`.
    pub const fn bypasses_armor(self) -> bool {
        matches!(
            self,
            Self::SecondFruit
                | Self::HakiRyuo
                | Self::SunaDehydration
                | Self::MokuChoke
                | Self::YomiSoul
                | Self::YomiSoulKingRoar
        )
    }

    /// Maps a Grand Line damage type to its [`DamageCategory`] for Logia hit-rule evaluation.
    pub const fn logia_category(self) -> DamageCategory {
        match self {
            Self::SecondFruit | Self::HakiRyuo => DamageCategory::Always,
            Self::BomuExplosion | Self::HoroBlast | Self::MaguEruption => DamageCategory::Explosion,
            Self::MaguMagma | Self::MaguScald | Self::MaguMeigo | Self::SeiryuFlame => {
                DamageCategory::Fire
            }
            Self::SeiryuThunder => DamageCategory::Lightning,
            Self::HieIce | Self::HieFrost | Self::HieShatter | Self::YomiHellfireCold => {
                DamageCategory::Freeze
            }
            Self::YomiSoul | Self::YomiSoulKingRoar | Self::HoroHollow | Self::KageShadow => {
                DamageCategory::Magic
            }
            _ => DamageCategory::Ability,
        }
    }
}

/// Reason a [`RestraintHold`] ended (`Restraint.End`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestraintEnd {
    Time = 0,
    Released = 1,
    Struggle = 2,
    Haki = 3,
    Seastone = 4,
    HolderGone = 5,
    HolderStunned = 6,
    Leash = 7,
    Death = 8,
    Gone = 9,
}

/// Restraint configuration (`Restraint.Rules`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RestraintRules {
    pub kind: String,
    pub ticks: i32,
    pub max_struggle_cut: f32,
    pub struggle_presses: f32,
    pub leash: f64,
    pub carried: bool,
    pub haki_breaks: bool,
    pub seastone_breaks: bool,
}

impl RestraintRules {
    pub fn of(kind: impl Into<String>, ticks: i32) -> Self {
        Self {
            kind: kind.into(),
            ticks,
            max_struggle_cut: 0.5,
            struggle_presses: 20.0,
            leash: 0.0,
            carried: false,
            haki_breaks: true,
            seastone_breaks: true,
        }
    }
}

/// Active restraint hold on a player/entity (`Restraint.Hold`).
#[derive(Clone, Debug, PartialEq)]
pub struct RestraintHold {
    pub holder: Option<PlayerUuid>,
    pub rules: RestraintRules,
    pub start_tick: i64,
    pub until_tick: i64,
    pub presses: f32,
    pub last_press_tick: i64,
    pub steer_target: Option<DVec3>,
}

impl RestraintHold {
    pub fn progress(&self) -> f32 {
        if self.rules.struggle_presses <= 0.0 {
            0.0
        } else {
            (self.presses / self.rules.struggle_presses).clamp(0.0, 1.0)
        }
    }

    pub fn ticks_left(&self, now: i64) -> i32 {
        (self.until_tick - now).max(0) as i32
    }

    /// Processes a struggle key press (`Restraint.struggle`): Haki coating doubles press weight (`2.0` vs `1.0`).
    pub fn struggle(&mut self, now: i64, haki_coated: bool) -> bool {
        if self.rules.max_struggle_cut <= 0.0 || self.rules.struggle_presses <= 0.0 {
            return false;
        }
        if now - self.last_press_tick < 2 && now >= self.last_press_tick {
            return false;
        }
        self.last_press_tick = now;
        let press = if haki_coated { 2.0 } else { 1.0 };
        let before = self.progress();
        self.presses += press;
        let after = self.progress();
        let cut = ((after - before) * self.rules.max_struggle_cut * (self.rules.ticks as f32))
            .round() as i64;
        self.until_tick -= cut;
        after >= 1.0 && self.rules.max_struggle_cut >= 0.999
    }
}

/// Per-player Stun, Exhaustion, Hitstop, and Restraint state (`Stun.java`, `Exhaustion.java`, `Restraint.java`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatStatus {
    pub stun_until_tick: i64,
    pub exhaustion_until_tick: i64,
    pub exhaustion_slowness: i32,
    pub exhaustion_weakness: i32,
    pub hitstop_until_tick: i64,
    pub restraint: Option<RestraintHold>,
}

impl CombatStatus {
    pub fn is_stunned(&self, now: i64) -> bool {
        now < self.stun_until_tick
    }

    pub fn is_exhausted(&self, now: i64) -> bool {
        now < self.exhaustion_until_tick
    }

    pub fn is_restrained(&self, now: i64) -> bool {
        self.restraint.as_ref().is_some_and(|r| now < r.until_tick)
    }

    /// True if the entity is blocked from casting abilities or item techniques.
    pub fn blocks_actions(&self, now: i64) -> bool {
        self.is_stunned(now) || self.is_exhausted(now) || self.is_restrained(now)
    }

    pub fn apply_stun(&mut self, now: i64, ticks: i32) {
        if ticks > 0 {
            self.stun_until_tick = self.stun_until_tick.max(now + ticks as i64);
        }
    }

    pub fn clear_stun(&mut self) {
        self.stun_until_tick = 0;
    }

    pub fn apply_exhaustion(&mut self, now: i64, ticks: i32, slowness: i32, weakness: i32) {
        if ticks > 0 {
            self.exhaustion_until_tick = self.exhaustion_until_tick.max(now + ticks as i64);
            self.exhaustion_slowness = slowness;
            self.exhaustion_weakness = weakness;
        }
    }

    pub fn apply_restraint(
        &mut self,
        holder: Option<PlayerUuid>,
        rules: RestraintRules,
        has_seastone_immunity: bool,
        now: i64,
    ) -> bool {
        if rules.ticks <= 0 || has_seastone_immunity {
            return false;
        }
        let until = now + rules.ticks as i64;
        self.restraint = Some(RestraintHold {
            holder,
            rules,
            start_tick: now,
            until_tick: until,
            presses: 0.0,
            last_press_tick: i64::MIN / 4,
            steer_target: None,
        });
        true
    }

    pub fn release_restraint(&mut self) -> Option<RestraintHold> {
        self.restraint.take()
    }

    /// Clamps velocity while stunned or restrained (`Stun.hold` / `Restraint.hold`).
    pub fn clamp_velocity(&mut self, velocity: DVec3, position: DVec3, now: i64) -> DVec3 {
        if let Some(hold) = &mut self.restraint {
            if now < hold.until_tick {
                if hold.rules.carried {
                    if let Some(spot) = hold.steer_target.take() {
                        let mut v = (spot - position) * 0.45;
                        if v.length_squared() > 2.5 * 2.5 {
                            v = v.normalize() * 2.5;
                        }
                        return v;
                    }
                }
                return DVec3::new(0.0, velocity.y.min(0.0), 0.0);
            } else {
                self.restraint = None;
            }
        }
        if self.is_stunned(now) {
            return DVec3::new(0.0, velocity.y.min(0.0), 0.0);
        }
        velocity
    }
}

/// Tracked in-flight ability projectile (`com.devilfruits.core.combat.AbilityProjectiles`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackedProjectile {
    pub id: u64,
    pub owner: PlayerUuid,
    pub position: DVec3,
    pub velocity: DVec3,
    pub radius: f64,
    pub damage: f32,
    pub damage_type: GrandLineDamageType,
    pub expires_tick: i64,
    pub broken: bool,
    pub done: bool,
}

/// Registry of active ability projectiles that can be shattered by quake/slash shockwaves (`AbilityProjectiles.java`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AbilityProjectileTracker {
    next_id: u64,
    flights: BTreeMap<u64, TrackedProjectile>,
}

impl AbilityProjectileTracker {
    pub fn spawn(
        &mut self,
        owner: PlayerUuid,
        position: DVec3,
        velocity: DVec3,
        radius: f64,
        damage: f32,
        damage_type: GrandLineDamageType,
        expires_tick: i64,
    ) -> u64 {
        self.flights.retain(|_, f| !f.done && !f.broken);
        self.next_id += 1;
        let id = self.next_id;
        self.flights.insert(
            id,
            TrackedProjectile {
                id,
                owner,
                position,
                velocity,
                radius,
                damage,
                damage_type,
                expires_tick,
                broken: false,
                done: false,
            },
        );
        id
    }

    pub fn step_all(&mut self, now: i64) {
        for flight in self.flights.values_mut() {
            if now >= flight.expires_tick {
                flight.done = true;
            } else {
                flight.position += flight.velocity;
            }
        }
        self.flights.retain(|_, f| !f.done && !f.broken);
    }

    /// Shatters all enemy projectiles whose center or edge falls inside `within` (`AbilityProjectiles.shatter`).
    pub fn shatter<F>(&mut self, by_owner: PlayerUuid, by_pos: DVec3, within: F) -> usize
    where
        F: Fn(DVec3) -> bool,
    {
        let mut count = 0usize;
        for flight in self.flights.values_mut() {
            if !flight.done && !flight.broken && flight.owner != by_owner {
                let to = by_pos - flight.position;
                let edge = if to.length_squared() < 1e-6 {
                    flight.position
                } else {
                    flight.position + to.normalize() * flight.radius
                };
                if within(flight.position) || within(edge) {
                    flight.broken = true;
                    count += 1;
                }
            }
        }
        self.flights.retain(|_, f| !f.done && !f.broken);
        count
    }

    pub fn active(&self) -> Vec<&TrackedProjectile> {
        self.flights
            .values()
            .filter(|f| !f.done && !f.broken)
            .collect()
    }
}

/// Candidate target snapshot for geometric AoE / beam / cone / capsule queries (`Targeting.java`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetCandidate {
    pub id: PlayerUuid,
    pub center: DVec3,
    pub width: f64,
    pub height: f64,
    pub knockback_resistance: f64,
}

/// Geometric targeting & knockback helpers (`com.devilfruits.core.util.Targeting` & `WeaponCombo.java`).
pub struct CombatTargeting;

impl CombatTargeting {
    /// Sphere AoE filter (`Targeting.inRadius`).
    pub fn in_radius(
        candidates: &[TargetCandidate],
        caster: PlayerUuid,
        center: DVec3,
        radius: f64,
    ) -> Vec<TargetCandidate> {
        let r2 = radius * radius;
        candidates
            .iter()
            .copied()
            .filter(|c| c.id != caster && c.center.distance_squared(center) <= r2)
            .collect()
    }

    /// Capsule / beam segment hit test (`Targeting.capsuleHits`).
    pub fn in_capsule(
        candidates: &[TargetCandidate],
        caster: PlayerUuid,
        a: DVec3,
        b: DVec3,
        radius: f64,
    ) -> Vec<TargetCandidate> {
        let ab = b - a;
        let len2 = ab.length_squared().max(1e-6);
        candidates
            .iter()
            .copied()
            .filter(|c| {
                if c.id == caster {
                    return false;
                }
                let t = ((c.center - a).dot(ab) / len2).clamp(0.0, 1.0);
                let closest = a + ab * t;
                let reach = radius + c.width * 0.5;
                closest.distance_squared(c.center) <= reach * reach
            })
            .collect()
    }

    /// Cone hit test (`Targeting.inCone`): includes point-blank `< 1.2` block check and half-angle cosine check.
    pub fn in_cone(
        candidates: &[TargetCandidate],
        caster: PlayerUuid,
        apex: DVec3,
        dir: DVec3,
        length: f64,
        half_angle_rad: f64,
    ) -> Vec<TargetCandidate> {
        let d = dir.normalize_or_zero();
        if d.length_squared() < 1e-8 {
            return Vec::new();
        }
        let cos = half_angle_rad.cos();
        candidates
            .iter()
            .copied()
            .filter(|c| {
                if c.id == caster {
                    return false;
                }
                let to = c.center - apex;
                let dist = to.length();
                if dist < 1.2 {
                    return true;
                }
                let along = to.dot(d);
                along > 0.0 && along <= length + c.width && (along / dist) >= cos
            })
            .collect()
    }

    /// Best-aimed target inside a cone matching `WeaponCombo.aimed`:
    /// minimizes `off_axis * 2.0 + along * 0.3`.
    pub fn aimed(
        candidates: &[TargetCandidate],
        caster: PlayerUuid,
        eye: DVec3,
        look: DVec3,
        reach: f64,
        half_angle_rad: f64,
    ) -> Option<TargetCandidate> {
        let d = look.normalize_or_zero();
        Self::in_cone(candidates, caster, eye, d, reach, half_angle_rad)
            .into_iter()
            .min_by(|a, b| {
                let score = |c: &TargetCandidate| {
                    let to = c.center - eye;
                    let along = to.dot(d);
                    let off = (to - d * along).length();
                    off * 2.0 + along * 0.3
                };
                score(a)
                    .partial_cmp(&score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Knockback launch velocity scaled by target's knockback resistance (`Targeting.launch`).
    pub fn launch_velocity(raw_velocity: DVec3, knockback_resistance: f64) -> DVec3 {
        let scale = (1.0 - knockback_resistance).max(0.0);
        raw_velocity * scale
    }
}

/// Complete outcome of resolving a Grand Line attack (`CombatResolver::resolve_attack`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttackResolution {
    pub final_hp_damage: f32,
    pub armor_bypass_damage: f32,
    pub haki: HakiHitResult,
    pub logia: Option<LogiaDamageOutcome>,
    pub seastone_suppress_ticks: i32,
    pub stun_ticks: i32,
    pub mastery_levels_gained: u32,
}

/// Unified combat pipeline combining Mastery, Haki, Seastone, and Logia intangibility.
pub struct CombatResolver;

impl CombatResolver {
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_attack(
        attacker_fruit: &mut PlayerFruitData,
        attacker_haki: &mut PlayerHakiData,
        defender_fruit: &mut PlayerFruitData,
        defender_haki: Option<&PlayerHakiData>,
        defender_logia: Option<&mut LogiaData>,
        defender_cuffs: &mut SeastoneHandcuffs,
        defender_combat: &mut CombatStatus,
        defender_seastone_armor_pieces: u32,
        base_damage: f32,
        damage_type: GrandLineDamageType,
        weapon_item: Option<&str>,
        is_seastone_arrow: bool,
        limb_index: usize,
        clash_rand: (f32, f32),
        now: i64,
        defender_creative: bool,
        core_cfg: &CoreConfig,
        haki_cfg: &HakiConfig,
        logia_cfg: &LogiaConfig,
        seastone_cfg: &SeastoneCombatConfig,
    ) -> AttackResolution {
        let is_ability = damage_type.is_devil_fruit_attack();
        let using_weapon = weapon_item.is_some() || is_seastone_arrow;
        let is_defender_logia = defender_fruit.fruit_id.is_some_and(|f| f.is_logia());

        // 1. Evaluate Seastone weapon / arrow hit & armor reduction.
        let ss_hit = SeastoneCombat::on_hit(
            defender_fruit,
            defender_cuffs,
            weapon_item,
            is_seastone_arrow,
            None,
            now,
            seastone_cfg,
        );
        if ss_hit.stun_ticks_applied > 0 {
            defender_combat.apply_stun(now, ss_hit.stun_ticks_applied);
        }

        let after_ss_armor = if is_ability {
            SeastoneCombat::scale_incoming_fruit_damage(
                defender_fruit.has_fruit(),
                defender_seastone_armor_pieces,
                base_damage,
                seastone_cfg,
            )
        } else {
            base_damage
        };

        // 2. Evaluate Armament Haki attack, defense, Ryuo, and Haki Clash.
        let haki_res = HakiCombat::resolve_hit(
            attacker_haki,
            defender_haki,
            after_ss_armor,
            using_weapon,
            is_ability,
            is_defender_logia,
            clash_rand,
            haki_cfg,
        );

        let bypass: Option<LogiaBypassResult> = ss_hit.logia_bypass.or(haki_res.logia_bypass);
        let attacker_elem = attacker_fruit.fruit_id.and_then(|f| f.logia_element());

        // 3. Route through defender's Logia profile if applicable.
        let mut final_hp_damage = haki_res.scaled_damage;
        let mut armor_bypass = haki_res.armor_bypass_damage;
        let mut logia_outcome = None;
        let mut extra_stun = 0;

        if let (Some(def_fruit_id), Some(logia_data)) = (defender_fruit.fruit_id, defender_logia) {
            if let Some(profile) = LogiaProfile::for_fruit(def_fruit_id) {
                let total_in = final_hp_damage + armor_bypass;
                let out = LogiaManager::evaluate_damage(
                    &profile,
                    defender_fruit,
                    logia_data,
                    damage_type.logia_category(),
                    total_in,
                    limb_index,
                    glam::Vec3::ZERO,
                    glam::Vec3::Z,
                    bypass,
                    attacker_elem,
                    now,
                    defender_creative,
                    core_cfg,
                    logia_cfg,
                );
                if out.passed_through {
                    final_hp_damage = 0.0;
                    armor_bypass = 0.0;
                } else if total_in > 0.0 {
                    let ratio = (out.final_damage / total_in).clamp(0.0, 2.0);
                    final_hp_damage *= ratio;
                    armor_bypass *= ratio;
                }
                if out.stagger_ticks > 0 {
                    defender_combat.apply_stun(now, out.stagger_ticks);
                    extra_stun = out.stagger_ticks;
                }
                logia_outcome = Some(out);
            }
        }

        // 4. Award mastery XP on successful fruit hit (`Targeting.hurt`: `xp_per_hit`).
        let mut mastery_levels_gained = 0;
        if is_ability && (final_hp_damage + armor_bypass) > 0.0 && attacker_fruit.has_fruit() {
            attacker_fruit.mark_combat(now);
            defender_fruit.mark_combat(now);
            mastery_levels_gained =
                Mastery::award_xp(attacker_fruit, core_cfg.xp_per_hit, core_cfg);
        }

        AttackResolution {
            final_hp_damage,
            armor_bypass_damage: armor_bypass,
            haki: haki_res,
            logia: logia_outcome,
            seastone_suppress_ticks: ss_hit.suppress_ticks_applied,
            stun_ticks: ss_hit.stun_ticks_applied.max(extra_stun),
            mastery_levels_gained,
        }
    }
}
