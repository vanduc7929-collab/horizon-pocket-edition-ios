//! Cold & freezing status engine: Chill stacks (1..=4 -> Frostbite burst),
//! Frostbite effect, Frozen encasement (`STATUE`, `BLOCK`, `CRYSTAL`),
//! struggle breakouts, heat/hit shatter, and ice slowness.
//!
//! Mirrors `core/cold/{Cold,ColdState,ColdEvents,Frozen,FrozenKind,StrugglePayload}.java`.

use serde::{Deserialize, Serialize};

use crate::core::combat::GrandLineDamageType;

/// Maximum chill stacks before triggering a Frostbite burst (`Cold.MAX_CHILL`).
pub const MAX_CHILL: i32 = 4;

/// Ticks before chill stacks begin melting (`Cold.CHILL_HOLD`).
pub const CHILL_HOLD_TICKS: i64 = 80;

/// Ticks per chill stack lost after `CHILL_HOLD_TICKS` (`Cold.CHILL_MELT`).
pub const CHILL_MELT_TICKS: i64 = 30;

/// Minimum ticks between struggle key presses (`Frozen.STRUGGLE_GAP`).
pub const FROZEN_STRUGGLE_GAP_TICKS: i64 = 2;

/// Visual & mechanical frozen encasement kind (`com.devilfruits.core.cold.FrozenKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrozenKind {
    Statue = 0,
    Block = 1,
    Crystal = 2,
}

impl FrozenKind {
    pub const fn from_ordinal(ord: i32) -> Option<Self> {
        match ord {
            0 => Some(Self::Statue),
            1 => Some(Self::Block),
            2 => Some(Self::Crystal),
            _ => None,
        }
    }

    pub const fn ordinal(self) -> i32 {
        self as i32
    }
}

/// Parameters for freezing an entity (`Frozen.Rules`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenRules {
    pub kind: FrozenKind,
    pub ticks: i32,
    pub breakable_by_hit: bool,
    pub struggle_need: f32,
    pub struggle_factor: f32,
    pub boss_factor: f32,
}

impl FrozenRules {
    /// Exact factory matching `Frozen.Rules.of(kind, ticks)`.
    pub const fn of(kind: FrozenKind, ticks: i32) -> Self {
        let is_statue = matches!(kind, FrozenKind::Statue);
        Self {
            kind,
            ticks,
            breakable_by_hit: !is_statue,
            struggle_need: if is_statue { 0.0 } else { 20.0 },
            struggle_factor: 1.0,
            boss_factor: 0.4,
        }
    }

    pub const fn with_struggle(mut self, need: f32, factor: f32) -> Self {
        self.struggle_need = need;
        self.struggle_factor = if factor < 0.0 { 0.0 } else { factor };
        self
    }

    pub const fn with_breakable(mut self, breakable: bool) -> Self {
        self.breakable_by_hit = breakable;
        self
    }

    pub fn effective_ticks(&self, is_boss: bool) -> i32 {
        if is_boss {
            ((self.ticks as f32) * self.boss_factor).round().max(10.0) as i32
        } else {
            self.ticks
        }
    }
}

/// Burst specification when chill reaches 4 stacks (`Cold.Burst`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChillBurst {
    pub damage: f32,
    pub damage_type: Option<GrandLineDamageType>,
    pub slow_ticks: i32,
    pub frostbite_level: i32,
}

impl Default for ChillBurst {
    fn default() -> Self {
        Self {
            damage: 6.0,
            damage_type: Some(GrandLineDamageType::HieFrost),
            slow_ticks: 100,
            frostbite_level: 2,
        }
    }
}

/// Per-entity synced Cold & Frozen state (`com.devilfruits.core.cold.ColdState`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColdState {
    pub chill: i32,
    pub last_chill: i64,
    pub cold_until: i64,
    pub kind: i32,
    pub frozen_at: i64,
    pub frozen_until: i64,
    pub breakable: bool,
    pub struggle_need: f32,
    pub struggle: f32,
    pub style: Option<String>,
    pub yaw: f32,
    pub head_yaw: f32,
    pub pitch: f32,
    pub struggle_factor: f32,
    pub last_struggle_press: i64,
    pub frostbite_amplifier: Option<u8>,
}

impl Default for ColdState {
    fn default() -> Self {
        Self::NONE
    }
}

impl ColdState {
    pub const NONE: Self = Self {
        chill: 0,
        last_chill: 0,
        cold_until: 0,
        kind: -1,
        frozen_at: 0,
        frozen_until: 0,
        breakable: false,
        struggle_need: 0.0,
        struggle: 0.0,
        style: None,
        yaw: 0.0,
        head_yaw: 0.0,
        pitch: 0.0,
        struggle_factor: 1.0,
        last_struggle_press: i64::MIN / 4,
        frostbite_amplifier: None,
    };

    /// Exact decaying chill stack count at `now` (`ColdState.chillAt`):
    /// `idle = now - lastChill - 80; idle < 0 ? chill : max(0, chill - 1 - (idle / 30))`.
    pub fn chill_at(&self, now: i64) -> i32 {
        let idle = now - self.last_chill - CHILL_HOLD_TICKS;
        if idle < 0 {
            self.chill
        } else {
            (self.chill - 1 - (idle / CHILL_MELT_TICKS) as i32).max(0)
        }
    }

    pub fn is_frozen(&self, now: i64) -> bool {
        self.kind >= 0 && self.frozen_until > now
    }

    pub fn frozen_kind(&self, now: i64) -> Option<FrozenKind> {
        if self.is_frozen(now) {
            FrozenKind::from_ordinal(self.kind)
        } else {
            None
        }
    }

    pub fn is_cold(&self, now: i64) -> bool {
        self.cold_until > now
    }

    pub fn struggle_progress(&self) -> f32 {
        if self.struggle_need <= 0.0 {
            0.0
        } else {
            (self.struggle / self.struggle_need).clamp(0.0, 1.0)
        }
    }

    pub fn is_empty(&self, now: i64) -> bool {
        self.chill_at(now) == 0 && !self.is_frozen(now) && !self.is_cold(now)
    }

    /// Ice/frostbite movement slowness multiplier in `0.0..=1.0` (`1.0` = normal speed, `0.0` = frozen solid).
    pub fn movement_speed_multiplier(&self, now: i64) -> f32 {
        if self.is_frozen(now) {
            return 0.0;
        }
        let chill_stacks = self.chill_at(now);
        let chill_slow = (chill_stacks as f32) * 0.12;
        let frost_slow = if self.is_cold(now) {
            let amp = self.frostbite_amplifier.unwrap_or(0) as f32;
            (0.25 + 0.15 * amp).min(0.75)
        } else {
            0.0
        };
        (1.0 - chill_slow.max(frost_slow)).clamp(0.1, 1.0)
    }

    pub fn thaw(&mut self) {
        self.kind = -1;
        self.frozen_at = 0;
        self.frozen_until = 0;
        self.breakable = false;
        self.struggle_need = 0.0;
        self.struggle = 0.0;
    }
}

/// Outcome of adding chill stacks (`Cold.addChill`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChillOutcome {
    Stacked(i32),
    BurstTriggered {
        damage: f32,
        damage_type: Option<GrandLineDamageType>,
        slow_ticks: i32,
        frostbite_amplifier: u8,
    },
}

/// Cold & Frozen state machine (`Cold.java` + `Frozen.java`).
pub struct ColdManager;

impl ColdManager {
    /// Adds 1 chill stack; on reaching `MAX_CHILL` (4), resets chill to 0 and triggers `burst` (`Cold.addChill`).
    pub fn add_chill(
        state: &mut ColdState,
        now: i64,
        style: Option<&str>,
        burst: ChillBurst,
    ) -> ChillOutcome {
        let stacks = state.chill_at(now) + 1;
        if let Some(st) = style {
            state.style = Some(st.to_string());
        }
        if stacks < MAX_CHILL {
            state.chill = stacks;
            state.last_chill = now;
            ChillOutcome::Stacked(stacks)
        } else {
            state.chill = 0;
            state.last_chill = now;
            state.cold_until = state.cold_until.max(now + burst.slow_ticks as i64);
            let amp = burst.frostbite_level.max(1).saturating_sub(1) as u8;
            state.frostbite_amplifier = Some(amp);
            ChillOutcome::BurstTriggered {
                damage: burst.damage,
                damage_type: burst.damage_type,
                slow_ticks: burst.slow_ticks,
                frostbite_amplifier: amp,
            }
        }
    }

    /// Applies Frostbite directly (`Cold.frostbite`).
    pub fn apply_frostbite(
        state: &mut ColdState,
        now: i64,
        style: Option<&str>,
        ticks: i32,
        level: i32,
    ) {
        if ticks <= 0 || level <= 0 {
            return;
        }
        state.cold_until = state.cold_until.max(now + ticks as i64);
        if let Some(st) = style {
            state.style = Some(st.to_string());
        }
        let new_amp = (level - 1).max(0) as u8;
        if state.frostbite_amplifier.is_none_or(|cur| cur <= new_amp) {
            state.frostbite_amplifier = Some(new_amp);
        }
    }

    /// Freezes a target with `FrozenRules` (`Frozen.freeze`).
    #[allow(clippy::too_many_arguments)]
    pub fn freeze(
        state: &mut ColdState,
        rules: FrozenRules,
        style: Option<&str>,
        is_boss: bool,
        creative_or_spectator: bool,
        pose_rot: (f32, f32, f32),
        now: i64,
    ) -> bool {
        if rules.ticks <= 0 || creative_or_spectator {
            return false;
        }
        let ticks = rules.effective_ticks(is_boss);
        let already = state.is_frozen(now);
        let until = (if already { state.frozen_until } else { 0 }).max(now + ticks as i64);
        state.chill = state.chill_at(now);
        state.kind = rules.kind.ordinal();
        if !already {
            state.frozen_at = now;
            state.struggle = 0.0;
            state.yaw = pose_rot.0;
            state.head_yaw = pose_rot.1;
            state.pitch = pose_rot.2;
        }
        state.frozen_until = until;
        state.breakable = rules.breakable_by_hit;
        state.struggle_need = rules.struggle_need;
        state.struggle_factor = rules.struggle_factor;
        if let Some(st) = style {
            state.style = Some(st.to_string());
        }
        true
    }

    /// Processes a player struggle input (`Frozen.struggle`); returns `true` if the player broke free.
    pub fn struggle(state: &mut ColdState, now: i64) -> bool {
        if !state.is_frozen(now) || state.struggle_need <= 0.0 {
            return false;
        }
        if now - state.last_struggle_press < FROZEN_STRUGGLE_GAP_TICKS && now >= state.last_struggle_press {
            return false;
        }
        state.last_struggle_press = now;
        if state.struggle_factor <= 0.0 {
            return false;
        }
        state.struggle += state.struggle_factor;
        if state.struggle >= state.struggle_need {
            state.thaw();
            true
        } else {
            false
        }
    }

    /// Evaluates whether an incoming damage event shatters/melts the frozen state (`Frozen.onHit`).
    pub fn on_hit(
        state: &mut ColdState,
        now: i64,
        is_heat_damage: bool,
        is_fire_or_explosion: bool,
        has_external_attacker: bool,
    ) -> bool {
        if !state.is_frozen(now) {
            return false;
        }
        let breaks = is_heat_damage
            || (state.breakable && (is_fire_or_explosion || has_external_attacker));
        if breaks {
            state.thaw();
            true
        } else {
            false
        }
    }

    /// Ticks frozen expiration and fire/lava melting (`Frozen.hold`).
    /// Returns `true` if the entity thawed or broke free on this tick.
    pub fn tick(state: &mut ColdState, now: i64, on_fire_or_in_lava: bool) -> bool {
        if !state.is_cold(now) {
            state.frostbite_amplifier = None;
        }
        if state.kind < 0 {
            return false;
        }
        if !state.is_frozen(now) {
            state.thaw();
            return true;
        }
        if state.breakable && on_fire_or_in_lava {
            state.thaw();
            return true;
        }
        false
    }
}
