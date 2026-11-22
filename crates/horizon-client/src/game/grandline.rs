//! Grand Line 0.4.0 (`devilfruits`) native integration for Horizon Pocket Edition iOS.
//!
//! Connects [`horizon_grandline::GrandLineRuntime`] into [`super::Game`]:
//! - Lists **Grand Line 0.4.0** (`id = "devilfruits"`) and its 5 customizable keys in `Options > Mods`.
//! - Exposes in-game settings (`VFX Quality`, `HUD Overlay`, `iOS Touch Buttons`) in `Options > Mods > Grand Line 0.4.0`.
//! - Syncs the 5 iOS touch buttons (`DF Cast`, `DF Next`, `DF Wheel`, `DF Form`, `DF Haki`) into `TouchControls`.
//! - Dispatches all 26 `/devilfruit` (`/df`), `/haki`, `/ability`, `/poneglyph`, and `/bounty` commands.
//! - Ticks [`GrandLineRuntime`] at 20 TPS, syncing player position, look direction, held items, and body modifiers.
//! - Draws the Grand Line HUD overlay (Devil Fruit energy bar, Haki spirit meter, 5 ability slots with cooldowns,
//!   active Form/Gear/Logia badge, and the Ability Wheel).

use std::cell::RefCell;

use glam::DVec3;
use horizon_client::hud::HudBatch;
use horizon_grandline::{
    GrandLineRuntime,
    core::{
        form::FruitFormDef,
        fruit::FruitId,
        logia::LogiaState,
        network::{
            AbilityInputAction, C2SPayload, FormInputAction, HakiInputAction, LogiaInputAction,
        },
        rules::{PlayerFruitData, PlayerUuid},
    },
};
use horizon_world::grandline_hooks::EntityBodyModifiers;

use super::{Game, mod_keys, native_hooks::NativeMod};

pub const ID: &str = "devilfruits";
pub const VERSION: &str = "0.4.0";
pub const DESCRIPTION: &str = "16 Devil Fruits (Paramecia, Logia, Mythical Zoan), Armament Haki & Ryuo, Seastone, Poneglyphs & Pirates.";

const LOCAL_PLAYER_UUID: PlayerUuid = [0xDF; 16];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfxQualitySetting {
    High,
    Medium,
    Low,
    Off,
}

impl VfxQualitySetting {
    fn from_stored(s: Option<&str>) -> Self {
        match s {
            Some("medium") => Self::Medium,
            Some("low") => Self::Low,
            Some("off") => Self::Off,
            _ => Self::High,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Off => "off",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::High => "High (Metal 2.1 Bloom + Refract)",
            Self::Medium => "Medium (Half-Res Bloom)",
            Self::Low => "Low (Mobile Battery Saver)",
            Self::Off => "Off",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::High => Self::Medium,
            Self::Medium => Self::Low,
            Self::Low => Self::Off,
            Self::Off => Self::High,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrandLineClientSettings {
    pub vfx_quality: VfxQualitySetting,
    pub hud_enabled: bool,
    pub touch_buttons: bool,
}

impl Default for GrandLineClientSettings {
    fn default() -> Self {
        Self {
            vfx_quality: VfxQualitySetting::High,
            hud_enabled: true,
            touch_buttons: true,
        }
    }
}

impl GrandLineClientSettings {
    fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let vfx_quality = VfxQualitySetting::from_stored(get("vfx_quality").as_deref());
        let hud_enabled = get("hud_enabled").as_deref() != Some("false");
        let touch_buttons = get("touch_buttons").as_deref() != Some("false");
        Self {
            vfx_quality,
            hud_enabled,
            touch_buttons,
        }
    }

    fn stored(self) -> [(&'static str, &'static str); 3] {
        [
            ("vfx_quality", self.vfx_quality.as_str()),
            (
                "hud_enabled",
                if self.hud_enabled { "true" } else { "false" },
            ),
            (
                "touch_buttons",
                if self.touch_buttons { "true" } else { "false" },
            ),
        ]
    }

    fn labels(self) -> [String; 3] {
        [
            format!("VFX Quality: {}", self.vfx_quality.label()),
            format!(
                "Grand Line HUD: {}",
                if self.hud_enabled { "ON" } else { "OFF" }
            ),
            format!(
                "iOS Touch Buttons: {}",
                if self.touch_buttons { "ON" } else { "OFF" }
            ),
        ]
    }

    fn click(&mut self, button: usize) {
        match button {
            0 => self.vfx_quality = self.vfx_quality.next(),
            1 => self.hud_enabled = !self.hud_enabled,
            2 => self.touch_buttons = !self.touch_buttons,
            _ => {}
        }
    }
}

struct ClientState {
    runtime: GrandLineRuntime,
    settings: Option<GrandLineClientSettings>,
    wheel_open: bool,
    body_modifiers: EntityBodyModifiers,
    last_synced_touch: Option<(bool, [String; 5])>,
}

impl Default for ClientState {
    fn default() -> Self {
        let mut runtime = GrandLineRuntime::new();
        let _ = runtime.player_mut(LOCAL_PLAYER_UUID);
        Self {
            runtime,
            settings: None,
            wheel_open: false,
            body_modifiers: EntityBodyModifiers::VANILLA,
            last_synced_touch: None,
        }
    }
}

thread_local! {
    static STATE: RefCell<ClientState> = RefCell::new(ClientState::default());
}

fn current_settings(game: &Game<'_>) -> GrandLineClientSettings {
    STATE.with_borrow_mut(|s| {
        *s.settings
            .get_or_insert_with(|| GrandLineClientSettings::read(|k| game.native_setting(ID, k)))
    })
}

/// Lists Grand Line 0.4.0 in `Options > Mods` with its 5 customizable keybinds.
pub(super) fn list_mod(mods: &mut Vec<NativeMod>) {
    if mods.iter().any(|m| m.id == ID) {
        return;
    }
    mods.push(NativeMod {
        id: ID.into(),
        title: format!("Grand Line {VERSION}"),
        description: DESCRIPTION.into(),
        keys: vec![
            [
                mod_keys::GRANDLINE_CAST.name.into(),
                "Use / Hold Devil Fruit Ability".into(),
                mod_keys::GRANDLINE_CAST.default.into(),
            ],
            [
                mod_keys::GRANDLINE_NEXT.name.into(),
                "Next Devil Fruit Ability Slot".into(),
                mod_keys::GRANDLINE_NEXT.default.into(),
            ],
            [
                mod_keys::GRANDLINE_WHEEL.name.into(),
                "Devil Fruit Ability Wheel".into(),
                mod_keys::GRANDLINE_WHEEL.default.into(),
            ],
            [
                mod_keys::GRANDLINE_FORM.name.into(),
                "Form / Gear / Elemental Body".into(),
                mod_keys::GRANDLINE_FORM.default.into(),
            ],
            [
                mod_keys::GRANDLINE_HAKI.name.into(),
                "Armament Haki / Ryuo Coating".into(),
                mod_keys::GRANDLINE_HAKI.default.into(),
            ],
        ],
    });
}

/// Populates `Options > Mods > Grand Line 0.4.0` settings buttons.
pub(super) fn settings_page(game: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    let s = current_settings(game);
    buttons.extend(s.labels());
}

/// Handles a click in `Options > Mods > Grand Line 0.4.0`.
pub(super) fn settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    let mut s = current_settings(game);
    s.click(button);
    STATE.with_borrow_mut(|st| {
        st.settings = Some(s);
        st.last_synced_touch = None;
    });
    for (k, v) in s.stored() {
        game.native_set_setting(ID, k, v);
    }
    sync_touch_buttons(game);
}

/// Synchronizes the 5 Grand Line iOS touch buttons into `TouchControls::settings.custom`.
pub(super) fn sync_touch_buttons(game: &mut Game<'_>) {
    let mod_on = game.native_on(ID);
    let settings = current_settings(game);
    let enabled = mod_on && settings.touch_buttons;
    let keys = [
        game.native_mod_key(
            ID,
            mod_keys::GRANDLINE_CAST.name,
            mod_keys::GRANDLINE_CAST.default,
        ),
        game.native_mod_key(
            ID,
            mod_keys::GRANDLINE_NEXT.name,
            mod_keys::GRANDLINE_NEXT.default,
        ),
        game.native_mod_key(
            ID,
            mod_keys::GRANDLINE_WHEEL.name,
            mod_keys::GRANDLINE_WHEEL.default,
        ),
        game.native_mod_key(
            ID,
            mod_keys::GRANDLINE_FORM.name,
            mod_keys::GRANDLINE_FORM.default,
        ),
        game.native_mod_key(
            ID,
            mod_keys::GRANDLINE_HAKI.name,
            mod_keys::GRANDLINE_HAKI.default,
        ),
    ];

    let unchanged = STATE.with_borrow(|st| {
        st.last_synced_touch
            .as_ref()
            .is_some_and(|(last_en, last_keys)| *last_en == enabled && *last_keys == keys)
    });
    if unchanged {
        return;
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let bindings = [
            (keys[0].as_str(), "DF Cast", [0.88, 0.46, 0.11]),
            (keys[1].as_str(), "DF Next", [0.78, 0.46, 0.095]),
            (keys[2].as_str(), "DF Wheel", [0.78, 0.34, 0.095]),
            (keys[3].as_str(), "DF Form", [0.88, 0.34, 0.095]),
            (keys[4].as_str(), "DF Haki", [0.94, 0.22, 0.095]),
        ];
        game.mobile.controls.sync_grandline_buttons(enabled, &bindings);
    }
    STATE.with_borrow_mut(|st| {
        st.last_synced_touch = Some((enabled, keys));
    });
}

/// Registers Grand Line chat command names for auto-completion.
pub(super) fn register_commands(commands: &mut Vec<&'static str>) {
    for cmd in [
        "devilfruit",
        "df",
        "haki",
        "ability",
        "poneglyph",
        "bounty",
    ] {
        if !commands.contains(&cmd) {
            commands.push(cmd);
        }
    }
}

fn active_ability_list(fruit: &PlayerFruitData) -> Vec<String> {
    let Some(fid) = fruit.fruit_id else {
        return Vec::new();
    };
    let forms = FruitFormDef::builtin_forms(fid);
    if let Some(form_def) = forms
        .iter()
        .find(|f| f.id.name == fruit.active_form)
        .or_else(|| forms.first())
    {
        return form_def.abilities.clone();
    }
    fid.meta()
        .base_abilities
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Executes a Grand Line chat command (`/devilfruit`, `/df`, `/haki`, `/ability`, `/poneglyph`, `/bounty`).
pub(super) fn on_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    if *ran || !game.native_on(ID) {
        return;
    }
    let trimmed = command.trim();
    let head = trimmed.split_whitespace().next().unwrap_or("");
    if !matches!(
        head,
        "devilfruit" | "df" | "haki" | "ability" | "poneglyph" | "bounty"
    ) {
        return;
    }
    sync_player_into_runtime(game);
    let resp = STATE.with_borrow_mut(|st| st.runtime.handle_command(LOCAL_PLAYER_UUID, trimmed));
    *ran = true;
    game.native_chat(&format!("[Grand Line] {}", resp.message));
}

/// Handles Grand Line key press / release events (keyboard, gamepad, or iOS touch button).
pub(super) fn on_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !game.native_on(ID) {
        return;
    }
    if pressed && !game.native_playing() {
        return;
    }

    let cast_key = game.native_mod_key(
        ID,
        mod_keys::GRANDLINE_CAST.name,
        mod_keys::GRANDLINE_CAST.default,
    );
    let next_key = game.native_mod_key(
        ID,
        mod_keys::GRANDLINE_NEXT.name,
        mod_keys::GRANDLINE_NEXT.default,
    );
    let wheel_key = game.native_mod_key(
        ID,
        mod_keys::GRANDLINE_WHEEL.name,
        mod_keys::GRANDLINE_WHEEL.default,
    );
    let form_key = game.native_mod_key(
        ID,
        mod_keys::GRANDLINE_FORM.name,
        mod_keys::GRANDLINE_FORM.default,
    );
    let haki_key = game.native_mod_key(
        ID,
        mod_keys::GRANDLINE_HAKI.name,
        mod_keys::GRANDLINE_HAKI.default,
    );

    if key != cast_key
        && key != next_key
        && key != wheel_key
        && key != form_key
        && key != haki_key
    {
        return;
    }

    sync_player_into_runtime(game);

    if key == cast_key {
        let action = if pressed {
            AbilityInputAction::Press
        } else {
            AbilityInputAction::Release
        };
        let impulse = STATE.with_borrow_mut(|st| {
            let ability = st.runtime.player(LOCAL_PLAYER_UUID).and_then(|p| {
                p.fruit
                    .selected_ability
                    .clone()
                    .or_else(|| active_ability_list(&p.fruit).into_iter().next())
            });
            if let Some(ability) = ability {
                let _ = st.runtime.handle_payload(
                    LOCAL_PLAYER_UUID,
                    C2SPayload::AbilityInput { action, ability },
                );
            }
            st.runtime
                .player(LOCAL_PLAYER_UUID)
                .map(|p| p.velocity)
        });
        if let Some(vel) = impulse {
            apply_runtime_impulse(game, vel);
        }
    } else if key == next_key && pressed {
        let msg = STATE.with_borrow_mut(|st| {
            let (next_idx, next_ab) = {
                let p = st.runtime.player(LOCAL_PLAYER_UUID)?;
                let abilities = active_ability_list(&p.fruit);
                if abilities.is_empty() {
                    return None;
                }
                let cur = p
                    .fruit
                    .selected_ability
                    .as_deref()
                    .and_then(|s| abilities.iter().position(|a| a == s))
                    .unwrap_or(abilities.len().saturating_sub(1));
                let next_idx = (cur + 1) % abilities.len();
                (next_idx, abilities[next_idx].clone())
            };
            let _ = st.runtime.handle_payload(
                LOCAL_PLAYER_UUID,
                C2SPayload::AbilityInput {
                    action: AbilityInputAction::Select,
                    ability: next_ab.clone(),
                },
            );
            Some(format!("Slot {}: {next_ab}", next_idx + 1))
        });
        if let Some(m) = msg {
            game.native_chat(&format!("[Grand Line] {m}"));
        }
    } else if key == wheel_key && pressed {
        STATE.with_borrow_mut(|st| {
            st.wheel_open = !st.wheel_open;
        });
    } else if key == form_key && pressed {
        let status = STATE.with_borrow_mut(|st| {
            let is_logia = st
                .runtime
                .player(LOCAL_PLAYER_UUID)
                .and_then(|p| p.fruit.fruit_id)
                .is_some_and(FruitId::is_logia);
            if is_logia {
                let _ = st.runtime.handle_payload(
                    LOCAL_PLAYER_UUID,
                    C2SPayload::LogiaInput {
                        action: LogiaInputAction::ElementalToggle,
                    },
                );
                st.runtime.player(LOCAL_PLAYER_UUID).map(|p| {
                    format!(
                        "Logia Elemental Body: {}",
                        if p.logia.state == LogiaState::ElementalBody {
                            "ON"
                        } else {
                            "OFF"
                        }
                    )
                })
            } else {
                let _ = st.runtime.handle_payload(
                    LOCAL_PLAYER_UUID,
                    C2SPayload::FormInput {
                        action: FormInputAction::Cycle,
                        form: String::new(),
                    },
                );
                st.runtime
                    .player(LOCAL_PLAYER_UUID)
                    .map(|p| format!("Active Form: {}", p.fruit.active_form))
            }
        });
        if let Some(s) = status {
            game.native_chat(&format!("[Grand Line] {s}"));
        }
    } else if key == haki_key && pressed {
        let status = STATE.with_borrow_mut(|st| {
            let haki_cfg = st.runtime.haki_config;
            let p = st.runtime.player_mut(LOCAL_PLAYER_UUID);
            if !p.haki.unlocked {
                p.haki.unlocked = true;
                p.haki.level = p.haki.level.max(1);
                p.haki.spirit = p.haki.max_spirit(&haki_cfg);
            }
            let _ = st.runtime.handle_payload(
                LOCAL_PLAYER_UUID,
                C2SPayload::HakiInput {
                    action: HakiInputAction::Toggle,
                    mask: 0,
                },
            );
            st.runtime.player(LOCAL_PLAYER_UUID).map(|p| {
                format!(
                    "Busoshoku Haki: {} (Lv.{}, Spirit {:.0}/{:.0})",
                    if p.haki.active { "COATED" } else { "OFF" },
                    p.haki.level,
                    p.haki.spirit,
                    p.haki.max_spirit(&haki_cfg)
                )
            })
        });
        if let Some(s) = status {
            game.native_chat(&format!("[Grand Line] {s}"));
        }
    }
}

/// Advances the Grand Line runtime by one tick (`20 TPS`) and syncs body modifiers.
pub(super) fn on_tick(game: &mut Game<'_>) {
    sync_touch_buttons(game);
    if !game.native_on(ID) || game.player.is_none() {
        return;
    }
    sync_player_into_runtime(game);
    STATE.with_borrow_mut(|st| {
        st.runtime.tick();
        st.body_modifiers = st.runtime.body_modifiers_for(LOCAL_PLAYER_UUID);
    });
}

/// Current [`EntityBodyModifiers`] computed by Grand Line for the local player.
#[allow(dead_code)]
pub(super) fn local_body_modifiers() -> EntityBodyModifiers {
    STATE.with_borrow(|st| st.body_modifiers)
}

fn sync_player_into_runtime(game: &Game<'_>) {
    let Some(np) = game.native_player() else {
        return;
    };
    let yaw_rad = np.yaw.to_radians() as f64;
    let pitch_rad = np.pitch.to_radians() as f64;
    let look_dir = DVec3::new(
        -yaw_rad.sin() * pitch_rad.cos(),
        -pitch_rad.sin(),
        yaw_rad.cos() * pitch_rad.cos(),
    )
    .normalize_or_zero();
    let pos = DVec3::new(np.x, np.y, np.z);
    let eye_pos = pos + DVec3::new(0.0, 1.62, 0.0);
    let (on_ground, in_water, food_level, main_hand) = match game.player.as_ref() {
        Some(p) => (
            p.ground.on_ground,
            p.swimming,
            p.health.food.food as f32,
            game.held().map(|s| s.item.to_string()),
        ),
        None => (true, false, 20.0, None),
    };

    STATE.with_borrow_mut(|st| {
        let p = st.runtime.player_mut(LOCAL_PLAYER_UUID);
        p.position = pos;
        p.eye_pos = eye_pos;
        p.look_dir = if look_dir.length_squared() > 1e-6 {
            look_dir
        } else {
            DVec3::Z
        };
        p.on_ground = on_ground;
        p.water_depth = if in_water { 1.0 } else { 0.0 };
        p.food_level = food_level;
        p.main_hand_item = main_hand;
    });
}

fn apply_runtime_impulse(game: &mut Game<'_>, vel: DVec3) {
    if vel.length_squared() < 1e-6 {
        return;
    }
    if let Some(p) = game.player.as_mut() {
        p.velocity.x += vel.x;
        p.velocity.y += vel.y;
        p.velocity.z += vel.z;
    }
    STATE.with_borrow_mut(|st| {
        st.runtime.player_mut(LOCAL_PLAYER_UUID).velocity = DVec3::ZERO;
    });
}

/// Draws the Grand Line HUD overlay (Devil Fruit status, Energy bar, Haki bar, Ability slots, and Ability Wheel).
pub(super) fn draw_hud(game: &mut Game<'_>, batch: &mut HudBatch) {
    if !game.native_on(ID) || !current_settings(game).hud_enabled {
        return;
    }
    let vm = STATE.with_borrow(|st| st.runtime.hud_view_model(LOCAL_PLAYER_UUID, st.wheel_open));
    let Some(vm) = vm else {
        return;
    };
    if !vm.has_fruit && !vm.haki.unlocked {
        return;
    }

    let x = 8.0_f32;
    let mut y = 34.0_f32;
    let bar_w = 128.0_f32;

    // Devil Fruit title + Form / Gear / Logia badge + Energy Bar
    if let Some(ref title) = vm.fruit_display_name {
        let header = format!(
            "{title} [M{} · {}]",
            vm.mastery,
            vm.active_form.to_uppercase()
        );
        game.native_fill(batch, [x - 2.0, y - 2.0, bar_w + 4.0, 24.0], 0x880B_1320);
        game.native_text(batch, &header, [x, y], 0xFFD770, true);
        y += 11.0;

        // Energy bar background & fill
        game.native_fill(batch, [x, y, bar_w, 8.0], 0xCC1A_2230);
        let fill_w = (bar_w * vm.energy_fraction.clamp(0.0, 1.0)).round();
        let gear2_active = vm.wheel.active_gear == "gear_2" || vm.active_form == "gear_2";
        let logia_reflex_active = vm.logia_state == Some(LogiaState::ElementalBody);
        let bar_color = if vm.suppressed {
            0xCC88_4444
        } else if gear2_active {
            0xCCFF_5577
        } else if logia_reflex_active {
            0xCC44_DDFF
        } else {
            0xCC2E_A8FF
        };
        if fill_w > 0.0 {
            game.native_fill(batch, [x, y, fill_w, 8.0], bar_color);
        }
        let energy_label = if vm.suppressed {
            "SEASTONE / WATER WEAKNESS".to_string()
        } else {
            format!("EN {:.0}/{:.0}", vm.energy, vm.max_energy)
        };
        game.native_text(batch, &energy_label, [x + 3.0, y], 0xF0F6FF, true);
        y += 13.0;
    }

    // Armament Haki Spirit meter
    if vm.haki.unlocked {
        game.native_fill(batch, [x, y, bar_w, 8.0], 0xCC1F_1424);
        let haki_w = (bar_w * vm.haki.spirit_fraction.clamp(0.0, 1.0)).round();
        let haki_color = if vm.haki.active {
            0xCCD4_2A5E
        } else {
            0xCC7A_3B69
        };
        if haki_w > 0.0 {
            game.native_fill(batch, [x, y, haki_w, 8.0], haki_color);
        }
        let haki_txt = format!(
            "HAKI Lv.{} {}",
            vm.haki.level,
            if vm.haki.active { "[ACTIVE]" } else { "" }
        );
        game.native_text(batch, &haki_txt, [x + 3.0, y], 0xFFE4F2, true);
        y += 12.0;
    }

    // Ability Slots
    for slot in &vm.wheel.slots {
        let name = &slot.ability_name;
        let bg = if slot.is_selected {
            0xAA1F_3B5C
        } else {
            0x7810_1824
        };
        game.native_fill(batch, [x - 2.0, y, bar_w + 4.0, 11.0], bg);
        let cd_suffix = if slot.cooldown_remaining_ticks > 0 {
            format!(
                " ({:.1}s)",
                (slot.cooldown_remaining_ticks as f32) / 20.0
            )
        } else if slot.is_toggled_on {
            " [ON]".to_string()
        } else {
            String::new()
        };
        let line = format!(
            "{}{} {}{}",
            if slot.is_selected { ">" } else { " " },
            slot.index + 1,
            name,
            cd_suffix
        );
        let color = if !slot.is_unlocked || slot.cooldown_remaining_ticks > 0 {
            0x99A3B0
        } else if slot.is_selected {
            0x55FFAA
        } else {
            0xE6EEF8
        };
        game.native_text(batch, &line, [x, y + 1.0], color, true);
        y += 12.0;
    }

    // Ability Wheel modal overlay when toggled open
    if vm.wheel.visible {
        let cx = batch.width * 0.5 - 75.0;
        let cy = batch.height * 0.35;
        let wheel_h = (vm.wheel.slots.len().max(1) as f32) * 12.0 + 22.0;
        game.native_fill(batch, [cx - 6.0, cy - 14.0, 162.0, wheel_h], 0xCC0A_101A);
        game.native_text(
            batch,
            "DEVIL FRUIT ABILITY WHEEL",
            [cx, cy - 11.0],
            0xFFD770,
            true,
        );
        for (i, slot) in vm.wheel.slots.iter().enumerate() {
            let row = format!("[{}] {}", i + 1, slot.ability_name);
            let col = if slot.is_selected {
                0x55FFAA
            } else {
                0xDCE6F2
            };
            game.native_text(batch, &row, [cx + 4.0, cy + (i as f32) * 12.0], col, true);
        }
    }
}
