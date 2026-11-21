//! Devil Fruit module interface (`DevilFruitModule`), execution context
//! (`FruitContext`), ability result (`AbilityExecutionResult`), and central
//! `FruitRegistry` for all 16 playable Devil Fruits in Grand Line 0.4.0.
//!
//! Each of the 16 fruits (`gura_gura`, `gomu_gomu`, `bomu_bomu`, `kilo_kilo`,
//! `suke_suke`, `hie_hie`, `magu_magu`, `moku_moku`, `pika_pika`, `suna_suna`,
//! `supa_supa`, `horo_horo`, `kage_kage`, `doru_doru`, `yomi_yomi`,
//! `uo_uo_seiryu`) implements [`DevilFruitModule`] and registers into
//! [`FruitRegistry`].

use std::collections::BTreeMap;
use std::sync::Arc;

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityContext, AbilityDef, AbilityEvent, AbilityId, AbilityStats, CastBudget, CastResult,
    CastType, TechniqueHeat,
};
use crate::core::cold::ChillBurst;
use crate::core::combat::{GrandLineDamageType, TargetCandidate};
use crate::core::form::{FormRuntime, FruitFormDef};
use crate::core::fruit::{FruitId, FruitMeta};
use crate::core::gear::GearState;
use crate::core::haki::PlayerHakiData;
use crate::core::logia::{LogiaData, LogiaProfile};
use crate::core::rules::{CoreConfig, DFGameRules, PlayerFruitData, PlayerUuid};
use crate::core::world_fx::{QueuedVfxEvent, WorldFxManager};

pub mod bomu_bomu;
pub mod doru_doru;
pub mod gomu_gomu;
pub mod gura_gura;
pub mod hie_hie;
pub mod horo_horo;
pub mod kage_kage;
pub mod kilo_kilo;
pub mod magu_magu;
pub mod moku_moku;
pub mod pika_pika;
pub mod suke_suke;
pub mod suna_suna;
pub mod supa_supa;
pub mod uo_uo_seiryu;
pub mod yomi_yomi;

pub use bomu_bomu::BomuBomuModule;
pub use doru_doru::DoruDoruModule;
pub use gomu_gomu::GomuGomuModule;
pub use gura_gura::GuraGuraModule;
pub use hie_hie::HieHieModule;
pub use horo_horo::HoroHoroModule;
pub use kage_kage::KageKageModule;
pub use kilo_kilo::KiloKiloModule;
pub use magu_magu::MaguMaguModule;
pub use moku_moku::MokuMokuModule;
pub use pika_pika::PikaPikaModule;
pub use suke_suke::SukeSukeModule;
pub use suna_suna::SunaSunaModule;
pub use supa_supa::SupaSupaModule;
pub use uo_uo_seiryu::UoUoSeiryuModule;
pub use yomi_yomi::YomiYomiModule;

/// Target hit produced by an ability execution.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityHitEffect {
    pub target_id: PlayerUuid,
    pub damage: f32,
    pub damage_type: GrandLineDamageType,
    pub knockback_impulse: DVec3,
    pub stun_ticks: i32,
    pub chill_stacks_added: i32,
    pub chill_burst: Option<ChillBurst>,
    pub freeze_ticks: i32,
    pub set_fire_seconds: i32,
}

/// Output returned by [`DevilFruitModule::execute_ability`] or [`DevilFruitModule::on_ability_event`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AbilityExecutionResult {
    pub cast_result: Option<CastResult>,
    pub damage_type: Option<GrandLineDamageType>,
    pub hits: Vec<AbilityHitEffect>,
    pub caster_velocity_impulse: Option<DVec3>,
    pub form_switch_to: Option<String>,
    pub spawned_projectiles: usize,
    pub destroyed_blocks: Vec<[i32; 3]>,
    pub transformed_blocks: Vec<([i32; 3], String)>,
    pub vfx_events: Vec<QueuedVfxEvent>,
    pub animation_clip: Option<String>,
    pub screen_shake: Option<(f32, i32)>,
}

/// Mutable execution context passed to a [`DevilFruitModule`] during ability casts and ticks.
pub struct FruitContext<'a> {
    pub player_id: PlayerUuid,
    pub player: &'a mut PlayerFruitData,
    pub form_runtime: &'a mut FormRuntime,
    pub gear_state: &'a mut GearState,
    pub haki: &'a mut PlayerHakiData,
    pub logia: Option<&'a mut LogiaData>,
    pub heat: &'a mut TechniqueHeat,
    pub world_fx: &'a mut WorldFxManager,
    pub budget: CastBudget,
    pub ability_ctx: AbilityContext,
    pub nearby_targets: &'a [TargetCandidate],
    pub config: &'a CoreConfig,
    pub game_rules: DFGameRules,
    pub creative: bool,
    pub submerged_in_water: bool,
    pub suppressed: bool,
    pub on_ground: bool,
    pub in_lava: bool,
}

/// Core trait implemented by each of the 16 playable Devil Fruits (`FruitModule.java` + `DevilFruit.java`).
pub trait DevilFruitModule: Send + Sync {
    /// Canonical [`FruitId`] for this Devil Fruit.
    fn fruit_id(&self) -> FruitId;

    /// Static canonical metadata (`FruitMeta`). Defaults to `self.fruit_id().meta()`.
    fn meta(&self) -> FruitMeta {
        self.fruit_id().meta()
    }

    /// Primary damage type dealt by this fruit's techniques (`DevilFruit.damageType()`).
    fn primary_damage_type(&self) -> GrandLineDamageType {
        match self.fruit_id() {
            FruitId::GuraGura => GrandLineDamageType::GuraQuake,
            FruitId::UoUoSeiryu => GrandLineDamageType::SeiryuDragon,
            FruitId::SunaSuna => GrandLineDamageType::SunaSand,
            FruitId::KiloKilo => GrandLineDamageType::KiloPress,
            FruitId::BomuBomu => GrandLineDamageType::BomuExplosion,
            FruitId::DoruDoru => GrandLineDamageType::DoruWax,
            FruitId::SupaSupa => GrandLineDamageType::SupaBlade,
            FruitId::KageKage => GrandLineDamageType::KageShadow,
            FruitId::YomiYomi => GrandLineDamageType::YomiSoul,
            FruitId::HoroHoro => GrandLineDamageType::HoroHollow,
            FruitId::SukeSuke => GrandLineDamageType::DevilFruit,
            FruitId::GomuGomu => GrandLineDamageType::DevilFruit,
            FruitId::PikaPika => GrandLineDamageType::PikaLight,
            FruitId::HieHie => GrandLineDamageType::HieIce,
            FruitId::MaguMagu => GrandLineDamageType::MaguMagma,
            FruitId::MokuMoku => GrandLineDamageType::MokuSmoke,
        }
    }

    /// All ability definitions registered by this fruit (`DevilFruit.abilities()`).
    fn abilities(&self) -> Vec<AbilityDef>;

    /// Looks up a single ability definition by name (`"kaishin"`, `"pistol"`, etc.).
    fn ability(&self, name: &str) -> Option<AbilityDef> {
        let stripped = name
            .split(':')
            .next_back()
            .unwrap_or(name)
            .split('/')
            .next_back()
            .unwrap_or(name);
        self.abilities()
            .into_iter()
            .find(|a| a.id.name == stripped || a.id.full_key() == name)
    }

    /// Transformation forms registered by this fruit (`DevilFruit.forms()`).
    fn forms(&self) -> Vec<FruitFormDef> {
        FruitFormDef::builtin_forms(self.fruit_id())
    }

    /// Optional Logia profile if this fruit is a Logia (`DevilFruit.logia()`).
    fn logia_profile(&self) -> Option<LogiaProfile> {
        LogiaProfile::for_fruit(self.fruit_id())
    }

    /// Executes an ability activation or release (`Ability.onCast` / `onRelease` / `onComboStep`).
    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult;

    /// Per-tick passive & active fruit update (e.g. Kilo-Kilo weight, Yomi-Yomi water sprint/revive,
    /// Suke-Suke stealth aura, Hie-Hie frost walk, Magu-Magu lava swim).
    fn on_tick(&self, _ctx: &mut FruitContext<'_>) -> AbilityExecutionResult {
        AbilityExecutionResult::default()
    }

    /// Computes physics/body/stealth modifiers for `horizon-world` (`EntityBodyModifiers`).
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
        if let Some(form) = self
            .forms()
            .into_iter()
            .find(|f| f.id.name == player.active_form)
        {
            mods.passes_through_leaves = form.collision.pass_through_leaves;
            mods.fire_immune = form.collision.fire_immune;
        }
        if self.fruit_id() == FruitId::MaguMagu {
            mods.fire_immune = true;
            mods.swim_in_lava = true;
        }
        let _ = form_runtime;
        mods
    }
}

/// Default canonical fallback module for any fruit before its specialized submodule is registered.
/// Synthesizes accurate [`AbilityDef`]s from [`FruitMeta`] and executes standard scaled cone/AoE hits.
#[derive(Clone, Copy, Debug)]
pub struct BuiltinFallbackFruitModule {
    pub id: FruitId,
}

impl BuiltinFallbackFruitModule {
    pub const fn new(id: FruitId) -> Self {
        Self { id }
    }
}

impl DevilFruitModule for BuiltinFallbackFruitModule {
    fn fruit_id(&self) -> FruitId {
        self.id
    }

    fn abilities(&self) -> Vec<AbilityDef> {
        let meta = self.id.meta();
        let forms = FruitFormDef::builtin_forms(self.id);
        let mut all_names: Vec<String> = meta
            .base_abilities
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        for f in &forms {
            for ab in &f.abilities {
                if !all_names.contains(ab) {
                    all_names.push(ab.clone());
                }
            }
        }

        all_names
            .into_iter()
            .enumerate()
            .map(|(idx, name)| {
                let is_ultimate = meta.ultimate_ability == Some(name.as_str())
                    || forms
                        .iter()
                        .any(|f| f.ultimate_ability.as_deref() == Some(name.as_str()));
                let is_form_switch =
                    name.ends_with("_form") || name == "champion" || name == "soul_king";
                let cast_type = if is_form_switch {
                    CastType::FormSwitch
                } else {
                    CastType::Instant
                };
                let unlock_mastery = if is_ultimate {
                    6
                } else {
                    ((idx as u32) + 1).min(5)
                };
                let allowed_forms: Vec<String> = forms
                    .iter()
                    .filter(|f| f.abilities.contains(&name))
                    .map(|f| f.id.name.clone())
                    .collect();

                AbilityDef {
                    id: AbilityId::new(self.id, name.clone()),
                    display_name: name.replace('_', " "),
                    cast_type,
                    allowed_forms,
                    stats: AbilityStats {
                        enabled: true,
                        energy_cost: if is_ultimate { 50.0 } else { 20.0 },
                        energy_per_second: 0.0,
                        cooldown_ticks: if is_ultimate { 160 } else { 40 },
                        unlock_mastery,
                        damage: if is_ultimate { 16.0 } else { 8.0 },
                        range: if is_ultimate { 16.0 } else { 8.0 },
                        knockback: if is_ultimate { 1.2 } else { 0.5 },
                        ..AbilityStats::default()
                    },
                    is_ultimate,
                    stops_movement_while_charging: false,
                    allows_cancel_on_hurt: false,
                }
            })
            .collect()
    }

    fn execute_ability(
        &self,
        ctx: &mut FruitContext<'_>,
        event: &AbilityEvent,
    ) -> AbilityExecutionResult {
        let mut res = AbilityExecutionResult {
            cast_result: Some(CastResult::Success),
            damage_type: Some(self.primary_damage_type()),
            ..AbilityExecutionResult::default()
        };

        let ability_name = match event {
            AbilityEvent::CastInstant { ability, .. }
            | AbilityEvent::ChargeReleased { ability, .. }
            | AbilityEvent::ChannelTick { ability, .. }
            | AbilityEvent::ComboStep { ability, .. }
            | AbilityEvent::FormSwitchRequested { ability } => ability.as_str(),
            _ => return res,
        };

        if let AbilityEvent::FormSwitchRequested { ability } = event {
            let target_form = ability.strip_suffix("_form").unwrap_or(ability);
            res.form_switch_to = Some(target_form.to_string());
            return res;
        }

        let Some(def) = self.ability(ability_name) else {
            return res;
        };
        let dmg = def.stats.scaled_damage(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) * ctx.ability_ctx.charge.max(0.5);
        let range = def.stats.scaled_range(
            ctx.player.mastery,
            ctx.config,
            &ctx.ability_ctx.modifiers,
        ) as f64;

        let targets = crate::core::combat::CombatTargeting::in_cone(
            ctx.nearby_targets,
            ctx.player_id,
            ctx.ability_ctx.eye_pos,
            ctx.ability_ctx.look_dir,
            range,
            40.0_f64.to_radians(),
        );
        let push_dir = ctx.ability_ctx.look_dir.normalize_or_zero() * (def.stats.knockback as f64);

        for t in targets {
            res.hits.push(AbilityHitEffect {
                target_id: t.id,
                damage: dmg,
                damage_type: self.primary_damage_type(),
                knockback_impulse: push_dir,
                stun_ticks: 0,
                chill_stacks_added: if self.id == FruitId::HieHie { 1 } else { 0 },
                chill_burst: None,
                freeze_ticks: 0,
                set_fire_seconds: if self.id == FruitId::MaguMagu { 4 } else { 0 },
            });
        }

        if def.is_ultimate {
            ctx.world_fx.emit_shake(ctx.ability_ctx.eye_pos, 0.8, 24);
            res.screen_shake = Some((0.8, 24));
        }

        res
    }
}

/// Registry holding all 16 [`DevilFruitModule`] implementations (`FruitRegistry.java`).
#[derive(Clone)]
pub struct FruitRegistry {
    modules: BTreeMap<FruitId, Arc<dyn DevilFruitModule>>,
    frozen: bool,
}

impl Default for FruitRegistry {
    fn default() -> Self {
        Self::with_builtin_fallbacks()
    }
}

impl FruitRegistry {
    /// Creates a registry pre-populated with all 16 full 1:1 Devil Fruit modules.
    pub fn with_builtin_fallbacks() -> Self {
        let mut modules: BTreeMap<FruitId, Arc<dyn DevilFruitModule>> = BTreeMap::new();
        let all_modules: [Arc<dyn DevilFruitModule>; 16] = [
            Arc::new(GuraGuraModule::new()),
            Arc::new(GomuGomuModule::new()),
            Arc::new(BomuBomuModule::new()),
            Arc::new(KiloKiloModule::new()),
            Arc::new(SukeSukeModule::new()),
            Arc::new(HieHieModule::new()),
            Arc::new(MaguMaguModule::new()),
            Arc::new(MokuMokuModule::new()),
            Arc::new(PikaPikaModule::new()),
            Arc::new(SunaSunaModule::new()),
            Arc::new(SupaSupaModule::new()),
            Arc::new(HoroHoroModule::new()),
            Arc::new(KageKageModule::new()),
            Arc::new(DoruDoruModule::new()),
            Arc::new(YomiYomiModule::new()),
            Arc::new(UoUoSeiryuModule::new()),
        ];
        for m in all_modules {
            modules.insert(m.fruit_id(), m);
        }
        Self {
            modules,
            frozen: false,
        }
    }

    /// Registers or replaces a [`DevilFruitModule`] for its [`FruitId`].
    pub fn register(&mut self, module: Arc<dyn DevilFruitModule>) -> Result<(), String> {
        if self.frozen {
            return Err(format!(
                "FruitRegistry is frozen; cannot register {}",
                module.fruit_id()
            ));
        }
        self.modules.insert(module.fruit_id(), module);
        Ok(())
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub const fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn get(&self, id: FruitId) -> Option<&Arc<dyn DevilFruitModule>> {
        self.modules.get(&id)
    }

    pub fn ability(&self, fruit: FruitId, ability_name: &str) -> Option<AbilityDef> {
        self.get(fruit).and_then(|m| m.ability(ability_name))
    }

    /// Finds which [`FruitId`] owns `ability_name` (supports both bare `"kaishin"` and qualified `"gura_gura:kaishin"`).
    pub fn find_ability(&self, ability_name: &str) -> Option<(FruitId, AbilityDef)> {
        let stripped = ability_name.strip_prefix("devilfruits:").unwrap_or(ability_name);
        if let Some((fruit_part, ab_part)) = stripped.split_once(':').or_else(|| stripped.split_once('/')) {
            if let Ok(fid) = fruit_part.parse::<FruitId>() {
                if let Some(def) = self.ability(fid, ab_part) {
                    return Some((fid, def));
                }
            }
        }
        for id in FruitId::ALL {
            if let Some(def) = self.ability(id, stripped) {
                return Some((id, def));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}
