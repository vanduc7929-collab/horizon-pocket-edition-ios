//! 1:1 Rust port of Grand Line 0.4.0 (`com.devilfruits`, NeoForge 1.21.1, MIT)
//! for Horizon Pocket Edition (`shared-worlds-mods`).
//!
//! Exports:
//! - [`core`]: All 16 core subsystems (Fruit metadata, Rules & Weakness, Ability engine,
//!   Forms, Gears, Logia + SolidBody, Armament Haki, Seastone, Cold/Frozen, Combat &
//!   30 damage types, Item techniques, Poneglyphs & Decipher Desk, Gacha caskets,
//!   World destruction & VFX, Network protocol `"13"`, and Commands).
//! - [`mobs`]: Pirate mob archetypes (`Deckhand`, `Gunner`, `Captain`), AI state machines,
//!   16 animation clips, flintlock/musket ballistics, and loot drops.
//! - [`ui`]: HUD, Ability Wheel, Gear Wheel, Haki meter, Decipher Desk, and iOS touch
//!   controls view-models.
//! - [`fruits`]: [`fruits::DevilFruitModule`] trait, [`fruits::FruitContext`],
//!   [`fruits::AbilityExecutionResult`], and [`fruits::FruitRegistry`].
//! - [`GrandLineRuntime`]: Top-level server/client runtime managing per-player state,
//!   `tick()`, `handle_payload()`, and `handle_command()`.

#![forbid(unsafe_code)]

pub mod core;
pub mod fruits;
pub mod mobs;
pub mod ui;

use std::collections::BTreeMap;

use glam::DVec3;
use horizon_world::grandline_hooks::EntityBodyModifiers;

use crate::core::ability::{
    AbilityContext, AbilityEvent, AbilityManager, AbilityModifiers, AbilityRuntime, AimHelper,
    CastBudget, TechniqueHeat,
};
use crate::core::cold::{ColdManager, ColdState};
use crate::core::combat::{AbilityProjectileTracker, CombatStatus, GrandLineDamageType};
use crate::core::command::{
    CommandParser, CommandResponse, GrandLineCommand, PirateSpawnKind, PoneglyphGiveKind,
};
use crate::core::form::{FormManager, FormRuntime, form_flags};
use crate::core::fruit::FruitId;
use crate::core::gacha::{CasketOpeningTracker, GachaCandidate, GachaConfig, GachaRoll};
use crate::core::gear::{Gear2Config, GearState, GearSwitch};
use crate::core::haki::{HakiConfig, HakiManager, PlayerHakiData};
use crate::core::itemtech::{
    BountyPosterData, DevilFruitEncyclopedia, ItemTechniqueAction, ItemTechniqueDef,
    ItemTechniqueRuntime,
};
use crate::core::logia::{LogiaConfig, LogiaData, LogiaManager, LogiaProfile};
use crate::core::network::{
    AbilityInputAction, C2SPayload, DecipherDeskAction, FormInputAction, HakiInputAction,
    JumpInputAction, LogiaInputAction, S2CPayload,
};
use crate::core::poneglyph::{
    DESK_BUTTON_HINT, DESK_BUTTON_REVEAL, DecipherDeskSession, DecipheredBlueprint,
    EncryptedBlueprint, KnowledgeLearnResult, PlayerKnowledge, PlayerRubbingState, PoneglyphConfig,
    PoneglyphLiteracy, PoneglyphRegistry, RubbingFragment,
};
use crate::core::rules::{
    CoreConfig, DFGameRules, EatOutcome, FruitRules, PlayerFruitData, PlayerUuid, Weakness,
    WeaknessStatus, WorldFruitData,
};
use crate::core::seastone::{SeastoneCombat, SeastoneCombatConfig, SeastoneHandcuffs};
use crate::core::world_fx::WorldFxManager;
use crate::fruits::{AbilityExecutionResult, FruitContext, FruitRegistry};
use crate::mobs::pirate::{PirateArchetype, PirateConfig, PirateMobState};
use crate::ui::{GrandLineHudViewModel, IosTouchControlsState};

/// Complete per-player Grand Line state bundle.
#[derive(Clone, Debug, PartialEq)]
pub struct GrandLinePlayerState {
    pub fruit: PlayerFruitData,
    pub ability_runtime: AbilityRuntime,
    pub form_runtime: FormRuntime,
    pub gear_state: GearState,
    pub haki: PlayerHakiData,
    pub logia: LogiaData,
    pub seastone_cuffs: SeastoneHandcuffs,
    pub cold: ColdState,
    pub combat: CombatStatus,
    pub item_tech: ItemTechniqueRuntime,
    pub heat: TechniqueHeat,
    pub knowledge: PlayerKnowledge,
    pub literacy: PoneglyphLiteracy,
    pub rubbing: PlayerRubbingState,
    pub desk_session: DecipherDeskSession,
    pub encyclopedia: DevilFruitEncyclopedia,
    pub bounty_berries: u64,
    pub touch_controls: IosTouchControlsState,

    pub position: DVec3,
    pub eye_pos: DVec3,
    pub look_dir: DVec3,
    pub velocity: DVec3,
    pub water_depth: f64,
    pub on_water_override: bool,
    pub on_ground: bool,
    pub in_lava: bool,
    pub in_rain: bool,
    pub creative: bool,
    pub food_level: f32,
    pub main_hand_item: Option<String>,
    pub off_hand_item: Option<String>,
    pub seastone_armor_pieces: u32,
    pub standing_on_seastone_block: bool,
    pub inside_seastone_block: bool,
}

impl Default for GrandLinePlayerState {
    fn default() -> Self {
        Self {
            fruit: Default::default(),
            ability_runtime: Default::default(),
            form_runtime: Default::default(),
            gear_state: Default::default(),
            haki: Default::default(),
            logia: Default::default(),
            seastone_cuffs: Default::default(),
            cold: Default::default(),
            combat: Default::default(),
            item_tech: Default::default(),
            heat: Default::default(),
            knowledge: Default::default(),
            literacy: Default::default(),
            rubbing: Default::default(),
            desk_session: Default::default(),
            encyclopedia: Default::default(),
            bounty_berries: 0,
            touch_controls: Default::default(),
            position: DVec3::ZERO,
            eye_pos: DVec3::ZERO,
            look_dir: DVec3::Z,
            velocity: DVec3::ZERO,
            water_depth: 0.0,
            on_water_override: false,
            on_ground: true,
            in_lava: false,
            in_rain: false,
            creative: false,
            food_level: 20.0,
            main_hand_item: None,
            off_hand_item: None,
            seastone_armor_pieces: 0,
            standing_on_seastone_block: false,
            inside_seastone_block: false,
        }
    }
}

impl GrandLinePlayerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates current water & Seastone weakness status (`Weakness::evaluate`).
    pub fn weakness_status(
        &self,
        now: i64,
        core_cfg: &CoreConfig,
        seastone_cfg: &SeastoneCombatConfig,
    ) -> WeaknessStatus {
        let touch = SeastoneCombat::build_touch_state(
            self.main_hand_item.as_deref(),
            self.off_hand_item.as_deref(),
            self.seastone_armor_pieces,
            self.standing_on_seastone_block,
            self.inside_seastone_block,
            self.seastone_cuffs.is_suppressed(now),
        );
        Weakness::evaluate(
            self.fruit.has_fruit(),
            self.creative,
            self.water_depth,
            self.on_water_override,
            touch,
            core_cfg,
            seastone_cfg.to_world_config(),
        )
    }

    /// Builds an [`S2CPayload::PlayerStateSync`] snapshot for networking.
    pub fn to_sync_payload(&self) -> S2CPayload {
        S2CPayload::PlayerStateSync {
            fruit: self.fruit.fruit_id,
            mastery: self.fruit.mastery,
            xp: self.fruit.xp,
            energy: self.fruit.energy,
            max_energy: self.fruit.max_energy(),
            active_form: self.fruit.active_form.clone(),
            selected_ability: self.fruit.selected_ability.clone(),
            cooldowns: self.fruit.cooldowns.clone(),
            haki_unlocked: self.haki.unlocked,
            haki_level: self.haki.level,
            haki_xp: self.haki.xp,
            haki_spirit: self.haki.spirit,
            haki_active: self.haki.active,
            haki_parts: self.haki.effective_parts(),
        }
    }
}

/// Top-level Grand Line runtime orchestrating all subsystems, players, world data, and pirates.
pub struct GrandLineRuntime {
    pub tick_count: i64,
    pub world_seed: i64,
    pub core_config: CoreConfig,
    pub game_rules: DFGameRules,
    pub haki_config: HakiConfig,
    pub logia_config: LogiaConfig,
    pub gear2_config: Gear2Config,
    pub seastone_config: SeastoneCombatConfig,
    pub poneglyph_config: PoneglyphConfig,
    pub gacha_config: GachaConfig,
    pub pirate_config: PirateConfig,

    pub fruit_registry: FruitRegistry,
    pub poneglyph_registry: PoneglyphRegistry,
    pub world_fruits: WorldFruitData,
    pub world_fx: WorldFxManager,
    pub projectiles: AbilityProjectileTracker,
    pub caskets: CasketOpeningTracker,
    pub aim_helper: AimHelper,
    pub players: BTreeMap<PlayerUuid, GrandLinePlayerState>,
    pub pirates: BTreeMap<u64, PirateMobState>,
    next_pirate_id: u64,
}

impl Default for GrandLineRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl GrandLineRuntime {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            world_seed: 0x4752_414E_444C_494E,
            core_config: CoreConfig::default(),
            game_rules: DFGameRules::vanilla_default(),
            haki_config: HakiConfig::default(),
            logia_config: LogiaConfig::default(),
            gear2_config: Gear2Config::default(),
            seastone_config: SeastoneCombatConfig::default(),
            poneglyph_config: PoneglyphConfig::default(),
            gacha_config: GachaConfig::default(),
            pirate_config: PirateConfig::default(),
            fruit_registry: FruitRegistry::with_builtin_fallbacks(),
            poneglyph_registry: PoneglyphRegistry::builtin(),
            world_fruits: WorldFruitData::default(),
            world_fx: WorldFxManager::default(),
            projectiles: AbilityProjectileTracker::default(),
            caskets: CasketOpeningTracker::default(),
            aim_helper: AimHelper::default(),
            players: BTreeMap::new(),
            pirates: BTreeMap::new(),
            next_pirate_id: 0,
        }
    }

    pub fn player_mut(&mut self, id: PlayerUuid) -> &mut GrandLinePlayerState {
        self.players.entry(id).or_default()
    }

    pub fn player(&self, id: PlayerUuid) -> Option<&GrandLinePlayerState> {
        self.players.get(&id)
    }

    /// Computes `horizon-world` [`EntityBodyModifiers`] for `player_id`.
    pub fn body_modifiers_for(&self, player_id: PlayerUuid) -> EntityBodyModifiers {
        let Some(p) = self.players.get(&player_id) else {
            return EntityBodyModifiers::VANILLA;
        };
        let weakness = p.weakness_status(self.tick_count, &self.core_config, &self.seastone_config);
        if let Some(fruit_id) = p.fruit.fruit_id {
            if let Some(module) = self.fruit_registry.get(fruit_id) {
                return module.body_modifiers(
                    &p.fruit,
                    &p.form_runtime,
                    weakness.suppressed,
                    p.water_depth,
                );
            }
        }
        EntityBodyModifiers::VANILLA
    }

    /// Builds the HUD view-model for `player_id`.
    pub fn hud_view_model(&self, player_id: PlayerUuid, wheel_open: bool) -> Option<GrandLineHudViewModel> {
        let p = self.players.get(&player_id)?;
        let weakness = p.weakness_status(self.tick_count, &self.core_config, &self.seastone_config);
        let logia = p
            .fruit
            .fruit_id
            .filter(|f| f.is_logia())
            .map(|_| &p.logia);
        Some(GrandLineHudViewModel::build(
            &p.fruit,
            &p.ability_runtime,
            &p.gear_state,
            &p.haki,
            logia,
            &p.cold,
            weakness.suppressed,
            wheel_open,
            self.tick_count,
            &self.haki_config,
            &self.gear2_config,
        ))
    }

    /// Advances the entire Grand Line server runtime by 1 tick (`20 ticks/sec`).
    pub fn tick(&mut self) {
        self.tick_count += 1;
        let now = self.tick_count;

        self.world_fx.tick(now);
        self.projectiles.step_all(now);
        let _deliveries = self.caskets.tick_deliveries(now);

        let player_ids: Vec<PlayerUuid> = self.players.keys().copied().collect();
        for pid in player_ids {
            self.tick_player(pid, now);
        }

        let first_player_pos = self.players.values().next().map(|p| p.position);
        let pirate_cfg = self.pirate_config;
        for pirate in self.pirates.values_mut() {
            pirate.tick(first_player_pos, &pirate_cfg);
        }
    }

    fn tick_player(&mut self, pid: PlayerUuid, now: i64) {
        let Some(p) = self.players.get_mut(&pid) else {
            return;
        };
        let weakness = p.weakness_status(now, &self.core_config, &self.seastone_config);
        if weakness.submerged_in_water {
            p.velocity = Weakness::apply_sinking(p.velocity, &self.core_config);
        }

        // Cold/Frozen tick
        if ColdManager::tick(&mut p.cold, now, p.in_lava) {
            p.combat.clear_stun();
        }
        p.velocity = p.combat.clamp_velocity(p.velocity, p.position, now);

        // Form upkeep & safe landing
        let (form_regen_mult, _slow_fall) = FormManager::tick(
            &mut p.fruit,
            &mut p.form_runtime,
            now,
            weakness.submerged_in_water,
            weakness.suppressed,
            p.on_ground,
            p.creative,
            &self.core_config,
        );

        // Gomu-Gomu Gear 2 strain & overexertion
        if p.fruit.fruit_id == Some(FruitId::GomuGomu) {
            let g2_out = p.gear_state.tick_gear2(now, p.food_level, &self.gear2_config);
            if g2_out.forced_exit {
                FormManager::revert(&mut p.fruit, &mut p.form_runtime, now, &self.core_config);
                p.combat
                    .apply_exhaustion(now, g2_out.exhaustion_ticks, 2, 1);
            }
        }

        // Logia tick
        let mut logia_regen_mult = 1.0f32;
        if let Some(fid) = p.fruit.fruit_id {
            if let Some(profile) = LogiaProfile::for_fruit(fid) {
                let (dash_pos, env_mult) = LogiaManager::tick(
                    &profile,
                    &mut p.fruit,
                    &mut p.logia,
                    now,
                    weakness.suppressed,
                    false,
                    weakness.submerged_in_water,
                    p.in_rain,
                    p.creative,
                    &self.core_config,
                    &self.logia_config,
                );
                if let Some(pos) = dash_pos {
                    p.position = pos;
                }
                logia_regen_mult = env_mult;
            }
        }

        // Energy regeneration
        if !weakness.suppressed {
            p.fruit
                .regen_energy(now, &self.core_config, form_regen_mult * logia_regen_mult);
        }

        // Haki Spirit drain & regeneration
        HakiManager::tick(&mut p.haki, now, p.creative, &self.haki_config);

        // Item techniques tick
        let builtins = ItemTechniqueDef::builtins();
        let free_to_act = !p.combat.blocks_actions(now) && !p.cold.is_frozen(now);
        let held_item = p.main_hand_item.clone();
        p.item_tech.tick(
            held_item.as_deref(),
            free_to_act,
            |tid| builtins.iter().find(|d| d.id == tid).cloned(),
            now,
        );

        // Ability tick & module hooks
        let active_mods = p
            .fruit
            .fruit_id
            .and_then(|fid| FormManager::lookup_form(fid, &p.fruit.active_form))
            .map(|f| f.stats.ability_modifiers)
            .unwrap_or(AbilityModifiers::IDENTITY);

        let fid_opt = p.fruit.fruit_id;
        let registry = self.fruit_registry.clone();
        let events = AbilityManager::tick(
            &mut p.fruit,
            &mut p.ability_runtime,
            now,
            weakness.suppressed,
            p.creative,
            &self.core_config,
            &active_mods,
            |ab_name| fid_opt.and_then(|fid| registry.ability(fid, ab_name)),
        );

        p.touch_controls.tick_holds();

        if let Some(fid) = fid_opt {
            if let Some(module) = registry.get(fid).cloned() {
                for ev in &events {
                    self.dispatch_fruit_event(pid, &module, ev, active_mods);
                }
            }
        }
    }

    fn dispatch_fruit_event(
        &mut self,
        pid: PlayerUuid,
        module: &std::sync::Arc<dyn crate::fruits::DevilFruitModule>,
        event: &AbilityEvent,
        modifiers: AbilityModifiers,
    ) -> AbilityExecutionResult {
        let now = self.tick_count;
        let Some(p) = self.players.get_mut(&pid) else {
            return AbilityExecutionResult::default();
        };
        let weakness = p.weakness_status(now, &self.core_config, &self.seastone_config);
        let aim_point = self.aim_helper.resolve(pid, p.eye_pos, p.look_dir, now);

        let (ability_name, charge, combo_step, channel_ticks) = match event {
            AbilityEvent::CastInstant { ability, .. } => (ability.clone(), 1.0, 1, 0),
            AbilityEvent::ChargeReleased {
                ability, charge, ..
            } => (ability.clone(), *charge, 1, 0),
            AbilityEvent::ChannelTick {
                ability,
                elapsed_ticks,
            }
            | AbilityEvent::ChannelEnded {
                ability,
                elapsed_ticks,
            } => (ability.clone(), 1.0, 1, *elapsed_ticks),
            AbilityEvent::ComboStep { ability, step, .. } => (ability.clone(), 1.0, *step, 0),
            AbilityEvent::Toggled { ability, .. }
            | AbilityEvent::ToggleTick { ability }
            | AbilityEvent::FormSwitchRequested { ability } => (ability.clone(), 1.0, 1, 0),
            _ => return AbilityExecutionResult::default(),
        };

        let ability_ctx = AbilityContext {
            caster: pid,
            fruit: module.fruit_id(),
            ability_name,
            mastery: p.fruit.mastery,
            charge,
            combo_step,
            channel_ticks,
            tick: now,
            eye_pos: p.eye_pos,
            look_dir: p.look_dir,
            aim_point,
            modifiers,
        };

        let budget = CastBudget::from_config(&self.core_config);
        let mut ctx = FruitContext {
            player_id: pid,
            player: &mut p.fruit,
            form_runtime: &mut p.form_runtime,
            gear_state: &mut p.gear_state,
            haki: &mut p.haki,
            logia: Some(&mut p.logia),
            heat: &mut p.heat,
            world_fx: &mut self.world_fx,
            budget,
            ability_ctx,
            nearby_targets: &[],
            config: &self.core_config,
            game_rules: self.game_rules,
            creative: p.creative,
            submerged_in_water: weakness.submerged_in_water,
            suppressed: weakness.suppressed,
            on_ground: p.on_ground,
            in_lava: p.in_lava,
        };

        let result = module.execute_ability(&mut ctx, event);
        if let Some(target_form) = &result.form_switch_to {
            FormManager::request_form(
                ctx.player,
                ctx.form_runtime,
                target_form,
                now,
                weakness.suppressed,
                ctx.creative,
                &self.core_config,
            );
        }
        if let Some(imp) = result.caster_velocity_impulse {
            p.velocity += imp;
        }
        result
    }

    /// Processes an incoming C2S network payload from `player_id` and returns resulting S2C responses.
    pub fn handle_payload(&mut self, player_id: PlayerUuid, payload: C2SPayload) -> Vec<S2CPayload> {
        let now = self.tick_count;
        let mut out = Vec::new();
        self.player_mut(player_id);

        match payload {
            C2SPayload::AbilityInput { action, ability } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if action == AbilityInputAction::Select {
                    p.fruit.selected_ability = Some(ability);
                    out.push(p.to_sync_payload());
                    return out;
                }
                let Some(fid) = p.fruit.fruit_id else {
                    return out;
                };
                let Some(module) = self.fruit_registry.get(fid).cloned() else {
                    return out;
                };
                let Some(def) = module.ability(&ability) else {
                    return out;
                };
                let weakness =
                    p.weakness_status(now, &self.core_config, &self.seastone_config);
                let blocked = p.combat.blocks_actions(now) || p.cold.is_frozen(now);
                let mods = FormManager::lookup_form(fid, &p.fruit.active_form)
                    .map(|f| f.stats.ability_modifiers)
                    .unwrap_or(AbilityModifiers::IDENTITY);

                let (_res, events) = match action {
                    AbilityInputAction::Press => AbilityManager::on_press(
                        &mut p.fruit,
                        &mut p.ability_runtime,
                        &def,
                        now,
                        weakness.suppressed,
                        blocked,
                        p.creative,
                        &self.core_config,
                        &mods,
                    ),
                    AbilityInputAction::Release => AbilityManager::on_release(
                        &mut p.fruit,
                        &mut p.ability_runtime,
                        &def,
                        now,
                        p.creative,
                        &self.core_config,
                        &mods,
                    ),
                    AbilityInputAction::Select => unreachable!(),
                };

                for ev in &events {
                    self.dispatch_fruit_event(player_id, &module, ev, mods);
                }
                if let Some(p) = self.players.get(&player_id) {
                    out.push(p.to_sync_payload());
                }
            }
            C2SPayload::FormInput { action, form } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                let weakness =
                    p.weakness_status(now, &self.core_config, &self.seastone_config);
                match action {
                    FormInputAction::Revert => {
                        FormManager::revert(
                            &mut p.fruit,
                            &mut p.form_runtime,
                            now,
                            &self.core_config,
                        );
                    }
                    FormInputAction::Set => {
                        FormManager::request_form(
                            &mut p.fruit,
                            &mut p.form_runtime,
                            &form,
                            now,
                            weakness.suppressed,
                            p.creative,
                            &self.core_config,
                        );
                    }
                    FormInputAction::Cycle => {
                        if let Some(fid) = p.fruit.fruit_id {
                            let forms = fid.meta().forms;
                            let cur_idx = forms
                                .iter()
                                .position(|&f| f == p.fruit.active_form)
                                .unwrap_or(0);
                            let next_form = forms[(cur_idx + 1) % forms.len()];
                            FormManager::request_form(
                                &mut p.fruit,
                                &mut p.form_runtime,
                                next_form,
                                now,
                                weakness.suppressed,
                                p.creative,
                                &self.core_config,
                            );
                        }
                    }
                    FormInputAction::BoostStart => {
                        p.form_runtime.set_flag(form_flags::BOOSTING, true);
                    }
                    FormInputAction::BoostStop => {
                        p.form_runtime.set_flag(form_flags::BOOSTING, false);
                    }
                }
                out.push(S2CPayload::FormState {
                    entity_id: 0,
                    fruit: p.fruit.fruit_id,
                    form_name: p.fruit.active_form.clone(),
                    flags: p.form_runtime.flags,
                });
            }
            C2SPayload::GearSwitch { gear_id, sub_gear } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if let Some(fid) = p.fruit.fruit_id {
                    let weakness =
                        p.weakness_status(now, &self.core_config, &self.seastone_config);
                    let outcome = GearSwitch::request(
                        fid,
                        p.fruit.mastery,
                        weakness.suppressed,
                        now,
                        &gear_id,
                        sub_gear.as_deref(),
                        &mut p.gear_state,
                    );
                    if outcome == crate::core::gear::GearSwitchOutcome::Switched {
                        let target_form = if gear_id == "gear_2" { "gear_2" } else { "base" };
                        FormManager::request_form(
                            &mut p.fruit,
                            &mut p.form_runtime,
                            target_form,
                            now,
                            weakness.suppressed,
                            p.creative,
                            &self.core_config,
                        );
                    }
                }
                out.push(p.to_sync_payload());
            }
            C2SPayload::HakiInput { action, mask } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                match action {
                    HakiInputAction::Toggle => {
                        HakiManager::toggle_coating(
                            &mut p.haki,
                            now,
                            p.creative,
                            &self.haki_config,
                        );
                    }
                    HakiInputAction::Select => {
                        HakiManager::set_parts(&mut p.haki, mask);
                    }
                    HakiInputAction::RyuoStart => {
                        HakiManager::start_ryuo(&mut p.haki, now);
                    }
                    HakiInputAction::RyuoRelease => {
                        HakiManager::release_ryuo(
                            &mut p.haki,
                            now,
                            p.creative,
                            &self.haki_config,
                        );
                    }
                }
                out.push(p.to_sync_payload());
            }
            C2SPayload::LogiaInput { action } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if let Some(fid) = p.fruit.fruit_id {
                    if let Some(profile) = LogiaProfile::for_fruit(fid) {
                        let weakness =
                            p.weakness_status(now, &self.core_config, &self.seastone_config);
                        match action {
                            LogiaInputAction::ElementalOn => {
                                if p.logia.state != crate::core::logia::LogiaState::ElementalBody {
                                    LogiaManager::toggle_elemental_body(
                                        &profile,
                                        &mut p.logia,
                                        now,
                                        weakness.suppressed,
                                        &self.logia_config,
                                    );
                                }
                            }
                            LogiaInputAction::ElementalOff => {
                                if p.logia.state == crate::core::logia::LogiaState::ElementalBody {
                                    LogiaManager::toggle_elemental_body(
                                        &profile,
                                        &mut p.logia,
                                        now,
                                        weakness.suppressed,
                                        &self.logia_config,
                                    );
                                }
                            }
                            LogiaInputAction::ElementalToggle => {
                                LogiaManager::toggle_elemental_body(
                                    &profile,
                                    &mut p.logia,
                                    now,
                                    weakness.suppressed,
                                    &self.logia_config,
                                );
                            }
                            LogiaInputAction::Dash => {
                                let target = p.position + p.look_dir * (profile.dash_range as f64);
                                LogiaManager::start_dash(
                                    &profile,
                                    &mut p.fruit,
                                    &mut p.logia,
                                    p.position,
                                    target,
                                    now,
                                    weakness.suppressed,
                                    p.creative,
                                    &self.core_config,
                                    &self.logia_config,
                                );
                            }
                            LogiaInputAction::IntangibilityToggle => {
                                p.logia.intangibility_toggled = !p.logia.intangibility_toggled;
                            }
                        }
                        out.push(S2CPayload::LogiaState {
                            entity_id: 0,
                            state: p.logia.state,
                            dissolved_limbs: p.logia.dissolved_limbs,
                            solid_cracks: p.logia.solid_state.cracks,
                            solid_shattered: p.logia.solid_state.shattered_limbs,
                        });
                    }
                }
            }
            C2SPayload::DashInput { dx, dz } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if dx.is_finite() && dz.is_finite() && p.form_runtime.dash.trigger(now, 30, 8) {
                    let dir = DVec3::new(dx as f64, 0.0, dz as f64).normalize_or_zero();
                    p.velocity = DVec3::new(dir.x * 1.25, p.velocity.y.max(0.2), dir.z * 1.25);
                }
            }
            C2SPayload::JumpInput { action } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                match action {
                    JumpInputAction::Start => {
                        p.form_runtime.charged_jump.start(now);
                    }
                    JumpInputAction::Release => {
                        let max_h = p
                            .fruit
                            .fruit_id
                            .and_then(|fid| FormManager::lookup_form(fid, &p.fruit.active_form))
                            .and_then(|f| f.stats.charged_jump_max_height)
                            .unwrap_or(12.0);
                        if let Some(vel) =
                            p.form_runtime.charged_jump.release(now, max_h, 0.08, p.look_dir)
                        {
                            p.velocity = vel;
                        }
                    }
                }
            }
            C2SPayload::Aim { point } => {
                self.aim_helper
                    .record(player_id, DVec3::from_array(point), now);
            }
            C2SPayload::ItemTechnique {
                action,
                predicted_step,
            } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                let free = !p.combat.blocks_actions(now) && !p.cold.is_frozen(now);
                let builtins = ItemTechniqueDef::builtins();
                if let Some(held_id) = p.main_hand_item.as_deref() {
                    let norm_id = crate::core::fruit::df_id(held_id);
                    match action {
                        ItemTechniqueAction::Attack => {
                            if let Some(def) = builtins.iter().find(|d| {
                                d.item_id == norm_id
                                    && d.kind == crate::core::itemtech::ItemTechniqueKind::Combo
                            }) {
                                let pred = (predicted_step >= 0).then_some(predicted_step as usize);
                                p.item_tech.attack(def, pred, free, now);
                            }
                        }
                        ItemTechniqueAction::Use => {
                            if let Some(def) = builtins.iter().find(|d| {
                                d.item_id == norm_id
                                    && d.kind == crate::core::itemtech::ItemTechniqueKind::Held
                            }) {
                                p.item_tech.begin_hold(def, free, now);
                            }
                        }
                        ItemTechniqueAction::Release => {
                            p.item_tech.end_hold(160, false, now);
                        }
                    }
                }
            }
            C2SPayload::Struggle => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if ColdManager::struggle(&mut p.cold, now) {
                    p.combat.clear_stun();
                }
                let coated = p.haki.effective_parts() != 0;
                if let Some(hold) = &mut p.combat.restraint {
                    if hold.struggle(now, coated) {
                        p.combat.restraint = None;
                    }
                }
            }
            C2SPayload::PoneglyphRubHeld { pos } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                let first_poneglyph = self
                    .poneglyph_registry
                    .poneglyphs
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "devilfruits:seastone_tools".into());
                let _ = p.rubbing.step_rubbing(
                    pos,
                    &first_poneglyph,
                    0,
                    &mut p.literacy,
                    &p.knowledge,
                    &self.poneglyph_registry,
                    p.creative,
                    now,
                    &self.poneglyph_config,
                );
            }
            C2SPayload::DecipherDesk {
                container_id,
                action,
            } => {
                let Some(p) = self.players.get_mut(&player_id) else {
                    return out;
                };
                if p.desk_session.container_id == container_id {
                    let button = match action {
                        DecipherDeskAction::PickOption(idx) => idx as i32,
                        DecipherDeskAction::UseHint => DESK_BUTTON_HINT,
                        DecipherDeskAction::UseReveal => DESK_BUTTON_REVEAL,
                    };
                    p.desk_session.click_button(
                        button,
                        1,
                        &mut p.literacy,
                        &self.poneglyph_registry,
                        now,
                        &self.poneglyph_config,
                    );
                }
            }
            C2SPayload::FireGun { .. } => {
                let Some(p) = self.players.get(&player_id) else {
                    return out;
                };
                let eye = p.eye_pos;
                let dir = p.look_dir.normalize_or_zero();
                let speed = self.pirate_config.ball_speed as f64;
                let dmg = self.pirate_config.pistol_damage;
                self.projectiles.spawn(
                    player_id,
                    eye + dir,
                    dir * speed,
                    0.15,
                    dmg,
                    GrandLineDamageType::DevilFruit,
                    now + 80,
                );
            }
        }

        out
    }

    /// Parses and executes a Grand Line command string (`/devilfruit ...`, `/haki ...`, `/ability ...`, `/poneglyph ...`, `/bounty ...`).
    pub fn handle_command(&mut self, player_id: PlayerUuid, command_line: &str) -> CommandResponse {
        let cmd = match CommandParser::parse(command_line) {
            Ok(c) => c,
            Err(e) => return CommandResponse::err(e),
        };
        self.execute_command(player_id, cmd)
    }

    /// Executes a parsed [`GrandLineCommand`] against `player_id` and world state.
    pub fn execute_command(
        &mut self,
        player_id: PlayerUuid,
        cmd: GrandLineCommand,
    ) -> CommandResponse {
        let now = self.tick_count;
        self.player_mut(player_id);

        match cmd {
            GrandLineCommand::GiveFruit { fruit } => {
                let p = self.players.get_mut(&player_id).unwrap();
                FruitRules::grant(
                    player_id,
                    &mut p.fruit,
                    &mut self.world_fruits,
                    fruit,
                    &self.core_config,
                );
                p.encyclopedia.record(fruit);
                CommandResponse::ok(format!("Granted {} to player", fruit.meta().display_name))
            }
            GrandLineCommand::RemoveFruit => {
                let p = self.players.get_mut(&player_id).unwrap();
                if let Some(old) = FruitRules::remove(
                    player_id,
                    &mut p.fruit,
                    &mut self.world_fruits,
                    &self.core_config,
                ) {
                    CommandResponse::ok(format!("Removed {}", old.meta().display_name))
                } else {
                    CommandResponse::ok("Player had no Devil Fruit")
                }
            }
            GrandLineCommand::SetForm { form } => {
                let p = self.players.get_mut(&player_id).unwrap();
                let res = FormManager::request_form(
                    &mut p.fruit,
                    &mut p.form_runtime,
                    &form,
                    now,
                    false,
                    true,
                    &self.core_config,
                );
                CommandResponse::ok(format!("Form switch to '{form}': {res:?}"))
            }
            GrandLineCommand::SetMastery { level } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.fruit.mastery = level.clamp(1, 10);
                p.fruit.xp = 0.0;
                p.fruit.energy = p.fruit.max_energy();
                CommandResponse::ok(format!("Set mastery level to {}", p.fruit.mastery))
            }
            GrandLineCommand::ResetFruit => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.fruit.clear_cooldowns();
                p.fruit.energy = p.fruit.max_energy();
                p.ability_runtime = AbilityRuntime::default();
                CommandResponse::ok("Reset cooldowns and restored energy")
            }
            GrandLineCommand::ListFruits => {
                let free = self.world_fruits.free_fruits().len();
                CommandResponse::ok(format!(
                    "16 playable Devil Fruits registered ({free} free in world)"
                ))
            }
            GrandLineCommand::FruitInfo => {
                let p = self.players.get(&player_id).unwrap();
                let fname = p
                    .fruit
                    .fruit_id
                    .map(|f| f.meta().display_name)
                    .unwrap_or("None");
                CommandResponse::ok(format!(
                    "Fruit: {fname}, Form: {}, Mastery: {} (XP {:.0}), Energy: {:.0}/{:.0}",
                    p.fruit.active_form,
                    p.fruit.mastery,
                    p.fruit.xp,
                    p.fruit.energy,
                    p.fruit.max_energy()
                ))
            }
            GrandLineCommand::CastAbility {
                ability,
                hold_ticks: _,
            } => {
                let Some((fid, def)) = self.fruit_registry.find_ability(&ability) else {
                    return CommandResponse::err(format!("Unknown ability: {ability}"));
                };
                {
                    let p = self.players.get_mut(&player_id).unwrap();
                    if p.fruit.fruit_id != Some(fid) {
                        FruitRules::grant(
                            player_id,
                            &mut p.fruit,
                            &mut self.world_fruits,
                            fid,
                            &self.core_config,
                        );
                    }
                    if p.fruit.mastery < def.stats.unlock_mastery {
                        p.fruit.mastery = def.stats.unlock_mastery;
                    }
                    if !def.allowed_in_form(&p.fruit.active_form) {
                        if let Some(first_form) = def.allowed_forms.first() {
                            p.fruit.active_form = first_form.clone();
                        }
                    }
                    p.fruit.clear_cooldowns();
                    p.fruit.energy = p.fruit.max_energy();
                }
                self.handle_payload(
                    player_id,
                    C2SPayload::AbilityInput {
                        action: AbilityInputAction::Press,
                        ability: def.id.name.clone(),
                    },
                );
                CommandResponse::ok(format!("Cast ability {}", def.id.full_key()))
            }
            GrandLineCommand::ReleaseWorldFruit { fruit } => {
                self.world_fruits.release(fruit);
                CommandResponse::ok(format!(
                    "Released {} back to world loot pool",
                    fruit.meta().display_name
                ))
            }
            GrandLineCommand::LogiaSetState { state, ticks } => {
                let p = self.players.get_mut(&player_id).unwrap();
                if !p.fruit.fruit_id.is_some_and(|f| f.is_logia()) {
                    return CommandResponse::err("Player does not hold a Logia Devil Fruit");
                }
                p.logia.state = state;
                p.logia.reform_until_tick = now + ticks as i64;
                CommandResponse::ok(format!("Set Logia state to {state:?} for {ticks} ticks"))
            }
            GrandLineCommand::LogiaForceCounter { counter, ticks } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.logia
                    .active_counters
                    .insert(counter.clone(), now + ticks as i64);
                CommandResponse::ok(format!("Forced Logia counter '{counter}' for {ticks} ticks"))
            }
            GrandLineCommand::CasketGive { count } => {
                CommandResponse::ok(format!("Gave {count} Devil Fruit Casket(s)"))
            }
            GrandLineCommand::CasketGiveKey { count } => {
                CommandResponse::ok(format!("Gave {count} Gold Key(s)"))
            }
            GrandLineCommand::CasketRollSimulation { rolls } => {
                let candidates = GachaRoll::candidates(&self.world_fruits, &self.core_config);
                if !GachaRoll::is_rollable(&candidates, &self.gacha_config) {
                    return CommandResponse::err("No rollable fruits in pool");
                }
                let mut counts: BTreeMap<FruitId, u32> = BTreeMap::new();
                for i in 0..rolls {
                    let seed = (self.world_seed as u64).wrapping_add((i as u64) * 997);
                    if let Some(GachaCandidate { fruit, .. }) =
                        GachaRoll::roll_with_seed(&candidates, &self.gacha_config, seed)
                    {
                        *counts.entry(fruit).or_insert(0) += 1;
                    }
                }
                CommandResponse::ok(format!(
                    "Simulated {rolls} casket rolls across {} distinct fruits",
                    counts.len()
                ))
            }
            GrandLineCommand::CasketPreview { tier, fruit } => {
                let chosen = fruit.unwrap_or(FruitId::GuraGura);
                CommandResponse::ok(format!(
                    "Previewing {:?} casket reveal for {}",
                    tier,
                    chosen.meta().display_name
                ))
            }
            GrandLineCommand::KnowledgeGrant { knowledge_id } => {
                let p = self.players.get_mut(&player_id).unwrap();
                match p.knowledge.learn(&knowledge_id, &self.poneglyph_registry) {
                    KnowledgeLearnResult::Learned | KnowledgeLearnResult::Already => {
                        CommandResponse::ok(format!("Granted knowledge {knowledge_id}"))
                    }
                    KnowledgeLearnResult::Unknown => {
                        CommandResponse::err(format!("Unknown knowledge branch: {knowledge_id}"))
                    }
                }
            }
            GrandLineCommand::KnowledgeRevoke { knowledge_id } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.knowledge.revoke(&knowledge_id);
                CommandResponse::ok(format!("Revoked knowledge {knowledge_id}"))
            }
            GrandLineCommand::KnowledgeList => {
                let p = self.players.get(&player_id).unwrap();
                CommandResponse::ok(format!(
                    "Learned {} knowledge branches (Literacy Lv.{} / {} XP)",
                    p.knowledge.learned.len(),
                    p.literacy.level,
                    p.literacy.xp
                ))
            }
            GrandLineCommand::PoneglyphGive { kind, poneglyph_id } => {
                let key = crate::core::fruit::df_id(&poneglyph_id);
                if !self.poneglyph_registry.poneglyphs.contains_key(&key) {
                    return CommandResponse::err(format!("Unknown poneglyph: {key}"));
                }
                let p = self.players.get_mut(&player_id).unwrap();
                match kind {
                    PoneglyphGiveKind::Rubbings => {
                        let frags: Vec<RubbingFragment> = (0..9)
                            .map(|cell| RubbingFragment {
                                poneglyph: key.clone(),
                                cell,
                            })
                            .collect();
                        if let Ok(bp) = RubbingFragment::assemble_blueprint(&frags, self.world_seed)
                        {
                            p.desk_session.input = Some(bp);
                        }
                    }
                    PoneglyphGiveKind::Encrypted => {
                        p.desk_session.input = Some(EncryptedBlueprint {
                            poneglyph: key.clone(),
                            seed: self.world_seed,
                            progress: Default::default(),
                        });
                    }
                    PoneglyphGiveKind::Deciphered => {
                        let knowledge = self.poneglyph_registry.poneglyphs[&key].knowledge.clone();
                        p.desk_session.output = Some(DecipheredBlueprint {
                            knowledge,
                            poneglyph: key.clone(),
                        });
                    }
                }
                CommandResponse::ok(format!("Gave {kind:?} for {key}"))
            }
            GrandLineCommand::PoneglyphSetLiteracy { level } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.literacy.set_level(level, &self.poneglyph_config);
                CommandResponse::ok(format!(
                    "Set Poneglyph Literacy to level {}",
                    p.literacy.level
                ))
            }
            GrandLineCommand::PirateSpawn {
                kind,
                variant,
                with_crew,
            } => {
                let pos = self
                    .players
                    .get(&player_id)
                    .map(|p| p.position)
                    .unwrap_or(DVec3::ZERO);
                let arch = match kind {
                    PirateSpawnKind::Deckhand => PirateArchetype::Deckhand,
                    PirateSpawnKind::Gunner => PirateArchetype::Gunner,
                    PirateSpawnKind::Captain => PirateArchetype::Captain,
                };
                self.next_pirate_id += 1;
                let id = self.next_pirate_id;
                let mob = PirateMobState::spawn(
                    id,
                    arch,
                    variant.unwrap_or(0),
                    pos,
                    false,
                    &self.pirate_config,
                );
                self.pirates.insert(id, mob);
                if with_crew && arch == PirateArchetype::Captain {
                    for i in 0..3 {
                        self.next_pirate_id += 1;
                        let cid = self.next_pirate_id;
                        let carch = if i < 2 {
                            PirateArchetype::Deckhand
                        } else {
                            PirateArchetype::Gunner
                        };
                        let mut mate = PirateMobState::spawn(
                            cid,
                            carch,
                            i as u8,
                            pos + DVec3::new((i + 1) as f64 * 1.5, 0.0, 0.0),
                            i == 2,
                            &self.pirate_config,
                        );
                        mate.leader_id = Some(id);
                        self.pirates.insert(cid, mate);
                    }
                }
                CommandResponse::ok(format!("Spawned Pirate {arch:?} (id={id})"))
            }
            GrandLineCommand::PiratePlayAnim { clip } => {
                let Some(c) = crate::mobs::pirate::PirateClip::by_name(&clip) else {
                    return CommandResponse::err(format!("Unknown pirate clip: {clip}"));
                };
                if let Some(pirate) = self.pirates.values_mut().next() {
                    pirate.play_action(c);
                    CommandResponse::ok(format!("Playing pirate clip '{clip}'"))
                } else {
                    CommandResponse::err("No pirate nearby")
                }
            }
            GrandLineCommand::GomuLimbTest { skin, mode } => {
                CommandResponse::ok(format!("limb test: {skin} {mode}"))
            }
            GrandLineCommand::HakiUnlock => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.haki.unlock(&self.haki_config);
                CommandResponse::ok("Unlocked Armament Haki at level 1")
            }
            GrandLineCommand::HakiSetLevel { level } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.haki.set_level(level, &self.haki_config);
                CommandResponse::ok(format!("Set Armament Haki level to {}", p.haki.level))
            }
            GrandLineCommand::HakiAddXp { amount } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.haki.unlock(&self.haki_config);
                let gained = p.haki.add_xp(amount, &self.haki_config);
                CommandResponse::ok(format!(
                    "Added {amount:.1} Haki XP (+{gained} levels, now Lv.{})",
                    p.haki.level
                ))
            }
            GrandLineCommand::HakiReset => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.haki.set_level(0, &self.haki_config);
                CommandResponse::ok("Reset Armament Haki")
            }
            GrandLineCommand::HakiInfo => {
                let p = self.players.get(&player_id).unwrap();
                CommandResponse::ok(format!(
                    "Armament Haki: unlocked={}, level={}, tier={:?}, spirit={:.0}/{:.0}, mask={:#011b}",
                    p.haki.unlocked,
                    p.haki.level,
                    p.haki.tier(),
                    p.haki.spirit,
                    p.haki.max_spirit(&self.haki_config),
                    p.haki.effective_parts()
                ))
            }
            GrandLineCommand::BountyGet => {
                let p = self.players.get(&player_id).unwrap();
                CommandResponse::ok(format!("Current Bounty: {} Berries", p.bounty_berries))
            }
            GrandLineCommand::BountySet { berries } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.bounty_berries = berries;
                let _poster = BountyPosterData::new(player_id, "Player", berries);
                CommandResponse::ok(format!("Set Bounty to {berries} Berries"))
            }
            GrandLineCommand::BountyAdd { berries } => {
                let p = self.players.get_mut(&player_id).unwrap();
                p.bounty_berries = p.bounty_berries.saturating_add(berries);
                CommandResponse::ok(format!("Updated Bounty to {} Berries", p.bounty_berries))
            }
        }
    }

    /// Player eats a Devil Fruit item (`FruitRules::try_eat`).
    pub fn eat_fruit(&mut self, player_id: PlayerUuid, fruit: FruitId) -> EatOutcome {
        self.player_mut(player_id);
        let p = self.players.get_mut(&player_id).unwrap();
        let outcome = FruitRules::try_eat(
            player_id,
            &mut p.fruit,
            &mut self.world_fruits,
            fruit,
            &self.core_config,
        );
        if matches!(outcome, EatOutcome::Granted(_)) {
            p.encyclopedia.record(fruit);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cold::{ChillBurst, ChillOutcome, FrozenKind, FrozenRules};
    use crate::core::combat::{CombatResolver, CombatTargeting, TargetCandidate};
    use crate::core::fruit::DevilFruitType;
    use crate::core::gacha::{CasketOpenOutcome, GachaTier};
    use crate::core::haki::{HakiPart, HakiParts, HakiTier, HakiTraining};
    use crate::core::logia::{DamageCategory, LogiaBypassResult, LogiaBypassSource};
    use crate::core::network::GrandLinePacket;
    use crate::core::poneglyph::{Cipher, CipherMode, PuzzleOutcome, RubbingFragment};
    use crate::core::rules::Mastery;
    use crate::core::seastone::{SEASTONE_JITTE_ID, SEASTONE_SWORD_ID};
    use crate::ui::IosTouchButton;

    const P1: PlayerUuid = [1; 16];
    const P2: PlayerUuid = [2; 16];

    #[test]
    fn all_16_fruits_and_30_damage_types_have_canonical_metadata() {
        assert_eq!(FruitId::ALL.len(), 16);
        assert_eq!(GrandLineDamageType::ALL.len(), 30);

        let logias: Vec<FruitId> = FruitId::ALL
            .iter()
            .copied()
            .filter(|f| f.is_logia())
            .collect();
        assert_eq!(logias.len(), 5);

        let gura = FruitId::GuraGura.meta();
        assert_eq!(gura.fruit_type, DevilFruitType::Paramecia);
        assert_eq!(gura.gacha_tier, GachaTier::Uncommon);
        assert_eq!(gura.energy.max_energy(1), 200.0);
        assert_eq!(gura.energy.max_energy(10), 380.0);

        let seiryu = FruitId::UoUoSeiryu.meta();
        assert_eq!(seiryu.fruit_type, DevilFruitType::MythicalZoan);
        assert_eq!(seiryu.forms, &["human", "hybrid", "dragon"]);
    }

    #[test]
    fn eating_second_fruit_is_fatal_and_world_uniqueness_enforced() {
        let mut rt = GrandLineRuntime::new();
        assert_eq!(
            rt.eat_fruit(P1, FruitId::GuraGura),
            EatOutcome::Granted(FruitId::GuraGura)
        );
        // Second fruit on P1 is fatal
        assert_eq!(
            rt.eat_fruit(P1, FruitId::PikaPika),
            EatOutcome::SecondFruitFatal
        );
        // Same fruit on P2 is blocked by one_per_world
        assert_eq!(
            rt.eat_fruit(P2, FruitId::GuraGura),
            EatOutcome::WorldDuplicateBlocked
        );
    }

    #[test]
    fn mastery_xp_curve_and_scaling_match_java() {
        let cfg = CoreConfig::default();
        let mut p = PlayerFruitData {
            fruit_id: Some(FruitId::GuraGura),
            ..Default::default()
        };
        let gained = Mastery::award_xp(&mut p, 300.0, &cfg);
        assert_eq!(gained, 2); // 100 (1->2) + 180 (2->3) = 280 <= 300
        assert_eq!(p.mastery, 3);
        assert!((p.xp - 20.0).abs() < 1e-3);
        assert!((Mastery::damage_multiplier(10, &cfg) - 2.08).abs() < 1e-4);
    }

    #[test]
    fn water_and_seastone_weakness_with_jitte_safe_handle() {
        let mut rt = GrandLineRuntime::new();
        rt.eat_fruit(P1, FruitId::PikaPika);

        {
            let p = rt.player_mut(P1);
            p.main_hand_item = Some(SEASTONE_JITTE_ID.to_string());
        }
        // Jitte has safe handle -> holder is NOT suppressed
        let w_jitte =
            rt.player(P1)
                .unwrap()
                .weakness_status(0, &rt.core_config, &rt.seastone_config);
        assert!(!w_jitte.suppressed);

        // Holding a Seastone Sword DOES suppress holder
        {
            let p = rt.player_mut(P1);
            p.main_hand_item = Some(SEASTONE_SWORD_ID.to_string());
        }
        let w_sword =
            rt.player(P1)
                .unwrap()
                .weakness_status(0, &rt.core_config, &rt.seastone_config);
        assert!(w_sword.touching_seastone);
        assert!(w_sword.suppressed);
    }

    #[test]
    fn cold_chill_stacks_burst_and_frozen_struggle() {
        let mut cold = ColdState::NONE;
        let burst = ChillBurst::default();
        assert_eq!(
            ColdManager::add_chill(&mut cold, 10, Some("hie"), burst),
            ChillOutcome::Stacked(1)
        );
        assert_eq!(
            ColdManager::add_chill(&mut cold, 15, Some("hie"), burst),
            ChillOutcome::Stacked(2)
        );
        assert_eq!(
            ColdManager::add_chill(&mut cold, 20, Some("hie"), burst),
            ChillOutcome::Stacked(3)
        );
        match ColdManager::add_chill(&mut cold, 25, Some("hie"), burst) {
            ChillOutcome::BurstTriggered { damage, .. } => assert!((damage - 6.0).abs() < 1e-4),
            other => panic!("expected burst on 4th chill stack, got {other:?}"),
        }
        assert_eq!(cold.chill_at(25), 0);

        // Freeze and struggle out
        let rules = FrozenRules::of(FrozenKind::Block, 100).with_struggle(3.0, 1.0);
        assert!(ColdManager::freeze(
            &mut cold,
            rules,
            Some("hie"),
            false,
            false,
            (0.0, 0.0, 0.0),
            30
        ));
        assert!(cold.is_frozen(35));
        assert!(!ColdManager::struggle(&mut cold, 35));
        assert!(!ColdManager::struggle(&mut cold, 38));
        assert!(ColdManager::struggle(&mut cold, 41));
        assert!(!cold.is_frozen(41));
    }

    #[test]
    fn haki_parts_normalization_logia_bypass_and_combat_resolution() {
        assert_eq!(
            HakiParts::normalize(HakiParts::RIGHT_ARM),
            HakiParts::RIGHT_ARM | HakiParts::RIGHT_FIST
        );
        assert_eq!(HakiTier::Novice.allowed_parts(), 259);
        assert_eq!(HakiTier::Adept.allowed_parts(), 319);
        assert_eq!(HakiTier::Expert.allowed_parts(), 511);

        let mut atk_fruit = PlayerFruitData::default();
        let mut atk_haki = PlayerHakiData::default();
        let haki_cfg = HakiConfig::default();
        atk_haki.set_level(4, &haki_cfg); // Level 4 -> 100% Logia reach (0.2 + 0.2*4 = 1.0)
        HakiManager::toggle_coating(&mut atk_haki, 10, false, &haki_cfg);
        assert!(atk_haki.active);
        assert!((atk_haki.logia_reach(&haki_cfg) - 1.0).abs() < 1e-4);

        let mut def_fruit = PlayerFruitData {
            fruit_id: Some(FruitId::SunaSuna),
            mastery: 5,
            energy: 140.0,
            ..Default::default()
        };
        let mut def_logia = LogiaData::default();
        let mut def_cuffs = SeastoneHandcuffs::default();
        let mut def_combat = CombatStatus::default();

        // Without Haki/Seastone, melee passes through Suna-Suna and carves a hole
        let profile = LogiaProfile::for_fruit(FruitId::SunaSuna).unwrap();
        let pass_out = LogiaManager::evaluate_damage(
            &profile,
            &mut def_fruit,
            &mut def_logia,
            DamageCategory::Melee,
            10.0,
            1,
            glam::Vec3::ZERO,
            glam::Vec3::Z,
            None,
            None,
            20,
            false,
            &CoreConfig::default(),
            &LogiaConfig::default(),
        );
        assert!(pass_out.passed_through);
        assert!(pass_out.carved_hole);
        assert_eq!(pass_out.final_damage, 0.0);

        // With active Level 4 Armament Haki, strike bypasses Logia intangibility at 100%
        let hit = CombatResolver::resolve_attack(
            &mut atk_fruit,
            &mut atk_haki,
            &mut def_fruit,
            None,
            Some(&mut def_logia),
            &mut def_cuffs,
            &mut def_combat,
            0,
            10.0,
            GrandLineDamageType::DevilFruit,
            None,
            false,
            1,
            (0.0, 0.0),
            25,
            false,
            &CoreConfig::default(),
            &haki_cfg,
            &LogiaConfig::default(),
            &SeastoneCombatConfig::default(),
        );
        assert!(hit.final_hp_damage > 10.0);
        assert_eq!(
            hit.logia.and_then(|l| l.bypass),
            Some(LogiaBypassResult {
                source: LogiaBypassSource::ArmamentHaki,
                fraction: 1.0,
            })
        );

        // Bare-fist training unlocks Haki
        let mut trainee = PlayerHakiData::default();
        let (xp, _) = HakiTraining::on_fist_hit(&mut trainee, 20.0, 100, &haki_cfg);
        assert!(trainee.unlocked);
        assert!(xp > 0.0);
    }

    #[test]
    fn poneglyph_cipher_assembly_and_decipher_desk_puzzle() {
        let perm = Cipher::permutation(12345);
        assert!(Cipher::is_bijective(&perm));

        let frags: Vec<RubbingFragment> = (0..9)
            .map(|cell| RubbingFragment {
                poneglyph: "devilfruits:seastone_tools".into(),
                cell,
            })
            .collect();
        let bp = RubbingFragment::assemble_blueprint(&frags, 999).expect("valid 9-cell assembly");

        let reg = PoneglyphRegistry::builtin();
        let cfg = PoneglyphConfig {
            cipher_mode: CipherMode::Worldbound,
            rounds: 2,
            ..Default::default()
        };
        let mut lit = PoneglyphLiteracy { level: 5, xp: 500 };
        let mut desk = DecipherDeskSession {
            container_id: 1,
            input: Some(bp),
            has_primer: true,
            ..Default::default()
        };
        desk.ensure_rounds(42, lit, &reg, &cfg);
        assert_eq!(desk.rounds.len(), 2);

        let r0_correct = desk.rounds[0].correct as i32;
        assert_eq!(
            desk.click_button(r0_correct, 42, &mut lit, &reg, 10, &cfg),
            Some(PuzzleOutcome::Correct)
        );
        let r1_correct = desk.rounds[1].correct as i32;
        assert_eq!(
            desk.click_button(r1_correct, 42, &mut lit, &reg, 20, &cfg),
            Some(PuzzleOutcome::Solved)
        );
        assert!(desk.output.is_some());
        assert_eq!(
            desk.output.as_ref().unwrap().knowledge,
            "devilfruits:seastone_tools"
        );
    }

    #[test]
    fn gacha_casket_rolling_and_delivery() {
        let mut rt = GrandLineRuntime::new();
        let out = rt.caskets.open(
            P1,
            true,
            true,
            &mut rt.world_fruits,
            777,
            100,
            &rt.core_config,
            &rt.gacha_config,
        );
        let (rolled_fruit, deliver_at) = match out {
            CasketOpenOutcome::Opened {
                fruit,
                deliver_at_tick,
                ..
            } => (fruit, deliver_at_tick),
            other => panic!("expected Opened, got {other:?}"),
        };
        assert!(!rt.world_fruits.is_free(rolled_fruit));
        let delivered = rt.caskets.tick_deliveries(deliver_at);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].1.fruit, rolled_fruit);
    }

    #[test]
    fn network_packet_binary_and_json_roundtrip() {
        let pkt = GrandLinePacket::C2S(C2SPayload::HakiInput {
            action: HakiInputAction::Select,
            mask: HakiPart::RightFist.bit() | HakiPart::Weapon.bit(),
        });
        let bytes = pkt.to_bytes().expect("encode bytes");
        let decoded = GrandLinePacket::from_bytes(&bytes).expect("decode bytes");
        assert_eq!(pkt, decoded);

        let json = pkt.to_json().expect("encode json");
        let decoded_json = GrandLinePacket::from_json(&json).expect("decode json");
        assert_eq!(pkt, decoded_json);
    }

    #[test]
    fn commands_pirates_world_fx_and_ui_view_models() {
        let mut rt = GrandLineRuntime::new();
        assert!(rt.handle_command(P1, "/devilfruit give gomu_gomu").success);
        assert!(rt.handle_command(P1, "/devilfruit setmastery 6").success);
        assert!(rt.handle_command(P1, "/haki level 8").success);
        assert!(rt.handle_command(P1, "/devilfruit cast pistol").success);
        assert!(
            rt.handle_command(P1, "/devilfruit mob pirate captain 0 crew")
                .success
        );
        assert_eq!(rt.pirates.len(), 4); // Captain + 3 crewmates

        // Tick runtime and verify HUD & iOS touch state
        rt.tick();
        let hud = rt.hud_view_model(P1, true).expect("hud view model");
        assert!(hud.has_fruit);
        assert_eq!(hud.mastery, 6);
        assert_eq!(hud.haki.level, 8);
        assert!(hud.haki.ryuo_unlocked);
        assert_eq!(hud.wheel.gears.len(), 5);

        let mut touch = IosTouchControlsState::default();
        touch.sync_from_hud(&hud, false);
        assert!(touch.cast_button.visible);
        assert!(touch.haki_button.visible);
        assert!(touch.on_touch_down(IosTouchButton::CastAbility));
        touch.tick_holds();
        assert_eq!(touch.on_touch_up(IosTouchButton::CastAbility), Some(1));

        // Targeting cone check
        let cands = [TargetCandidate {
            id: P2,
            center: DVec3::new(0.0, 0.0, 5.0),
            width: 0.6,
            height: 1.8,
            knockback_resistance: 0.0,
        }];
        let aimed =
            CombatTargeting::aimed(&cands, P1, DVec3::ZERO, DVec3::Z, 10.0, 35.0_f64.to_radians());
        assert_eq!(aimed.map(|c| c.id), Some(P2));
    }
}
