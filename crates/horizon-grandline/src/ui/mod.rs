//! UI view-models for Ability Wheel, HUD ability bar & cooldowns, Armament Haki
//! meter & 9-part paper-doll selector, Decipher Desk screen, Gacha Casket
//! reveal overlay, and iOS touch button layout & gesture states.
//!
//! Mirrors `client/hud/{AbilityHud,HakiHud,FormBar,ColdOverlay}.java`,
//! `client/screen/{AbilityWheelScreen,GearWheelScreen,HakiPartScreen,DecipherDeskScreen}.java`,
//! and Horizon Pocket Edition's iOS touch controls.

use serde::{Deserialize, Serialize};

use crate::core::ability::AbilityRuntime;
use crate::core::cold::ColdState;
use crate::core::form::FruitFormDef;
use crate::core::fruit::{FruitId, FruitTheme};
use crate::core::gear::{Gear2Config, GearDefinition, GearRegistry, GearState};
use crate::core::haki::{HakiConfig, HakiPart, HakiParts, HakiTier, PlayerHakiData};
use crate::core::logia::{LogiaData, LogiaState};
use crate::core::poneglyph::{
    Cipher, CipherMode, DESK_STATUS_ACTIVE, DecipherDeskSession, PoneglyphConfig,
    PoneglyphLiteracy, PoneglyphRegistry,
};
use crate::core::rules::{Mastery, PlayerFruitData};

/// Single slot in the HUD ability bar or radial Ability Wheel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilitySlotViewModel {
    pub index: usize,
    pub ability_name: String,
    pub is_selected: bool,
    pub is_ultimate: bool,
    pub is_unlocked: bool,
    pub is_toggled_on: bool,
    pub cooldown_remaining_ticks: i64,
    pub cooldown_fraction: f32,
}

/// Radial Ability Wheel & Gear Wheel view-model (`AbilityWheelScreen.java` & `GearWheelScreen.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilityWheelViewModel {
    pub visible: bool,
    pub fruit: Option<FruitId>,
    pub active_form: String,
    pub available_forms: Vec<String>,
    pub slots: Vec<AbilitySlotViewModel>,
    pub gears: Vec<GearDefinition>,
    pub active_gear: String,
    pub gear2_adaptation_fraction: f32,
}

impl AbilityWheelViewModel {
    pub fn build(
        player: &PlayerFruitData,
        ability_rt: &AbilityRuntime,
        gear_state: &GearState,
        gear2_cfg: &Gear2Config,
        visible: bool,
        now: i64,
    ) -> Self {
        let Some(fruit) = player.fruit_id else {
            return Self {
                visible: false,
                fruit: None,
                active_form: "base".into(),
                available_forms: Vec::new(),
                slots: Vec::new(),
                gears: Vec::new(),
                active_gear: gear_state.active_gear.0.clone(),
                gear2_adaptation_fraction: 0.0,
            };
        };

        let meta = fruit.meta();
        let forms = FruitFormDef::builtin_forms(fruit);
        let active_form_def = forms
            .iter()
            .find(|f| f.id.name == player.active_form)
            .or_else(|| forms.first());

        let ability_names: Vec<String> = active_form_def
            .map(|f| f.abilities.clone())
            .unwrap_or_else(|| meta.base_abilities.iter().map(|s| (*s).to_string()).collect());
        let ult_name = active_form_def
            .and_then(|f| f.ultimate_ability.clone())
            .or_else(|| meta.ultimate_ability.map(str::to_string));

        let slots = ability_names
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let cd = player.cooldowns.get(&name).copied();
                let rem = cd.map(|c| c.remaining(now)).unwrap_or(0);
                let frac = cd.map(|c| c.fraction(now)).unwrap_or(0.0);
                let is_selected = player.selected_ability.as_deref() == Some(name.as_str());
                let is_ultimate = ult_name.as_deref() == Some(name.as_str());
                let is_toggled_on = ability_rt.active_toggles.iter().any(|t| t == &name);
                AbilitySlotViewModel {
                    index,
                    ability_name: name,
                    is_selected,
                    is_ultimate,
                    is_unlocked: true,
                    is_toggled_on,
                    cooldown_remaining_ticks: rem,
                    cooldown_fraction: frac,
                }
            })
            .collect();

        Self {
            visible,
            fruit: Some(fruit),
            active_form: player.active_form.clone(),
            available_forms: meta.forms.iter().map(|s| (*s).to_string()).collect(),
            slots,
            gears: GearRegistry::for_fruit(fruit),
            active_gear: gear_state.active_gear.0.clone(),
            gear2_adaptation_fraction: gear_state.gear2_adaptation.fraction(gear2_cfg),
        }
    }
}

/// Armament Haki meter & 9-part body coating view-model (`HakiHud.java` & `HakiPartScreen.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HakiMeterViewModel {
    pub unlocked: bool,
    pub active: bool,
    pub level: u32,
    pub tier: HakiTier,
    pub xp_fraction: f32,
    pub spirit: f32,
    pub max_spirit: f32,
    pub spirit_fraction: f32,
    pub effective_parts_mask: u16,
    pub allowed_parts_mask: u16,
    pub body_coverage: f32,
    pub logia_reach_fraction: f32,
    pub ryuo_unlocked: bool,
    pub ryuo_charge_fraction: f32,
}

impl HakiMeterViewModel {
    pub fn build(data: &PlayerHakiData, now: i64, cfg: &HakiConfig) -> Self {
        let max_spirit = data.max_spirit(cfg);
        let spirit_fraction = if max_spirit <= 0.0 {
            0.0
        } else {
            (data.spirit / max_spirit).clamp(0.0, 1.0)
        };
        let need_xp = PlayerHakiData::xp_to_next(data.level, cfg);
        let xp_fraction = if data.level >= 10 {
            1.0
        } else if need_xp <= 0.0 {
            0.0
        } else {
            (data.xp / need_xp).clamp(0.0, 1.0)
        };
        let ryuo_charge_fraction = if let Some(since) = data.ryuo_charging_since {
            ((now - since).max(0) as f32 / cfg.ryuo_charge_ticks.max(1) as f32).clamp(0.0, 1.0)
        } else {
            data.ryuo_armed_fraction
        };
        let mask = data.effective_parts();
        Self {
            unlocked: data.unlocked,
            active: data.active,
            level: data.level,
            tier: data.tier(),
            xp_fraction,
            spirit: data.spirit,
            max_spirit,
            spirit_fraction,
            effective_parts_mask: mask,
            allowed_parts_mask: data.tier().allowed_parts(),
            body_coverage: HakiParts::coverage(mask),
            logia_reach_fraction: data.logia_reach(cfg),
            ryuo_unlocked: data.can_use_ryuo(),
            ryuo_charge_fraction,
        }
    }

    pub const fn is_part_coated(&self, part: HakiPart) -> bool {
        (self.effective_parts_mask & part.bit()) != 0
    }
}

/// Top-level HUD overlay view-model (`AbilityHud.java` + `FormBar.java` + `ColdOverlay.java`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrandLineHudViewModel {
    pub has_fruit: bool,
    pub fruit_display_name: Option<String>,
    pub theme: Option<FruitTheme>,
    pub mastery: u32,
    pub mastery_progress: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub energy_fraction: f32,
    pub active_form: String,
    pub charge_progress: Option<f32>,
    pub logia_state: Option<LogiaState>,
    pub chill_stacks: i32,
    pub frozen_struggle_progress: Option<f32>,
    pub suppressed: bool,
    pub wheel: AbilityWheelViewModel,
    pub haki: HakiMeterViewModel,
}

impl GrandLineHudViewModel {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        player: &PlayerFruitData,
        ability_rt: &AbilityRuntime,
        gear_state: &GearState,
        haki: &PlayerHakiData,
        logia: Option<&LogiaData>,
        cold: &ColdState,
        suppressed: bool,
        wheel_open: bool,
        now: i64,
        haki_cfg: &HakiConfig,
        gear2_cfg: &Gear2Config,
    ) -> Self {
        let meta = player.fruit_id.map(|f| f.meta());
        let max_energy = player.max_energy();
        let energy_fraction = if max_energy <= 0.0 {
            0.0
        } else {
            (player.energy / max_energy).clamp(0.0, 1.0)
        };
        let charge_progress = ability_rt.charging.as_ref().map(|c| c.progress(now));
        let frozen_struggle_progress = cold.is_frozen(now).then(|| cold.struggle_progress());

        Self {
            has_fruit: player.has_fruit(),
            fruit_display_name: meta.as_ref().map(|m| m.display_name.to_string()),
            theme: meta.as_ref().map(|m| m.theme),
            mastery: player.mastery,
            mastery_progress: Mastery::progress_fraction(player.mastery, player.xp),
            energy: player.energy,
            max_energy,
            energy_fraction,
            active_form: player.active_form.clone(),
            charge_progress,
            logia_state: logia.map(|l| l.state),
            chill_stacks: cold.chill_at(now),
            frozen_struggle_progress,
            suppressed,
            wheel: AbilityWheelViewModel::build(
                player,
                ability_rt,
                gear_state,
                gear2_cfg,
                wheel_open,
                now,
            ),
            haki: HakiMeterViewModel::build(haki, now, haki_cfg),
        }
    }
}

/// Decipher Desk screen view-model (`DecipherDeskScreen.java`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecipherDeskViewModel {
    pub container_id: i32,
    pub status: i32,
    pub round: usize,
    pub total_rounds: usize,
    pub clue_plain: String,
    pub encoded_options: Vec<String>,
    pub mistakes: i32,
    pub max_mistakes: i32,
    pub lock_ticks_remaining: i32,
    pub hint_available: bool,
    pub reveal_available: bool,
    pub revealed_index: Option<usize>,
    pub literacy_level: u32,
    pub literacy_xp: i32,
    pub has_primer: bool,
}

impl DecipherDeskViewModel {
    pub fn build(
        desk: &DecipherDeskSession,
        literacy: PoneglyphLiteracy,
        registry: &PoneglyphRegistry,
        world_seed: i64,
        now: i64,
        cfg: &PoneglyphConfig,
    ) -> Self {
        let status = desk.status(literacy, registry);
        if status != DESK_STATUS_ACTIVE {
            return Self {
                container_id: desk.container_id,
                status,
                round: 0,
                total_rounds: cfg.rounds,
                clue_plain: String::new(),
                encoded_options: Vec::new(),
                mistakes: 0,
                max_mistakes: cfg.max_mistakes,
                lock_ticks_remaining: 0,
                hint_available: false,
                reveal_available: false,
                revealed_index: None,
                literacy_level: literacy.level,
                literacy_xp: literacy.xp,
                has_primer: desk.has_primer,
            };
        }

        let Some(enc) = &desk.input else {
            return Self {
                container_id: desk.container_id,
                status: 0,
                round: 0,
                total_rounds: cfg.rounds,
                clue_plain: String::new(),
                encoded_options: Vec::new(),
                mistakes: 0,
                max_mistakes: cfg.max_mistakes,
                lock_ticks_remaining: 0,
                hint_available: false,
                reveal_available: false,
                revealed_index: None,
                literacy_level: literacy.level,
                literacy_xp: literacy.xp,
                has_primer: desk.has_primer,
            };
        };

        let perm = Cipher::active(cfg.cipher_mode, world_seed);
        let r_idx = enc.progress.round.min(desk.rounds.len().saturating_sub(1));
        let round = desk.rounds.get(r_idx);
        let clue_plain = round.map(|r| r.clue.clone()).unwrap_or_default();
        let encoded_options = round
            .map(|r| {
                r.options
                    .iter()
                    .map(|o| Cipher::encode(o, &perm))
                    .collect()
            })
            .unwrap_or_default();
        let revealed_index = if enc.progress.has_flag(2) {
            round.map(|r| r.correct)
        } else {
            None
        };
        let lock_ticks_remaining = (enc.progress.lock_until - now).max(0) as i32;

        Self {
            container_id: desk.container_id,
            status,
            round: enc.progress.round,
            total_rounds: desk.rounds.len().max(cfg.rounds),
            clue_plain,
            encoded_options,
            mistakes: enc.progress.mistakes,
            max_mistakes: cfg.max_mistakes,
            lock_ticks_remaining,
            hint_available: literacy.has_hint() && !enc.progress.has_flag(1),
            reveal_available: literacy.has_reveal() && !enc.progress.has_flag(2),
            revealed_index,
            literacy_level: literacy.level,
            literacy_xp: literacy.xp,
            has_primer: desk.has_primer || cfg.cipher_mode == CipherMode::Standard,
        }
    }
}

/// iOS touch button identifiers for Horizon Pocket Edition's Grand Line HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IosTouchButton {
    /// Primary ability cast / hold button (bottom-right cluster).
    CastAbility,
    /// Radial Ability Wheel hold-to-open button.
    AbilityWheel,
    /// Transformation Form / Gear toggle button.
    FormToggle,
    /// Armament Haki coating / hold-for-Ryuo button.
    HakiToggle,
    /// Logia elemental body / elemental dash button.
    LogiaReflex,
    /// Contextual struggle mash button (shown when Frozen or Restrained).
    StruggleMash,
}

/// Per-button touch state on iOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TouchButtonState {
    pub visible: bool,
    pub enabled: bool,
    pub pressed: bool,
    pub hold_ticks: i32,
    pub cooldown_fraction: f32,
}

/// Complete iOS touch controls layout & gesture state for Horizon Pocket Edition.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IosTouchControlsState {
    pub cast_button: TouchButtonState,
    pub wheel_button: TouchButtonState,
    pub form_button: TouchButtonState,
    pub haki_button: TouchButtonState,
    pub logia_button: TouchButtonState,
    pub struggle_button: TouchButtonState,
    pub wheel_drag_index: Option<usize>,
}

impl IosTouchControlsState {
    /// Synchronizes iOS touch button visibility, enabled state, and cooldown rings from HUD state.
    pub fn sync_from_hud(&mut self, hud: &GrandLineHudViewModel, is_restrained_or_frozen: bool) {
        let selected_cd = hud
            .wheel
            .slots
            .iter()
            .find(|s| s.is_selected)
            .map(|s| s.cooldown_fraction)
            .unwrap_or(0.0);

        self.cast_button.visible = hud.has_fruit;
        self.cast_button.enabled = hud.has_fruit && !hud.suppressed && !is_restrained_or_frozen;
        self.cast_button.cooldown_fraction = selected_cd;

        self.wheel_button.visible = hud.has_fruit;
        self.wheel_button.enabled = hud.has_fruit;

        let has_multiple_forms =
            hud.wheel.available_forms.len() > 1 || !hud.wheel.gears.is_empty();
        self.form_button.visible = hud.has_fruit && has_multiple_forms;
        self.form_button.enabled =
            hud.has_fruit && has_multiple_forms && !hud.suppressed && !is_restrained_or_frozen;

        self.haki_button.visible = hud.haki.unlocked;
        self.haki_button.enabled = hud.haki.unlocked && !is_restrained_or_frozen;
        self.haki_button.cooldown_fraction = 1.0 - hud.haki.spirit_fraction;

        let is_logia = hud.logia_state.is_some();
        self.logia_button.visible = is_logia;
        self.logia_button.enabled = is_logia && !hud.suppressed && !is_restrained_or_frozen;

        self.struggle_button.visible = is_restrained_or_frozen;
        self.struggle_button.enabled = is_restrained_or_frozen;
    }

    pub fn on_touch_down(&mut self, button: IosTouchButton) -> bool {
        let b = self.button_mut(button);
        if !b.visible || !b.enabled {
            return false;
        }
        b.pressed = true;
        b.hold_ticks = 0;
        true
    }

    pub fn on_touch_up(&mut self, button: IosTouchButton) -> Option<i32> {
        let b = self.button_mut(button);
        if !b.pressed {
            return None;
        }
        b.pressed = false;
        let held = b.hold_ticks;
        b.hold_ticks = 0;
        Some(held)
    }

    pub fn tick_holds(&mut self) {
        for b in [
            &mut self.cast_button,
            &mut self.wheel_button,
            &mut self.form_button,
            &mut self.haki_button,
            &mut self.logia_button,
            &mut self.struggle_button,
        ] {
            if b.pressed {
                b.hold_ticks += 1;
            }
        }
    }

    fn button_mut(&mut self, button: IosTouchButton) -> &mut TouchButtonState {
        match button {
            IosTouchButton::CastAbility => &mut self.cast_button,
            IosTouchButton::AbilityWheel => &mut self.wheel_button,
            IosTouchButton::FormToggle => &mut self.form_button,
            IosTouchButton::HakiToggle => &mut self.haki_button,
            IosTouchButton::LogiaReflex => &mut self.logia_button,
            IosTouchButton::StruggleMash => &mut self.struggle_button,
        }
    }
}
