//! Horizon PE: Shared Worlds + Mods: the keys of the native mods this edition is built with that
//! a touch button or a controller presses. Zoom's key is held (the touch controls' zoom button,
//! the controller's View button); Waymap's two are taps (the waypoint buttons, the D-pad's left
//! and down).
//!
//! A mod reads its key through `native_mod_key` (the player's choice in Options > Mods, else its
//! default), so what is pressed here is that same key by name: the mod can't tell it from a
//! keyboard's. The buttons are there only while their mod is in the app and turned on.
use super::{Game, Screen};

/// A native mod's key: the mod, the key's name in it, and the key pressed when the player
/// chose none.
pub(super) struct ModKey {
    pub mod_id: &'static str,
    pub name: &'static str,
    pub default: &'static str,
}

/// Zoom: hold to zoom in (`key.zoom.zoom`, Z).
pub(super) const ZOOM: ModKey = ModKey {
    mod_id: "zoom",
    name: "zoom",
    default: "key.keyboard.z",
};
/// Waymap: the list of waypoints. M when none is chosen: Waymap takes M as well as its own
/// default.
pub(super) const WAYPOINTS: ModKey = ModKey {
    mod_id: "waymap",
    name: "panel",
    default: "key.keyboard.m",
};
/// Waymap: a waypoint here (N).
pub(super) const ADD_WAYPOINT: ModKey = ModKey {
    mod_id: "waymap",
    name: "add",
    default: "key.keyboard.n",
};
/// Grand Line (`devilfruits`): cast / charge / hold the active Devil Fruit ability (`R`).
pub(super) const GRANDLINE_CAST: ModKey = ModKey {
    mod_id: "devilfruits",
    name: "use_ability",
    default: "key.keyboard.r",
};
/// Grand Line (`devilfruits`): open or cycle the Devil Fruit Ability Wheel (`V`).
pub(super) const GRANDLINE_WHEEL: ModKey = ModKey {
    mod_id: "devilfruits",
    name: "ability_wheel",
    default: "key.keyboard.v",
};
/// Grand Line (`devilfruits`): advance to the next equipped Devil Fruit ability slot (`G`).
pub(super) const GRANDLINE_NEXT: ModKey = ModKey {
    mod_id: "devilfruits",
    name: "next_ability",
    default: "key.keyboard.g",
};
/// Grand Line (`devilfruits`): toggle Transformation Form / Gear / Logia Elemental Body (`X`).
pub(super) const GRANDLINE_FORM: ModKey = ModKey {
    mod_id: "devilfruits",
    name: "form_switch",
    default: "key.keyboard.x",
};
/// Grand Line (`devilfruits`): toggle Busoshoku (Armament) Haki / Ryuo coating (`H`).
pub(super) const GRANDLINE_HAKI: ModKey = ModKey {
    mod_id: "devilfruits",
    name: "armament_haki",
    default: "key.keyboard.h",
};

/// What is held, and which of the mods are in this app.
#[derive(Default)]
pub(super) struct ModKeys {
    /// Who holds which key down: the touch button or the controller, and the key as it was
    /// pressed (let go of by that name, whatever the player chooses meanwhile).
    held: Vec<(&'static str, String)>,
    /// The ids of the native mods in this app, asked once.
    built: Option<Vec<String>>,
    /// The mod whose own screen is open says one of its text fields was tapped: the phone's
    /// keyboard is up for it (`native_mod_screen_typing`).
    typing: bool,
}

impl Game<'_> {
    /// Whether native mod `id` is in this app and left on in Options > Mods.
    pub(super) fn mod_key_ready(&mut self, id: &str) -> bool {
        if self.mod_keys.built.is_none() {
            let mut mods = Vec::new();
            self.native_mods(&mut mods);
            self.mod_keys.built = Some(mods.into_iter().map(|m| m.id).collect());
        }
        let built = self.mod_keys.built.as_ref();
        built.is_some_and(|ids| ids.iter().any(|m| m == id)) && self.native_on(id)
    }

    /// The mod whose own screen is open (`native_mod_screen_open`) says whether the player is
    /// typing in one of its text fields: a tap on one, until a tap elsewhere. A phone has no
    /// keys until its keyboard is up, and the game can't see a mod's fields: with this it
    /// brings the keyboard up and puts it away (game/touch.rs `text_context`). What is typed
    /// reaches the mod as on a computer (`native_screen_char_typed_pre`, the key hooks).
    /// Called by the mods built into this edition, so not by a build without them.
    #[allow(dead_code)]
    pub(super) fn native_mod_screen_typing(&mut self, mod_id: &str, typing: bool) {
        if self.native_mod_screen().is_some_and(|id| id == mod_id) {
            self.mod_keys.typing = typing;
        }
    }

    /// Which world is open, for a mod that keeps something for each (Waymap's waypoints):
    /// the kind and a name that is its alone. A shared world is ("shared", its id), the same
    /// on every phone and whoever hosts it; a world of this device ("world", its folder in
    /// `saves/`, where two worlds with one name still differ); a server ("server", its
    /// address). None when no world is open.
    ///
    /// The game joins its own worlds over a connection too, whose address is "local" for
    /// every one of them: a mod that went by the address kept one list for all of them.
    /// Called by the mods built into this edition, so not by a build without them.
    #[allow(dead_code)]
    pub(super) fn native_world_key(&self) -> Option<(&'static str, String)> {
        if self.integrated.is_none() && self.remote.is_none() {
            return None;
        }
        if let Some(id) = self.shared_world_id() {
            return Some(("shared", id.to_owned()));
        }
        if let Some(server) = &self.integrated {
            // A server started without a save folder of its own goes by its world's name.
            let name = if server.folder.is_empty() {
                &server.world_name
            } else {
                &server.folder
            };
            return Some(("world", name.clone()));
        }
        let address = &self.remote.as_ref()?.session.address;
        // The game's own world, on its way out: no world of its own any more.
        (address != "local").then(|| ("server", address.clone()))
    }

    /// Whether the open screen is a mod's own with one of its text fields being typed in.
    pub(super) fn mod_screen_typing(&mut self) -> bool {
        if self.screen != Screen::Mod {
            self.mod_keys.typing = false;
        }
        self.mod_keys.typing
    }

    /// `key` goes down or up for `holder` ("touch", "pad"). A key two hold is down until both
    /// let go; one that isn't held isn't let go of.
    pub(super) fn mod_key_hold(&mut self, holder: &'static str, key: &ModKey, down: bool) {
        let at = self.mod_keys.held.iter().position(|(h, _)| *h == holder);
        match (down, at) {
            (true, None) => {
                if !self.mod_key_ready(key.mod_id) {
                    return;
                }
                let name = self.native_mod_key(key.mod_id, key.name, key.default);
                let already = self.mod_keys.held.iter().any(|(_, n)| *n == name);
                self.mod_keys.held.push((holder, name.clone()));
                if !already {
                    self.mod_key_send(&name, true);
                }
            }
            (false, Some(i)) => {
                let (_, name) = self.mod_keys.held.remove(i);
                if !self.mod_keys.held.iter().any(|(_, n)| *n == name) {
                    self.mod_key_send(&name, false);
                }
            }
            _ => {}
        }
    }

    /// `key` pressed and let go.
    pub(super) fn mod_key_tap(&mut self, key: &ModKey) {
        if !self.mod_key_ready(key.mod_id) {
            return;
        }
        let name = self.native_mod_key(key.mod_id, key.name, key.default);
        #[cfg(any(target_os = "android", target_os = "ios"))]
        self.android_custom_token_tap(&name);
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            self.mod_key_send(&name, true);
            self.mod_key_send(&name, false);
        }
    }

    /// The key by its name (`key.keyboard.z`), as a keyboard's: on a phone the way a custom
    /// touch button's key goes, so the game and the mods both hear it.
    fn mod_key_send(&mut self, name: &str, down: bool) {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        self.android_custom_token_edge(name, down);
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        if let Some(code) = horizon_client::keybinds::key_code(name) {
            self.native_key_event(code, down, false);
        }
    }
}
