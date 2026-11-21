//! Grand Line (Devil Fruits 0.4.0) physics, body, stealth, and knowledge-gating
//! contracts over Horizon's vanilla world layer.
//!
//! Every constant and predicate mirrors the decompiled common Mixins and core
//! body/physics/rules classes of `com.devilfruits`:
//! - `BlockStateBaseMixin` / `FormCollision` (leaves pass-through, form fire immunity)
//! - `EntityMixin` / `FireProof` / `EntityMass` / `PlayerMixin` (fire immunity, piston/fluid immovability, mass kg)
//! - `EntitySwimMixin` / `LivingEntityFluidSwimMixin` / `FluidSwim` (lava swimming as water)
//! - `LivingEntityFrictionMixin` / `Friction` / `HieEvents` (ground/ice friction overrides)
//! - `Weakness` / `CoreConfig` / `PoneglyphConfig` (water submersion sinking, debuffs, seastone suppression)
//! - `ClearEntityMixin` / `ClearLivingEntityMixin` / `Clear` / `Steps` / `SukeEvents` (stealth flags & muffled footsteps)
//! - `CrafterBlockMixin` / `CraftingMenuMixin` / `Knowledge` (Poneglyph crafting gate)
//! - `WalkAnimationStateAccessor` / `ServerGamePacketListenerImplAccessor` / `AirTime`
//!
//! When [`EntityBodyModifiers::default()`] is used, every hook resolves to 100%
//! vanilla behavior.

use crate::{
    level::data::PushReaction,
    living_air::{self, LivingAirWorld},
};
use horizon_core::{geometry::Vec3, position::BlockPos};

// ---------------------------------------------------------------------------
// Exact Grand Line constants
// ---------------------------------------------------------------------------

/// `EntityMass.DEFAULT_KG`: baseline human mass in kilograms.
pub const ENTITY_MASS_DEFAULT_KG: f64 = 70.0;

/// `EntityMass.hitboxRatio`: vanilla player hitbox volume (`0.6 * 0.6 * 1.8`).
pub const ENTITY_MASS_PLAYER_HITBOX_VOLUME: f64 = 0.648;

/// `EntityMass.hitboxRatio`: lower clamp on the hitbox volume ratio.
pub const ENTITY_MASS_HITBOX_RATIO_MIN: f64 = 0.5;

/// `EntityMass.hitboxRatio`: upper clamp on the hitbox volume ratio.
pub const ENTITY_MASS_HITBOX_RATIO_MAX: f64 = 40.0;

/// `WeightManager.IMMOVABLE_KG`: Kilo-Kilo mass threshold for piston/fluid immovability.
pub const ENTITY_MASS_IMMOVABLE_KG: f64 = 3000.0;

/// `ChampionForm`: Doru-Doru Wax Champion mass in kilograms.
pub const ENTITY_MASS_CHAMPION_KG: f64 = 250.0;

/// `WeightManager.targetOf(1)`: Kilo-Kilo 1 kg light mode target.
pub const ENTITY_MASS_LIGHT_TARGET_KG: f64 = 1.0;

/// `WeightManager.targetOf(2)`: Kilo-Kilo 10,000 kg press mode target.
pub const ENTITY_MASS_HEAVY_TARGET_KG: f64 = 10000.0;

/// `FluidSwim.travelAsWater`: horizontal drag while sprinting in lava.
pub const FLUID_SWIM_SPRINT_DRAG: f32 = 0.9;

/// `FluidSwim.travelAsWater`: base horizontal swim acceleration before modifiers.
pub const FLUID_SWIM_BASE_SPEED: f32 = 0.02;

/// `FluidSwim.travelAsWater`: drag interpolation target for `WATER_MOVEMENT_EFFICIENCY`.
pub const FLUID_SWIM_EFFICIENCY_DRAG_TARGET: f32 = 0.54600006;

/// `FluidSwim.travelAsWater`: `WATER_MOVEMENT_EFFICIENCY` multiplier when off ground.
pub const FLUID_SWIM_AIR_EFFICIENCY_SCALE: f32 = 0.5;

/// `FluidSwim.travelAsWater`: horizontal drag under Dolphin's Grace.
pub const FLUID_SWIM_DOLPHINS_GRACE_DRAG: f32 = 0.96;

/// `FluidSwim.travelAsWater`: vertical dive impulse when holding shift off ground.
pub const FLUID_SWIM_SHIFT_DIVE_DELTA_Y: f64 = -0.04;

/// `FluidSwim.travelAsWater`: upward climb speed when colliding horizontally on climbables.
pub const FLUID_SWIM_CLIMBABLE_Y: f64 = 0.2;

/// `FluidSwim.travelAsWater`: vertical velocity drag factor.
pub const FLUID_SWIM_VERTICAL_DRAG: f32 = 0.8;

/// `FluidSwim.travelAsWater`: maximum gravity while falling with Slow Falling.
pub const FLUID_SWIM_SLOW_FALLING_MAX_GRAVITY: f64 = 0.01;

/// `FluidSwim.travelAsWater`: upward free-space probe offset for climbing out of lava.
pub const FLUID_SWIM_STEP_OUT_PROBE_Y: f32 = 0.6;

/// `FluidSwim.travelAsWater`: upward velocity set when stepping out of lava onto a ledge.
pub const FLUID_SWIM_STEP_OUT_VELOCITY_Y: f32 = 0.3;

/// `HieEvents.GROUND`: standard ground friction applied to Hie-Hie users on ice.
pub const FRICTION_GROUND: f32 = 0.6;

/// `HieEvents.ICE`: slippery friction applied on top of `HieBlocks.ICE_SHEET`.
pub const FRICTION_ICE: f32 = 0.98;

/// `Weakness.isSubmerged`: vertical offset added to entity Y when sampling the feet block.
pub const WEAKNESS_FEET_Y_OFFSET: f64 = 0.1;

/// `CoreConfig.WATER_WEAKNESS`: submersion depth threshold ("deeper than one block").
pub const WEAKNESS_SUBMERSION_THRESHOLD: f64 = 1.0;

/// `CoreConfig.SINK_SPEED`: default downward speed (blocks/tick) forced on submerged users.
pub const WEAKNESS_DEFAULT_SINK_SPEED: f64 = 0.08;

/// `Weakness.applySinking`: horizontal velocity multiplier while sinking.
pub const WEAKNESS_SINK_HORIZONTAL_DRAG: f64 = 0.6;

/// `Weakness.tickServer`: server tick cadence for refreshing suppression debuffs.
pub const WEAKNESS_EFFECT_INTERVAL_TICKS: i32 = 10;

/// `Weakness.tickServer`: duration in ticks of each suppression debuff instance.
pub const WEAKNESS_EFFECT_DURATION_TICKS: i32 = 40;

/// `Weakness.tickServer`: Slowness amplifier (`2` = Slowness III).
pub const WEAKNESS_SLOWNESS_AMPLIFIER: u8 = 2;

/// `Weakness.tickServer`: Weakness amplifier (`1` = Weakness II).
pub const WEAKNESS_WEAKNESS_AMPLIFIER: u8 = 1;

/// `Weakness.tickServer`: Mining Fatigue (`DIG_SLOWDOWN`) amplifier (`1` = Mining Fatigue II).
pub const WEAKNESS_MINING_FATIGUE_AMPLIFIER: u8 = 1;

/// `PoneglyphConfig.HIT_SUPPRESS_TICKS`: default seastone hit suppression duration.
pub const SEASTONE_HIT_SUPPRESS_TICKS: i32 = 60;

/// `PoneglyphConfig.JITTE_STUN_TICKS`: default Seastone Jitte stun duration.
pub const SEASTONE_JITTE_STUN_TICKS: i32 = 30;

/// `PoneglyphConfig.CUFFS_TICKS`: default Seastone Cuffs restraint duration.
pub const SEASTONE_CUFFS_TICKS: i32 = 600;

/// `Clear.INVISIBLE` bit flag.
pub const CLEAR_FLAG_INVISIBLE: u32 = 1;

/// `Clear.HIDDEN_ITEMS` bit flag.
pub const CLEAR_FLAG_HIDDEN_ITEMS: u32 = 2;

/// `Clear.SHIP` bit flag.
pub const CLEAR_FLAG_SHIP: u32 = 4;

/// `Clear.NOBODY`: zero UUID for ungrouped clear state.
pub const CLEAR_NOBODY_UUID: [u8; 16] = [0; 16];

/// `SukeSuke` `"mobVisibility"` default factor.
pub const SUKE_DEFAULT_MOB_VISIBILITY: f64 = 0.07;

/// `SukeSuke` `"muffleSteps"` default footstep volume multiplier.
pub const SUKE_DEFAULT_MUFFLE_STEPS: f32 = 0.5;

/// `SukeSuke` `"shipSoundFactor"` default hidden-ship sound multiplier.
pub const SUKE_DEFAULT_SHIP_SOUND_FACTOR: f32 = 0.35;

/// `SukeSuke` `"groupRadius"` default shared-invisibility radius in blocks.
pub const SUKE_DEFAULT_GROUP_RADIUS: f64 = 3.0;

/// `SukeSuke` `"spectatorsSee"` default flag.
pub const SUKE_DEFAULT_SPECTATORS_SEE: bool = true;

/// Vanilla `#minecraft:leaves` block identifiers checked by `BlockStateBaseMixin`.
pub const VANILLA_LEAVES_BLOCKS: [&str; 11] = [
    "minecraft:oak_leaves",
    "minecraft:spruce_leaves",
    "minecraft:birch_leaves",
    "minecraft:jungle_leaves",
    "minecraft:acacia_leaves",
    "minecraft:dark_oak_leaves",
    "minecraft:mangrove_leaves",
    "minecraft:azalea_leaves",
    "minecraft:flowering_azalea_leaves",
    "minecraft:cherry_leaves",
    "minecraft:pale_oak_leaves",
];

// ---------------------------------------------------------------------------
// Core modifier types
// ---------------------------------------------------------------------------

/// Mass classification mirroring `EntityMass` and Kilo-Kilo / Champion weight bands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EntityMassTier {
    /// Reduced mass (`< 70.0 kg`, e.g. Kilo-Kilo 1 kg mode).
    Light,
    /// Standard vanilla mass (`70.0 * hitbox_ratio`).
    #[default]
    Normal,
    /// Elevated but movable mass (`> 70.0 kg` and `< 3000.0 kg`, e.g. Doru-Doru Champion `250 kg`).
    Heavy,
    /// Immovable mass (`>= 3000.0 kg` with `heavyIgnoresPushing = true`): ignores piston & fluid pushing.
    Immovable,
}

/// Stealth and Suke-Suke visibility flags mirroring `ClearEntityMixin`,
/// `ClearLivingEntityMixin`, `Clear`, `Steps`, and `SukeEvents`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StealthFlags {
    /// Mirrors `Clear.INVISIBLE` (`ClearEntityMixin#isInvisible` / `#isInvisibleTo`).
    pub invisible: bool,
    /// Mirrors `Clear.HIDDEN_ITEMS` (`ClearLivingEntityMixin#spawnItemParticles`).
    pub hidden_items: bool,
    /// Mirrors `Clear.SHIP` bit (`Clear.SHIP = 4`).
    pub ship_cloaked: bool,
    /// Suppresses burning fire overlay (`ClearEntityMixin#displayFireAnimation`).
    pub hide_fire: bool,
    /// Suppresses glowing outline (`ClearEntityMixin#isCurrentlyGlowing`).
    pub hide_glow: bool,
    /// Suppresses sprint dust particles (`ClearEntityMixin#canSpawnSprintParticle`).
    pub hide_sprint_particles: bool,
    /// Suppresses water splash particles (`ClearEntityMixin#doWaterSplashEffect`).
    pub hide_splash_particles: bool,
    /// Suppresses potion effect swirls (`ClearLivingEntityMixin#tickEffects`).
    pub hide_potion_particles: bool,
    /// Suppresses eating/drinking item crumb particles (`ClearLivingEntityMixin#spawnItemParticles`).
    pub hide_eating_particles: bool,
    /// Footstep volume multiplier (`1.0` = vanilla unchanged, `0.5` = Suke-Suke default, `0.0` = muted).
    pub footstep_volume_scale: f32,
    /// Enables step-sound interception (`ClearEntityMixin#playStepSound` / `Steps` / `SukeEvents#steps`).
    pub hook_footsteps: bool,
    /// Mirrors `SukeSuke` `"spectatorsSee"` flag (default `true`).
    pub spectators_see: bool,
    /// Cloak group UUID (`Clear.State.group`, `[0; 16]` = `Clear.NOBODY`).
    pub cloak_group: [u8; 16],
}

impl Default for StealthFlags {
    fn default() -> Self {
        Self::NONE
    }
}

impl StealthFlags {
    /// Vanilla-neutral stealth state (no visibility, particle, or footstep overrides).
    pub const NONE: Self = Self {
        invisible: false,
        hidden_items: false,
        ship_cloaked: false,
        hide_fire: false,
        hide_glow: false,
        hide_sprint_particles: false,
        hide_splash_particles: false,
        hide_potion_particles: false,
        hide_eating_particles: false,
        footstep_volume_scale: 1.0,
        hook_footsteps: false,
        spectators_see: SUKE_DEFAULT_SPECTATORS_SEE,
        cloak_group: CLEAR_NOBODY_UUID,
    };

    /// Constructs the exact client-hidden Suke-Suke clear state (`Clear.hiddenOnClient == true`).
    pub const fn suke_clear_hidden(group: [u8; 16], hidden_items: bool, ship_cloaked: bool) -> Self {
        Self {
            invisible: true,
            hidden_items,
            ship_cloaked,
            hide_fire: true,
            hide_glow: true,
            hide_sprint_particles: true,
            hide_splash_particles: true,
            hide_potion_particles: true,
            hide_eating_particles: true,
            footstep_volume_scale: SUKE_DEFAULT_MUFFLE_STEPS,
            hook_footsteps: true,
            spectators_see: SUKE_DEFAULT_SPECTATORS_SEE,
            cloak_group: group,
        }
    }

    /// Reconstructs `StealthFlags` from serialized `Clear.State` bitmask (`INVISIBLE | HIDDEN_ITEMS | SHIP`)
    /// plus the client-side `hidden_on_client` predicate (`Clear.hiddenHere`).
    pub const fn from_clear_bits(
        flags: u32,
        group: [u8; 16],
        hidden_on_client: bool,
        muffle_steps: f32,
        spectators_see: bool,
    ) -> Self {
        let invisible = (flags & CLEAR_FLAG_INVISIBLE) != 0;
        let hidden_items = (flags & CLEAR_FLAG_HIDDEN_ITEMS) != 0;
        let ship_cloaked = (flags & CLEAR_FLAG_SHIP) != 0;
        let client_hidden = invisible && hidden_on_client;
        Self {
            invisible,
            hidden_items,
            ship_cloaked,
            hide_fire: client_hidden,
            hide_glow: client_hidden,
            hide_sprint_particles: client_hidden,
            hide_splash_particles: client_hidden,
            hide_potion_particles: client_hidden,
            hide_eating_particles: client_hidden,
            footstep_volume_scale: if invisible { muffle_steps } else { 1.0 },
            hook_footsteps: invisible,
            spectators_see,
            cloak_group: group,
        }
    }

    /// Encodes the `Clear.State` bitmask (`INVISIBLE = 1`, `HIDDEN_ITEMS = 2`, `SHIP = 4`).
    pub const fn clear_bits(&self) -> u32 {
        (if self.invisible {
            CLEAR_FLAG_INVISIBLE
        } else {
            0
        }) | (if self.hidden_items {
            CLEAR_FLAG_HIDDEN_ITEMS
        } else {
            0
        }) | (if self.ship_cloaked {
            CLEAR_FLAG_SHIP
        } else {
            0
        })
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clear` (`Entity.isInvisible`).
    pub const fn is_invisible(&self, vanilla: bool, drawn_solid: bool) -> bool {
        if !drawn_solid && self.invisible {
            true
        } else {
            vanilla
        }
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clearTo` (`Entity.isInvisibleTo`).
    pub fn is_invisible_to(
        &self,
        vanilla_invisible_to: bool,
        drawn_solid: bool,
        viewer: Option<StealthViewer>,
    ) -> bool {
        if !drawn_solid && self.invisible {
            !clear_sees(viewer, self)
        } else {
            vanilla_invisible_to
        }
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clearGlow` (`Entity.isCurrentlyGlowing`).
    pub const fn is_currently_glowing(&self, vanilla: bool) -> bool {
        if self.hide_glow { false } else { vanilla }
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clearFire` (`Entity.displayFireAnimation`).
    pub const fn display_fire_animation(&self, vanilla: bool) -> bool {
        if self.hide_fire { false } else { vanilla }
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clearSprintDust` (`Entity.canSpawnSprintParticle`).
    pub const fn can_spawn_sprint_particle(&self, vanilla: bool) -> bool {
        if self.hide_sprint_particles {
            false
        } else {
            vanilla
        }
    }

    /// Mirrors `ClearEntityMixin#devilfruits$clearSplash` (`Entity.doWaterSplashEffect`).
    pub const fn can_spawn_water_splash_particle(&self) -> bool {
        !self.hide_splash_particles
    }

    /// Mirrors `ClearLivingEntityMixin#devilfruits$clearSwirls` (`LivingEntity.tickEffects`).
    pub const fn can_spawn_potion_particles(&self) -> bool {
        !self.hide_potion_particles
    }

    /// Mirrors `ClearLivingEntityMixin#devilfruits$clearCrumbs` (`LivingEntity.spawnItemParticles`).
    pub const fn can_spawn_eating_particles(
        &self,
        is_client_side: bool,
        items_hidden_here: bool,
    ) -> bool {
        !is_client_side
            || (!self.hide_eating_particles && (!self.hidden_items || !items_hidden_here))
    }

    /// Mirrors `ClearEntityMixin#playStepSound` + `Steps` + `SukeEvents#steps`.
    pub fn effective_footstep_volume(&self, vanilla_volume: f32) -> f32 {
        if self.hook_footsteps && self.invisible {
            vanilla_volume * self.footstep_volume_scale
        } else {
            vanilla_volume
        }
    }

    /// Mirrors `SukeEvents#visibility` (`LivingVisibilityEvent`).
    pub fn effective_mob_visibility(
        &self,
        vanilla_visibility: f64,
        viewer: Option<StealthViewer>,
        mob_visibility_factor: f64,
    ) -> f64 {
        if !self.invisible {
            return vanilla_visibility;
        }
        // `SukeEvents#visibility` passes `spectatorsSee = false` to `Clear.sees`.
        let mut target_no_spec = *self;
        target_no_spec.spectators_see = false;
        let viewer_sees = viewer.is_some() && clear_sees(viewer, &target_no_spec);
        if !viewer_sees && vanilla_visibility > mob_visibility_factor && vanilla_visibility > 0.0 {
            mob_visibility_factor
        } else {
            vanilla_visibility
        }
    }
}

/// Viewer snapshot used by [`clear_sees`] (`Clear.sees(viewer, target, spectatorsSee)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StealthViewer {
    pub uuid: [u8; 16],
    pub is_same_entity: bool,
    pub is_spectator: bool,
    pub is_invisible: bool,
    pub cloak_group: [u8; 16],
}

/// Plain, zero-allocation modifier contract passed into `horizon-world` physics
/// and collision hooks by Grand Line client/server gameplay.
///
/// [`EntityBodyModifiers::default()`] produces 100% vanilla behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityBodyModifiers {
    /// Mirrors `BlockStateBaseMixin` / `FormCollision.passesThroughLeaves`.
    pub passes_through_leaves: bool,
    /// Mirrors `EntityMixin#fireImmune` / `FormCollision.fireImmune` / `FireProof.test`.
    pub fire_immune: bool,
    /// Mirrors `EntityMass` weight classification.
    pub mass_tier: EntityMassTier,
    /// Mirrors `EntityMass.immovable` (`EntityMixin#getPistonPushReaction` & `PlayerMixin#isPushedByFluid`).
    pub immovable: bool,
    /// Optional explicit mass override in kilograms (e.g. `Some(1.0)`, `Some(250.0)`, `Some(10000.0)`).
    pub mass_kg_override: Option<f64>,
    /// Mirrors `EntitySwimMixin` & `LivingEntityFluidSwimMixin` / `FluidSwim.swimsInLava`.
    pub swim_in_lava: bool,
    /// Mirrors `Weakness`: whether the entity is subject to Devil Fruit water weakness.
    pub devil_fruit_water_weakness: bool,
    /// Current water submersion depth in blocks (`> 1.0` triggers water weakness when not on water).
    pub water_submersion: f64,
    /// Mirrors `Weakness.onWater` predicate (e.g. Yomi-Yomi running across water).
    pub on_water_override: bool,
    /// Optional override for `CoreConfig.SINK_SPEED` (`None` uses [`WEAKNESS_DEFAULT_SINK_SPEED`] = `0.08`).
    pub sink_speed_override: Option<f64>,
    /// Mirrors `LivingEntityFrictionMixin` / `Friction.of`. `None` (or `Some(NaN)`) keeps vanilla friction.
    pub friction_override: Option<f32>,
    /// Mirrors `ClearEntityMixin` & `ClearLivingEntityMixin` / `Clear` / `Steps`.
    pub stealth_flags: StealthFlags,
    /// Mirrors `ServerGamePacketListenerImplAccessor#devilfruits$setAboveGroundTickCount(0)` / `AirTime`.
    pub reset_above_ground_ticks: bool,
}

impl Default for EntityBodyModifiers {
    fn default() -> Self {
        Self::VANILLA
    }
}

impl EntityBodyModifiers {
    /// Vanilla-neutral modifier snapshot (zero active overrides).
    pub const VANILLA: Self = Self {
        passes_through_leaves: false,
        fire_immune: false,
        mass_tier: EntityMassTier::Normal,
        immovable: false,
        mass_kg_override: None,
        swim_in_lava: false,
        devil_fruit_water_weakness: false,
        water_submersion: 0.0,
        on_water_override: false,
        sink_speed_override: None,
        friction_override: None,
        stealth_flags: StealthFlags::NONE,
        reset_above_ground_ticks: false,
    };

    /// Whether the entity is immovable by pistons and fluid currents (`EntityMass.immovable`).
    pub const fn is_immovable(&self) -> bool {
        self.immovable || matches!(self.mass_tier, EntityMassTier::Immovable)
    }

    /// Whether scalar water submersion exceeds the Devil Fruit weakness threshold (`> 1.0` block)
    /// while not protected by an `on_water` override.
    pub fn is_water_weakened(&self) -> bool {
        self.devil_fruit_water_weakness
            && !self.on_water_override
            && weakness_is_submerged_height(self.water_submersion)
    }

    /// Effective downward sinking speed in blocks/tick (`CoreConfig.SINK_SPEED`, clamped `0.0..=1.0`).
    pub fn effective_sink_speed(&self) -> f64 {
        self.sink_speed_override
            .unwrap_or(WEAKNESS_DEFAULT_SINK_SPEED)
            .clamp(0.0, 1.0)
    }

    /// Computes effective body mass in kilograms using `EntityMass.kg`.
    pub fn effective_mass_kg(&self, hitbox_width: f64, hitbox_height: f64) -> f64 {
        entity_mass_kg(self.mass_kg_override, hitbox_width, hitbox_height)
    }
}

/// Mirrors `WalkAnimationStateAccessor` (`speedOld`, `speed`, `position`) used by
/// `ColdClient` to snapshot, freeze, and restore limb swing animation on frozen entities.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WalkAnimationSnapshot {
    pub speed_old: f32,
    pub speed: f32,
    pub position: f32,
}

impl WalkAnimationSnapshot {
    pub const fn new(speed_old: f32, speed: f32, position: f32) -> Self {
        Self {
            speed_old,
            speed,
            position,
        }
    }

    /// Exact pose freeze applied in `ColdClient`: `position = pose.walkPos`,
    /// `speed = pose.walkSpeed`, `speedOld = pose.walkSpeed`.
    pub const fn frozen_at(position: f32, speed: f32) -> Self {
        Self {
            speed_old: speed,
            speed,
            position,
        }
    }
}

/// Inputs for `FluidSwim.takesOver(LivingEntity, boolean)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FluidSwimContext {
    pub controlled_by_local_instance: bool,
    pub in_lava: bool,
    pub in_water: bool,
    pub affected_by_fluids: bool,
    pub fall_flying: bool,
    pub player_flying: bool,
}

/// Status effect kind applied by `Weakness.tickServer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaknessEffectKind {
    MovementSlowdown,
    Weakness,
    DigSlowdown,
}

/// Exact `MobEffectInstance` payload emitted by `Weakness.tickServer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaknessDebuff {
    pub kind: WeaknessEffectKind,
    pub duration_ticks: i32,
    pub amplifier: u8,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
}

/// Inputs for `Weakness.touchesSeastone`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeastoneTouchState {
    pub main_hand_seastone: bool,
    pub main_hand_is_jitte: bool,
    pub main_hand_is_gear: bool,
    pub off_hand_seastone: bool,
    pub off_hand_is_jitte: bool,
    pub off_hand_is_gear: bool,
    pub has_suppressed_effect: bool,
    pub seastone_armor_pieces: u32,
    pub standing_on_seastone_block: bool,
    pub inside_seastone_block: bool,
}

/// Configuration flags for `Weakness.touchesSeastone` (`CoreConfig` + `PoneglyphConfig`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeastoneConfig {
    pub seastone_weakness: bool,
    pub armor_suppresses_fruit_user: bool,
    pub jitte_safe_handle: bool,
    pub gear_suppresses_holder: bool,
}

impl Default for SeastoneConfig {
    fn default() -> Self {
        Self {
            seastone_weakness: true,
            armor_suppresses_fruit_user: true,
            jitte_safe_handle: true,
            gear_suppresses_holder: true,
        }
    }
}

/// Outcome of the Poneglyph `Knowledge.allows` crafting gate (`CrafterBlockMixin` / `CraftingMenuMixin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftingGateResult {
    /// Recipe is unlocked, gating is disabled, the knowledge is ungated, or the player knows it.
    Allowed,
    /// Recipe is locked and `CrafterBlock` has no player (`Knowledge.allows(null, recipe)`).
    DeniedNoPlayer,
    /// Recipe is locked and the player lacks the required Poneglyph knowledge.
    DeniedMissingKnowledge,
}

impl CraftingGateResult {
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }
}

// ---------------------------------------------------------------------------
// Java-faithful math & hook helpers
// ---------------------------------------------------------------------------

#[inline]
fn java_min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() | b.to_bits())
    } else {
        a.min(b)
    }
}

#[inline]
fn java_max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a == 0.0 && b == 0.0 {
        f64::from_bits(a.to_bits() & b.to_bits())
    } else {
        a.max(b)
    }
}

/// Returns `true` if `name` is one of the vanilla `#minecraft:leaves` block identifiers.
pub fn is_leaves_block_name(name: &str) -> bool {
    VANILLA_LEAVES_BLOCKS.contains(&name)
}

/// Mirrors `BlockStateBaseMixin#devilfruits$leavesPassThrough` + `FormCollision.passesThroughLeaves`.
pub const fn passes_through_leaves(
    is_leaves: bool,
    has_entity_context: bool,
    modifiers: &EntityBodyModifiers,
) -> bool {
    has_entity_context && is_leaves && modifiers.passes_through_leaves
}

/// Mirrors `EntityMixin#devilfruits$formFireImmune` (`FormCollision.fireImmune || FireProof.test`).
pub const fn effective_fire_immune(vanilla: bool, modifiers: &EntityBodyModifiers) -> bool {
    modifiers.fire_immune || vanilla
}

/// Mirrors `EntityMass.hitboxRatio(Entity)`:
/// `Math.max(0.5, Math.min(40.0, (width * width * height) / 0.648))`.
pub fn entity_mass_hitbox_ratio(width: f64, height: f64) -> f64 {
    let volume = width * width * height;
    java_max(
        ENTITY_MASS_HITBOX_RATIO_MIN,
        java_min(
            ENTITY_MASS_HITBOX_RATIO_MAX,
            volume / ENTITY_MASS_PLAYER_HITBOX_VOLUME,
        ),
    )
}

/// Mirrors `EntityMass.kg(Entity)`: uses `provided_kg` when present and non-NaN,
/// otherwise `70.0 * entity_mass_hitbox_ratio(width, height)`.
pub fn entity_mass_kg(provided_kg: Option<f64>, width: f64, height: f64) -> f64 {
    match provided_kg {
        Some(kg) if !kg.is_nan() => kg,
        _ => ENTITY_MASS_DEFAULT_KG * entity_mass_hitbox_ratio(width, height),
    }
}

/// Mirrors `WeightManager.MASS.immovable(Entity)`: `kg >= 3000.0 && heavyIgnoresPushing`.
pub fn is_mass_immovable(kg: f64, heavy_ignores_pushing: bool) -> bool {
    heavy_ignores_pushing && kg >= ENTITY_MASS_IMMOVABLE_KG
}

/// Maps a mass in kilograms to an [`EntityMassTier`].
pub fn mass_tier_from_kg(kg: f64, heavy_ignores_pushing: bool) -> EntityMassTier {
    if is_mass_immovable(kg, heavy_ignores_pushing) {
        EntityMassTier::Immovable
    } else if kg > ENTITY_MASS_DEFAULT_KG {
        EntityMassTier::Heavy
    } else if kg < ENTITY_MASS_DEFAULT_KG {
        EntityMassTier::Light
    } else {
        EntityMassTier::Normal
    }
}

/// Mirrors `EntityMixin#devilfruits$immovableBody` (`Entity.getPistonPushReaction`).
pub const fn effective_piston_push_reaction(
    vanilla: PushReaction,
    modifiers: &EntityBodyModifiers,
) -> PushReaction {
    if modifiers.is_immovable() {
        PushReaction::IgnoreEntity
    } else {
        vanilla
    }
}

/// Mirrors `PlayerMixin#devilfruits$immovableBody` (`Player.isPushedByFluid`).
pub const fn effective_is_pushed_by_fluid(vanilla: bool, modifiers: &EntityBodyModifiers) -> bool {
    if modifiers.is_immovable() {
        false
    } else {
        vanilla
    }
}

/// Mirrors `FluidSwim.takesOver(LivingEntity, boolean)`.
pub const fn fluid_swim_takes_over(
    ctx: FluidSwimContext,
    modifiers: &EntityBodyModifiers,
) -> bool {
    modifiers.swim_in_lava
        && ctx.controlled_by_local_instance
        && ctx.in_lava
        && !ctx.in_water
        && ctx.affected_by_fluids
        && !ctx.fall_flying
        && !ctx.player_flying
}

/// Mirrors the gravity calculation at the start of `FluidSwim.travelAsWater`.
pub fn fluid_swim_effective_gravity(
    base_gravity: f64,
    velocity_y: f64,
    slow_falling: bool,
) -> f64 {
    if velocity_y <= 0.0 && slow_falling {
        java_min(base_gravity, FLUID_SWIM_SLOW_FALLING_MAX_GRAVITY)
    } else {
        base_gravity
    }
}

/// Mirrors `LivingEntityFrictionMixin` / `Friction.of` for a single modifier snapshot.
pub fn resolve_friction(vanilla: f32, modifiers: &EntityBodyModifiers) -> f32 {
    match modifiers.friction_override {
        Some(f) if !f.is_nan() => f,
        _ => vanilla,
    }
}

/// Mirrors `Friction.of`: returns the first non-NaN override in registration order,
/// or `vanilla` if none apply.
pub fn resolve_friction_chain(vanilla: f32, overrides: impl IntoIterator<Item = f32>) -> f32 {
    for f in overrides {
        if !f.is_nan() {
            return f;
        }
    }
    vanilla
}

/// Mirrors `HieEvents.friction`:
/// - Returns `None` (`Float.NaN` in Java) when neither on `#devilfruits:ice` nor under `ICE_SHEET`.
/// - Returns `Some(0.6)` for a Hie-Hie user when `!user_slides_on_ice`.
/// - Returns `Some(0.98)` when `has_ice_sheet_above`.
/// - Otherwise `None` (preserve vanilla ice friction).
pub const fn hie_ice_friction_override(
    on_ice_tag: bool,
    has_ice_sheet_above: bool,
    is_hie_user: bool,
    user_slides_on_ice: bool,
) -> Option<f32> {
    if !has_ice_sheet_above && !on_ice_tag {
        None
    } else if is_hie_user && !user_slides_on_ice {
        Some(FRICTION_GROUND)
    } else if has_ice_sheet_above {
        Some(FRICTION_ICE)
    } else {
        None
    }
}

/// Adapter that wraps `LivingAirWorld::block_friction` identically to
/// `LivingEntityFrictionMixin#devilfruits$ownFriction` (`@WrapOperation`).
struct FrictionOverrideWorld<'a, W> {
    inner: &'a mut W,
    modifiers: &'a EntityBodyModifiers,
}

impl<W: LivingAirWorld> LivingAirWorld for FrictionOverrideWorld<'_, W> {
    type Error = W::Error;

    fn movement_support_position(&mut self) -> Result<BlockPos, Self::Error> {
        self.inner.movement_support_position()
    }
    fn on_ground(&mut self) -> Result<bool, Self::Error> {
        self.inner.on_ground()
    }
    fn block_friction(&mut self, position: BlockPos) -> Result<f32, Self::Error> {
        let vanilla = self.inner.block_friction(position)?;
        Ok(resolve_friction(vanilla, self.modifiers))
    }
    fn friction_modifier(&mut self) -> Result<f64, Self::Error> {
        self.inner.friction_modifier()
    }
    fn speed(&mut self) -> Result<f32, Self::Error> {
        self.inner.speed()
    }
    fn flying_speed(&mut self) -> Result<f32, Self::Error> {
        self.inner.flying_speed()
    }
    fn yaw(&mut self) -> Result<f32, Self::Error> {
        self.inner.yaw()
    }
    fn velocity(&self) -> Vec3 {
        self.inner.velocity()
    }
    fn set_velocity(&mut self, velocity: Vec3) -> Result<(), Self::Error> {
        self.inner.set_velocity(velocity)
    }
    fn on_climbable(&mut self) -> Result<bool, Self::Error> {
        self.inner.on_climbable()
    }
    fn reset_fall_distance(&mut self) -> Result<(), Self::Error> {
        self.inner.reset_fall_distance()
    }
    fn in_scaffolding(&mut self) -> Result<bool, Self::Error> {
        self.inner.in_scaffolding()
    }
    fn suppressing_ladder_slide(&mut self) -> Result<bool, Self::Error> {
        self.inner.suppressing_ladder_slide()
    }
    fn is_player_class(&self) -> bool {
        self.inner.is_player_class()
    }
    fn move_self(&mut self, movement: Vec3) -> Result<(), Self::Error> {
        self.inner.move_self(movement)
    }
    fn horizontal_collision(&self) -> bool {
        self.inner.horizontal_collision()
    }
    fn jumping(&self) -> bool {
        self.inner.jumping()
    }
    fn was_in_powder_snow(&self) -> bool {
        self.inner.was_in_powder_snow()
    }
    fn can_walk_powder_snow(&mut self) -> Result<bool, Self::Error> {
        self.inner.can_walk_powder_snow()
    }
    fn levitation_amplifier(&mut self) -> Result<Option<i32>, Self::Error> {
        self.inner.levitation_amplifier()
    }
    fn is_client_side(&mut self) -> Result<bool, Self::Error> {
        self.inner.is_client_side()
    }
    fn has_chunk_at(&mut self, position: BlockPos) -> Result<bool, Self::Error> {
        self.inner.has_chunk_at(position)
    }
    fn effective_gravity(&mut self) -> Result<f64, Self::Error> {
        self.inner.effective_gravity()
    }
    fn y(&self) -> f64 {
        self.inner.y()
    }
    fn minimum_y(&mut self) -> Result<i32, Self::Error> {
        self.inner.minimum_y()
    }
    fn discard_friction(&mut self) -> Result<bool, Self::Error> {
        self.inner.discard_friction()
    }
    fn air_drag_modifier(&mut self) -> Result<f64, Self::Error> {
        self.inner.air_drag_modifier()
    }
    fn omnidirectional_air_mover(&mut self) -> Result<bool, Self::Error> {
        self.inner.omnidirectional_air_mover()
    }
}

/// Runs [`living_air::travel_in_air`] with `LivingEntityFrictionMixin` applied to
/// `block_friction`. When `modifiers.friction_override` is `None` (or `NaN`), this
/// executes identically to vanilla `travel_in_air`.
pub fn travel_in_air_with_modifiers<W: LivingAirWorld>(
    world: &mut W,
    input: Vec3,
    modifiers: &EntityBodyModifiers,
) -> Result<(), W::Error> {
    if modifiers.friction_override.is_none() {
        return living_air::travel_in_air(world, input);
    }
    let mut wrapped = FrictionOverrideWorld {
        inner: world,
        modifiers,
    };
    living_air::travel_in_air(&mut wrapped, input)
}

/// Mirrors `Weakness.isSubmerged`: `BlockPos.containing(x, y + 0.1, z)`.
pub fn weakness_feet_block(pos: Vec3) -> BlockPos {
    BlockPos::new(
        pos.x.floor() as i32,
        (pos.y + WEAKNESS_FEET_Y_OFFSET).floor() as i32,
        pos.z.floor() as i32,
    )
}

/// Mirrors `Weakness.isSubmerged`: requires water at both `feet` and `feet.above()`.
pub const fn weakness_is_submerged_blocks(feet_is_water: bool, above_feet_is_water: bool) -> bool {
    feet_is_water && above_feet_is_water
}

/// Scalar submersion depth check ("submerged deeper than one block").
pub fn weakness_is_submerged_height(water_submersion: f64) -> bool {
    water_submersion > WEAKNESS_SUBMERSION_THRESHOLD
}

/// Mirrors `Weakness.drownedPowers(Player, DevilFruit)`.
pub const fn weakness_drowned_powers(
    water_weakness_config: bool,
    fruit_water_weakness: bool,
    submerged: bool,
    on_water: bool,
) -> bool {
    water_weakness_config && fruit_water_weakness && submerged && !on_water
}

/// Mirrors `Weakness.suppressesHolder(ItemStack)`.
const fn seastone_item_suppresses_holder(
    is_seastone_item: bool,
    is_jitte: bool,
    is_gear: bool,
    config: SeastoneConfig,
) -> bool {
    is_seastone_item
        && (!is_jitte || !config.jitte_safe_handle)
        && (!is_gear || config.gear_suppresses_holder)
}

/// Mirrors `Weakness.touchesSeastone(Player)`.
pub const fn weakness_touches_seastone(state: SeastoneTouchState, config: SeastoneConfig) -> bool {
    if !config.seastone_weakness {
        return false;
    }
    if seastone_item_suppresses_holder(
        state.main_hand_seastone,
        state.main_hand_is_jitte,
        state.main_hand_is_gear,
        config,
    ) || seastone_item_suppresses_holder(
        state.off_hand_seastone,
        state.off_hand_is_jitte,
        state.off_hand_is_gear,
        config,
    ) || state.has_suppressed_effect
    {
        return true;
    }
    if config.armor_suppresses_fruit_user && state.seastone_armor_pieces > 0 {
        return true;
    }
    state.standing_on_seastone_block || state.inside_seastone_block
}

/// Mirrors `Weakness.applySinking`:
/// `(motion.x * 0.6, Math.min(motion.y, -sink_speed), motion.z * 0.6)`.
pub fn weakness_sinking_velocity(motion: Vec3, sink_speed: f64) -> Vec3 {
    Vec3::new(
        motion.x * WEAKNESS_SINK_HORIZONTAL_DRAG,
        java_min(motion.y, -sink_speed),
        motion.z * WEAKNESS_SINK_HORIZONTAL_DRAG,
    )
}

/// Mirrors `Weakness.tickServer`: returns the 3 suppression debuffs every 10 ticks
/// while suppressed (`Slowness III`, `Weakness II`, `Mining Fatigue II`, 40 ticks each).
pub const fn weakness_tick_effects(
    suppressed: bool,
    tick_count: i32,
) -> Option<[WeaknessDebuff; 3]> {
    if suppressed && tick_count % WEAKNESS_EFFECT_INTERVAL_TICKS == 0 {
        Some([
            WeaknessDebuff {
                kind: WeaknessEffectKind::MovementSlowdown,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_SLOWNESS_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
            WeaknessDebuff {
                kind: WeaknessEffectKind::Weakness,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_WEAKNESS_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
            WeaknessDebuff {
                kind: WeaknessEffectKind::DigSlowdown,
                duration_ticks: WEAKNESS_EFFECT_DURATION_TICKS,
                amplifier: WEAKNESS_MINING_FATIGUE_AMPLIFIER,
                ambient: true,
                visible: false,
                show_icon: true,
            },
        ])
    } else {
        None
    }
}

/// Mirrors `Clear.sees(@Nullable Entity viewer, Entity target, boolean spectatorsSee)`.
pub fn clear_sees(viewer: Option<StealthViewer>, target: &StealthFlags) -> bool {
    if !target.invisible {
        return true;
    }
    let Some(v) = viewer else {
        return false;
    };
    if v.is_same_entity {
        return true;
    }
    if target.spectators_see && v.is_spectator {
        return true;
    }
    if v.uuid == target.cloak_group {
        return true;
    }
    v.is_invisible && v.cloak_group == target.cloak_group
}

/// Mirrors `AirTime.keepAirborne` / `ServerGamePacketListenerImplAccessor#devilfruits$setAboveGroundTickCount(0)`.
pub const fn effective_above_ground_ticks(
    current_ticks: i32,
    modifiers: &EntityBodyModifiers,
) -> i32 {
    if modifiers.reset_above_ground_ticks {
        0
    } else {
        current_ticks
    }
}

/// Mirrors `Knowledge.allows(@Nullable Player, Recipe<?>)` for both `CrafterBlockMixin`
/// (`player_knows = None`) and `CraftingMenuMixin` (`player_knows = Some(...)`).
pub fn check_crafting_gate<'a>(
    required_knowledge: Option<&str>,
    gate_recipes: bool,
    ungated_knowledge: impl IntoIterator<Item = &'a str>,
    player_knows: Option<&dyn Fn(&str) -> bool>,
) -> CraftingGateResult {
    let Some(knowledge_id) = required_knowledge else {
        return CraftingGateResult::Allowed;
    };
    if !gate_recipes {
        return CraftingGateResult::Allowed;
    }
    if ungated_knowledge.into_iter().any(|id| id == knowledge_id) {
        return CraftingGateResult::Allowed;
    }
    match player_knows {
        None => CraftingGateResult::DeniedNoPlayer,
        Some(knows) if knows(knowledge_id) => CraftingGateResult::Allowed,
        Some(_) => CraftingGateResult::DeniedMissingKnowledge,
    }
}

/// Mirrors `CrafterBlockMixin#devilfruits$knowledgeGate` (`Knowledge.allows(null, recipe)`).
pub fn check_crafter_block_gate<'a>(
    required_knowledge: Option<&str>,
    gate_recipes: bool,
    ungated_knowledge: impl IntoIterator<Item = &'a str>,
) -> CraftingGateResult {
    check_crafting_gate(required_knowledge, gate_recipes, ungated_knowledge, None)
}

/// Mirrors `CraftingMenuMixin#devilfruits$knowledgeGate` (`Knowledge.allows(player, recipe)`).
pub fn check_crafting_menu_gate<'a>(
    required_knowledge: Option<&str>,
    gate_recipes: bool,
    ungated_knowledge: impl IntoIterator<Item = &'a str>,
    player_knows: impl Fn(&str) -> bool,
) -> CraftingGateResult {
    check_crafting_gate(
        required_knowledge,
        gate_recipes,
        ungated_knowledge,
        Some(&player_knows),
    )
}

// ---------------------------------------------------------------------------
// Comprehensive unit tests verifying every Java Mixin & constant 1:1
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DATA_VERSION,
        block_collision::{BlockCollisionRegistry, DynamicCollisionWorld, ShulkerCollisionState},
        collision::CachedCollisionRegistry,
        collision_context::{CollisionContext, CollisionEntity},
        dynamic_support::DynamicSupportWorld,
        entity_fluid::{CurrentAccumulator, CurrentTarget, EntityFluidInteraction, FluidTagId},
        entity_obstruction::EntityIdentity,
        fluid::FluidState,
        liquid_collision::LiquidCollisionWorld,
        living_air::LivingAirWorld,
        living_fluid::{
            LivingFluidWorld, TravelFluidTag, travel_in_fluid, travel_in_fluid_with_modifiers,
        },
        nbt::Tag,
        piston_push::{
            effective_push_reaction, ignores_push_with_modifiers,
            matches_sticky_criteria_with_modifiers,
        },
        player_travel::{
            PlayerTravelWorld, is_pushed_by_fluid_with_modifiers, travel_with_modifiers,
        },
        powder_snow_collision::{PowderSnowEntity, PowderSnowWorld},
        registry::{BlockRegistry, BlockStateId},
        swimming::{
            PlayerSwimmingWorld, SwimmingWorld, update_player_with_modifiers, update_with_modifiers,
        },
    };
    use horizon_core::geometry::{Aabb, VoxelShape};
    use std::sync::Arc;

    // -----------------------------------------------------------------------
    // 1. Default vanilla neutrality
    // -----------------------------------------------------------------------

    #[test]
    fn default_modifiers_preserve_vanilla_behavior() {
        let m = EntityBodyModifiers::default();
        assert!(!m.passes_through_leaves);
        assert!(!m.fire_immune);
        assert_eq!(m.mass_tier, EntityMassTier::Normal);
        assert!(!m.immovable);
        assert!(!m.is_immovable());
        assert_eq!(m.mass_kg_override, None);
        assert!(!m.swim_in_lava);
        assert!(!m.devil_fruit_water_weakness);
        assert_eq!(m.water_submersion, 0.0);
        assert!(!m.is_water_weakened());
        assert_eq!(m.effective_sink_speed(), 0.08);
        assert_eq!(m.friction_override, None);
        assert!(!m.reset_above_ground_ticks);
        assert!(!passes_through_leaves(true, true, &m));
        assert!(!effective_fire_immune(false, &m));
        assert!(effective_fire_immune(true, &m));
        assert_eq!(
            effective_piston_push_reaction(PushReaction::PushPull, &m),
            PushReaction::PushPull
        );
        assert!(effective_is_pushed_by_fluid(true, &m));
        assert_eq!(resolve_friction(0.98, &m), 0.98);
        assert_eq!(effective_above_ground_ticks(42, &m), 42);
    }

    // -----------------------------------------------------------------------
    // 2. BlockStateBaseMixin / FormCollision (leaves pass-through)
    // -----------------------------------------------------------------------

    struct RejectingCollisionWorld;
    impl DynamicSupportWorld for RejectingCollisionWorld {
        type Error = std::convert::Infallible;
        fn shulker_is_closed(&mut self, _position: [i32; 3]) -> Result<Option<bool>, Self::Error> {
            Ok(None)
        }
        fn moving_piston_shape(
            &mut self,
            _position: [i32; 3],
        ) -> Result<Option<VoxelShape>, Self::Error> {
            Ok(None)
        }
    }
    impl DynamicCollisionWorld for RejectingCollisionWorld {
        fn shulker_collision_state(
            &mut self,
            _position: [i32; 3],
        ) -> Result<Option<ShulkerCollisionState>, Self::Error> {
            Ok(None)
        }
    }
    impl LiquidCollisionWorld for RejectingCollisionWorld {
        type Error = std::convert::Infallible;
        fn entity_liquid_shape(
            &mut self,
            _entity: EntityIdentity,
        ) -> Result<VoxelShape, Self::Error> {
            Ok(VoxelShape::empty())
        }
        fn fluid(&mut self, _position: BlockPos) -> Result<FluidState, Self::Error> {
            Ok(FluidState::EMPTY)
        }
        fn entity_can_stand(
            &mut self,
            _entity: EntityIdentity,
            _fluid: FluidState,
        ) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }
    impl PowderSnowWorld for RejectingCollisionWorld {
        type Error = std::convert::Infallible;
        fn powder_snow_entity(
            &mut self,
            _entity: EntityIdentity,
        ) -> Result<PowderSnowEntity, Self::Error> {
            Ok(PowderSnowEntity {
                fall_distance: 0.0,
                falling_block: false,
            })
        }
        fn powder_snow_walkable_tag(
            &mut self,
            _entity: EntityIdentity,
        ) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn feet_are_leather_boots(&mut self, _entity: EntityIdentity) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }

    fn build_leaves_collision_fixture() -> (BlockRegistry, Tag) {
        let states = vec![
            Tag::String("minecraft:air".into()),
            Tag::String("minecraft:stone".into()),
            Tag::String("minecraft:oak_leaves".into()),
            Tag::String("minecraft:cherry_leaves".into()),
        ];
        let registry_tag = Tag::compound([
            ("DataVersion", Tag::Int(DATA_VERSION)),
            ("states", Tag::List(states.clone())),
            (
                "defaults",
                Tag::compound([
                    ("minecraft:air", Tag::Int(0)),
                    ("minecraft:stone", Tag::Int(1)),
                    ("minecraft:oak_leaves", Tag::Int(2)),
                    ("minecraft:cherry_leaves", Tag::Int(3)),
                ]),
            ),
        ]);
        let empty = Tag::compound([
            ("canonical", Tag::Byte(1)),
            ("x", Tag::LongArray(vec![0])),
            ("y", Tag::LongArray(vec![0])),
            ("z", Tag::LongArray(vec![0])),
            ("occupied", Tag::ByteArray(vec![])),
            ("cube", Tag::Byte(0)),
            ("cube_axes", Tag::Byte(0)),
            ("discrete_min", Tag::IntArray(vec![0; 3])),
            ("discrete_max", Tag::IntArray(vec![0; 3])),
        ]);
        let coords = Tag::LongArray(vec![0, 1.0f64.to_bits() as i64]);
        let block = Tag::compound([
            ("canonical", Tag::Byte(2)),
            ("x", coords.clone()),
            ("y", coords.clone()),
            ("z", coords),
            ("occupied", Tag::ByteArray(vec![1])),
            ("cube", Tag::Byte(1)),
            ("cube_axes", Tag::Byte(7)),
            ("discrete_min", Tag::IntArray(vec![0; 3])),
            ("discrete_max", Tag::IntArray(vec![1; 3])),
        ]);
        let collision_tag = Tag::compound([
            ("DataVersion", Tag::Int(DATA_VERSION)),
            ("states", Tag::List(states)),
            ("shapes", Tag::List(vec![empty, block])),
            ("cached_shapes", Tag::IntArray(vec![0, 1, 1, 1])),
        ]);
        (BlockRegistry::from_nbt(&registry_tag).unwrap(), collision_tag)
    }

    #[test]
    fn leaves_pass_through_matches_block_state_base_mixin() {
        let (registry, collision_tag) = build_leaves_collision_fixture();
        let cached = Arc::new(CachedCollisionRegistry::from_nbt(&collision_tag, &registry).unwrap());
        let collisions = BlockCollisionRegistry::new(cached).unwrap();

        let stone = BlockStateId(1);
        let oak_leaves = BlockStateId(2);
        let cherry_leaves = BlockStateId(3);
        assert!(!collisions.is_leaves(stone));
        assert!(collisions.is_leaves(oak_leaves));
        assert!(collisions.is_leaves(cherry_leaves));

        let entity_ctx = CollisionContext::for_entity(
            CollisionEntity {
                identity: EntityIdentity(7),
                y: 64.0,
                descending: false,
                main_hand: None,
            },
            false,
        );
        let empty_ctx = CollisionContext::empty(false);
        let pos = BlockPos::new(0, 64, 0);
        let mut world = RejectingCollisionWorld;

        // Vanilla default: leaves are solid full blocks.
        let vanilla = EntityBodyModifiers::default();
        assert!(collisions.may_collide_with_modifiers(oak_leaves, &vanilla));
        let shape = collisions
            .collision_shape_with_modifiers(&mut world, oak_leaves, pos, &entity_ctx, &vanilla)
            .unwrap();
        assert!(shape.is_block());

        // Winged/dragon form with passes_through_leaves = true:
        let form = EntityBodyModifiers {
            passes_through_leaves: true,
            ..EntityBodyModifiers::default()
        };
        assert!(!collisions.may_collide_with_modifiers(oak_leaves, &form));
        assert!(collisions.may_collide_with_modifiers(stone, &form));

        // EntityCollisionContext + LEAVES -> empty shape!
        let leaves_shape = collisions
            .collision_shape_with_modifiers(&mut world, oak_leaves, pos, &entity_ctx, &form)
            .unwrap();
        assert!(leaves_shape.is_empty());

        // Non-entity CollisionContext -> still solid (exact `context instanceof EntityCollisionContext` guard)!
        let no_entity_shape = collisions
            .collision_shape_with_modifiers(&mut world, oak_leaves, pos, &empty_ctx, &form)
            .unwrap();
        assert!(no_entity_shape.is_block());

        // Non-leaves block (stone) -> still solid!
        let stone_shape = collisions
            .collision_shape_with_modifiers(&mut world, stone, pos, &entity_ctx, &form)
            .unwrap();
        assert!(stone_shape.is_block());

        // All 11 vanilla leaves IDs recognized.
        for name in VANILLA_LEAVES_BLOCKS {
            assert!(is_leaves_block_name(name), "expected leaves: {name}");
        }
        assert!(!is_leaves_block_name("minecraft:oak_log"));
    }

    // -----------------------------------------------------------------------
    // 3. EntityMixin / FireProof / EntityMass / PlayerMixin / PistonPush
    // -----------------------------------------------------------------------

    #[test]
    fn entity_mass_matches_java_constants_and_immovability() {
        // Player hitbox (0.6 x 1.8): volume = 0.648 -> ratio = 1.0 -> 70.0 kg.
        let ratio = entity_mass_hitbox_ratio(0.6, 1.8);
        assert!((ratio - 1.0).abs() < 1e-12);
        assert!((entity_mass_kg(None, 0.6, 1.8) - 70.0).abs() < 1e-12);

        // Tiny entity clamps ratio to 0.5 -> 35.0 kg.
        assert!((entity_mass_hitbox_ratio(0.1, 0.1) - 0.5).abs() < 1e-12);
        assert!((entity_mass_kg(None, 0.1, 0.1) - 35.0).abs() < 1e-12);

        // Giant entity clamps ratio to 40.0 -> 2800.0 kg.
        assert!((entity_mass_hitbox_ratio(10.0, 10.0) - 40.0).abs() < 1e-12);
        assert!((entity_mass_kg(None, 10.0, 10.0) - 2800.0).abs() < 1e-12);

        // Explicit provider overrides hitbox formula (e.g. Champion 250 kg, Kilo-Kilo 1 kg / 10000 kg).
        assert_eq!(entity_mass_kg(Some(ENTITY_MASS_CHAMPION_KG), 0.6, 1.8), 250.0);
        assert_eq!(entity_mass_kg(Some(ENTITY_MASS_LIGHT_TARGET_KG), 0.6, 1.8), 1.0);
        assert_eq!(entity_mass_kg(Some(ENTITY_MASS_HEAVY_TARGET_KG), 0.6, 1.8), 10000.0);
        // NaN provider falls back to hitbox ratio (`Double.isNaN(kg)`).
        assert!((entity_mass_kg(Some(f64::NAN), 0.6, 1.8) - 70.0).abs() < 1e-12);

        // Immovable threshold = 3000.0 kg (`WeightManager.IMMOVABLE_KG`).
        assert!(!is_mass_immovable(2999.99, true));
        assert!(is_mass_immovable(3000.0, true));
        assert!(is_mass_immovable(10000.0, true));
        assert!(!is_mass_immovable(10000.0, false));

        assert_eq!(mass_tier_from_kg(1.0, true), EntityMassTier::Light);
        assert_eq!(mass_tier_from_kg(70.0, true), EntityMassTier::Normal);
        assert_eq!(mass_tier_from_kg(250.0, true), EntityMassTier::Heavy);
        assert_eq!(mass_tier_from_kg(3000.0, true), EntityMassTier::Immovable);
        assert_eq!(mass_tier_from_kg(3000.0, false), EntityMassTier::Heavy);
    }

    #[test]
    fn immovable_body_blocks_pistons_and_fluid_currents() {
        let heavy = EntityBodyModifiers {
            mass_tier: EntityMassTier::Immovable,
            ..EntityBodyModifiers::default()
        };
        assert!(heavy.is_immovable());
        assert_eq!(
            effective_push_reaction(PushReaction::PushPull, &heavy),
            PushReaction::IgnoreEntity
        );
        assert!(ignores_push_with_modifiers(false, &heavy));
        assert!(!is_pushed_by_fluid_with_modifiers(true, &heavy));

        // Honey sticky drag ignores immovable entities (PushReaction::IgnoreEntity != PushPull).
        let area = Aabb::new(Vec3::ZERO, Vec3::new(1.0, 1.5, 1.0));
        let on_top = Vec3::new(0.5, 0.0, 0.5);
        assert!(matches_sticky_criteria_with_modifiers(
            area,
            PushReaction::PushPull,
            true,
            false,
            on_top,
            &EntityBodyModifiers::default(),
        ));
        assert!(!matches_sticky_criteria_with_modifiers(
            area,
            PushReaction::PushPull,
            true,
            false,
            on_top,
            &heavy,
        ));

        // Fluid current accumulator does NOT push immovable entities.
        struct MockCurrentTarget {
            added: Vec3,
        }
        impl CurrentTarget for MockCurrentTarget {
            type Error = std::convert::Infallible;
            fn is_player_class(&self) -> bool {
                true
            }
            fn velocity(&mut self) -> Result<Vec3, Self::Error> {
                Ok(Vec3::ZERO)
            }
            fn add_delta_movement(&mut self, delta: Vec3) -> Result<(), Self::Error> {
                self.added = self.added.add_vector(delta);
                Ok(())
            }
        }

        let mut acc = CurrentAccumulator::default();
        acc.accumulate(Vec3::new(1.0, 0.0, 0.0));

        let mut normal_target = MockCurrentTarget { added: Vec3::ZERO };
        acc.apply_to_with_modifiers(&mut normal_target, 0.014, &EntityBodyModifiers::default())
            .unwrap();
        assert!((normal_target.added.x - 0.014).abs() < 1e-12);

        let mut heavy_target = MockCurrentTarget { added: Vec3::ZERO };
        acc.apply_to_with_modifiers(&mut heavy_target, 0.014, &heavy)
            .unwrap();
        assert_eq!(heavy_target.added, Vec3::ZERO);

        // EntityFluidInteraction wrapper also respects immovability.
        let interaction = EntityFluidInteraction::new([FluidTagId(1)]);
        interaction
            .apply_current_with_modifiers(&mut heavy_target, FluidTagId(1), 0.014, &heavy)
            .unwrap();
        assert_eq!(heavy_target.added, Vec3::ZERO);
    }

    #[test]
    fn fire_immunity_mirrors_entity_mixin() {
        let fireproof = EntityBodyModifiers {
            fire_immune: true,
            ..EntityBodyModifiers::default()
        };
        assert!(effective_fire_immune(false, &fireproof));
        assert!(effective_fire_immune(true, &fireproof));
    }

    // -----------------------------------------------------------------------
    // 4. EntitySwimMixin & LivingEntityFluidSwimMixin (FluidSwim)
    // -----------------------------------------------------------------------

    struct MockSwimWorld {
        swimming: bool,
        sprinting: bool,
        in_water: bool,
        under_water: bool,
        passenger: bool,
        in_lava: bool,
        eye_in_lava: bool,
        flying: bool,
        living: bool,
    }
    impl SwimmingWorld for MockSwimWorld {
        type Error = std::convert::Infallible;
        fn is_swimming(&mut self) -> Result<bool, Self::Error> {
            Ok(self.swimming)
        }
        fn is_sprinting(&mut self) -> Result<bool, Self::Error> {
            Ok(self.sprinting)
        }
        fn is_in_water(&mut self) -> Result<bool, Self::Error> {
            Ok(self.in_water)
        }
        fn is_under_water(&mut self) -> Result<bool, Self::Error> {
            Ok(self.under_water)
        }
        fn is_passenger(&mut self) -> Result<bool, Self::Error> {
            Ok(self.passenger)
        }
        fn was_eye_in_water(&self) -> bool {
            self.under_water
        }
        fn block_position(&self) -> BlockPos {
            BlockPos::new(0, 64, 0)
        }
        fn water_at(&mut self, _position: BlockPos) -> Result<bool, Self::Error> {
            Ok(self.in_water)
        }
        fn set_swimming(&mut self, value: bool) -> Result<(), Self::Error> {
            self.swimming = value;
            Ok(())
        }
        fn is_in_lava(&mut self) -> Result<bool, Self::Error> {
            Ok(self.in_lava)
        }
        fn is_eye_in_lava(&mut self) -> Result<bool, Self::Error> {
            Ok(self.eye_in_lava)
        }
        fn is_living(&self) -> bool {
            self.living
        }
    }
    impl PlayerSwimmingWorld for MockSwimWorld {
        fn flying_ability(&self) -> bool {
            self.flying
        }
    }

    #[test]
    fn lava_swimming_state_transitions_match_entity_swim_mixin() {
        let lava_mod = EntityBodyModifiers {
            swim_in_lava: true,
            ..EntityBodyModifiers::default()
        };

        // Starting to swim in lava requires eyes in lava (`wasSwimming == false`).
        let mut world = MockSwimWorld {
            swimming: false,
            sprinting: true,
            in_water: false,
            under_water: false,
            passenger: false,
            in_lava: true,
            eye_in_lava: false,
            flying: false,
            living: true,
        };
        update_player_with_modifiers(&mut world, &lava_mod).unwrap();
        assert!(!world.swimming, "eyes not in lava yet: cannot start swimming");

        world.eye_in_lava = true;
        update_player_with_modifiers(&mut world, &lava_mod).unwrap();
        assert!(world.swimming, "eyes in lava + sprinting -> starts lava swim");

        // Staying swimming only requires `isInLava()` (`wasSwimming == true`), even if eyes poke above surface!
        world.eye_in_lava = false;
        update_player_with_modifiers(&mut world, &lava_mod).unwrap();
        assert!(world.swimming, "already swimming + still in lava -> stays swimming");

        // Creative flying disables lava swimming (`!(p.getAbilities().flying)`).
        world.flying = true;
        update_player_with_modifiers(&mut world, &lava_mod).unwrap();
        assert!(!world.swimming);

        // Non-living entities never start lava swimming (`e instanceof LivingEntity`).
        world.flying = false;
        world.eye_in_lava = true;
        world.living = false;
        update_with_modifiers(&mut world, &lava_mod).unwrap();
        assert!(!world.swimming);

        // Devil Fruit water weakness forces swimming off when submerged deeper than 1 block.
        let drowned = EntityBodyModifiers {
            devil_fruit_water_weakness: true,
            water_submersion: 1.5,
            ..EntityBodyModifiers::default()
        };
        world.living = true;
        world.in_water = true;
        world.under_water = true;
        world.in_lava = false;
        world.eye_in_lava = false;
        world.swimming = true;
        update_player_with_modifiers(&mut world, &drowned).unwrap();
        assert!(!world.swimming);
    }

    #[test]
    fn fluid_swim_takes_over_checks_all_seven_guards() {
        let lava_mod = EntityBodyModifiers {
            swim_in_lava: true,
            ..EntityBodyModifiers::default()
        };
        let base = FluidSwimContext {
            controlled_by_local_instance: true,
            in_lava: true,
            in_water: false,
            affected_by_fluids: true,
            fall_flying: false,
            player_flying: false,
        };
        assert!(fluid_swim_takes_over(base, &lava_mod));
        assert!(!fluid_swim_takes_over(base, &EntityBodyModifiers::default()));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                controlled_by_local_instance: false,
                ..base
            },
            &lava_mod
        ));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                in_lava: false,
                ..base
            },
            &lava_mod
        ));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                in_water: true,
                ..base
            },
            &lava_mod
        ));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                affected_by_fluids: false,
                ..base
            },
            &lava_mod
        ));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                fall_flying: true,
                ..base
            },
            &lava_mod
        ));
        assert!(!fluid_swim_takes_over(
            FluidSwimContext {
                player_flying: true,
                ..base
            },
            &lava_mod
        ));
    }

    #[derive(Clone)]
    struct MockFluidWorld {
        velocity: Vec3,
        y: f64,
        on_ground: bool,
        in_water: bool,
        in_lava: bool,
        sprinting: bool,
        shift_down: bool,
        swim_speed: f64,
        water_efficiency: f64,
        dolphins_grace: bool,
        block_friction: f32,
        gravity: f64,
        horizontal_collision: bool,
        on_climbable: bool,
        is_free_out: bool,
        ridden_float_calls: usize,
    }
    impl Default for MockFluidWorld {
        fn default() -> Self {
            Self {
                velocity: Vec3::ZERO,
                y: 64.0,
                on_ground: false,
                in_water: false,
                in_lava: true,
                sprinting: false,
                shift_down: false,
                swim_speed: 1.0,
                water_efficiency: 0.0,
                dolphins_grace: false,
                block_friction: 0.6,
                gravity: 0.08,
                horizontal_collision: false,
                on_climbable: false,
                is_free_out: false,
                ridden_float_calls: 0,
            }
        }
    }
    impl LivingAirWorld for MockFluidWorld {
        type Error = std::convert::Infallible;
        fn movement_support_position(&mut self) -> Result<BlockPos, Self::Error> {
            Ok(BlockPos::new(0, 63, 0))
        }
        fn on_ground(&mut self) -> Result<bool, Self::Error> {
            Ok(self.on_ground)
        }
        fn block_friction(&mut self, _position: BlockPos) -> Result<f32, Self::Error> {
            Ok(self.block_friction)
        }
        fn friction_modifier(&mut self) -> Result<f64, Self::Error> {
            Ok(1.0)
        }
        fn speed(&mut self) -> Result<f32, Self::Error> {
            Ok(0.1)
        }
        fn flying_speed(&mut self) -> Result<f32, Self::Error> {
            Ok(0.02)
        }
        fn yaw(&mut self) -> Result<f32, Self::Error> {
            Ok(0.0)
        }
        fn velocity(&self) -> Vec3 {
            self.velocity
        }
        fn set_velocity(&mut self, velocity: Vec3) -> Result<(), Self::Error> {
            self.velocity = velocity;
            Ok(())
        }
        fn on_climbable(&mut self) -> Result<bool, Self::Error> {
            Ok(self.on_climbable)
        }
        fn reset_fall_distance(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn in_scaffolding(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn suppressing_ladder_slide(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn is_player_class(&self) -> bool {
            true
        }
        fn move_self(&mut self, movement: Vec3) -> Result<(), Self::Error> {
            self.y += movement.y;
            Ok(())
        }
        fn horizontal_collision(&self) -> bool {
            self.horizontal_collision
        }
        fn jumping(&self) -> bool {
            false
        }
        fn was_in_powder_snow(&self) -> bool {
            false
        }
        fn can_walk_powder_snow(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn levitation_amplifier(&mut self) -> Result<Option<i32>, Self::Error> {
            Ok(None)
        }
        fn is_client_side(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn has_chunk_at(&mut self, _position: BlockPos) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn effective_gravity(&mut self) -> Result<f64, Self::Error> {
            Ok(self.gravity)
        }
        fn y(&self) -> f64 {
            self.y
        }
        fn minimum_y(&mut self) -> Result<i32, Self::Error> {
            Ok(-64)
        }
        fn discard_friction(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn air_drag_modifier(&mut self) -> Result<f64, Self::Error> {
            Ok(1.0)
        }
        fn omnidirectional_air_mover(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }
    impl LivingFluidWorld for MockFluidWorld {
        fn in_water(&mut self) -> Result<bool, Self::Error> {
            Ok(self.in_water)
        }
        fn sprinting(&mut self) -> Result<bool, Self::Error> {
            Ok(self.sprinting)
        }
        fn water_slow_down(&mut self) -> Result<f32, Self::Error> {
            Ok(0.8)
        }
        fn water_movement_efficiency(&mut self) -> Result<f64, Self::Error> {
            Ok(self.water_efficiency)
        }
        fn dolphins_grace(&mut self) -> Result<bool, Self::Error> {
            Ok(self.dolphins_grace)
        }
        fn fluid_height(&mut self, _tag: TravelFluidTag) -> Result<f64, Self::Error> {
            Ok(0.8)
        }
        fn fluid_jump_threshold(&mut self) -> Result<f64, Self::Error> {
            Ok(0.4)
        }
        fn is_free(&mut self, _offset: Vec3) -> Result<bool, Self::Error> {
            Ok(self.is_free_out)
        }
        fn is_vehicle(&mut self) -> Result<bool, Self::Error> {
            self.ridden_float_calls += 1;
            Ok(false)
        }
        fn can_float_while_ridden(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn swim_speed(&mut self) -> Result<f64, Self::Error> {
            Ok(self.swim_speed)
        }
        fn shift_key_down(&mut self) -> Result<bool, Self::Error> {
            Ok(self.shift_down)
        }
        fn in_lava(&mut self) -> Result<bool, Self::Error> {
            Ok(self.in_lava)
        }
    }

    #[test]
    fn travel_in_lava_as_water_matches_fluid_swim_math() {
        let lava_mod = EntityBodyModifiers {
            swim_in_lava: true,
            ..EntityBodyModifiers::default()
        };

        // 1. Sprinting forward in lava with SWIM_SPEED=1.5 and shift diving off ground.
        let mut lava_swimmer = MockFluidWorld {
            sprinting: true,
            shift_down: true,
            swim_speed: 1.5,
            ..MockFluidWorld::default()
        };
        travel_in_fluid_with_modifiers(&mut lava_swimmer, Vec3::new(0.0, 0.0, 1.0), &lava_mod)
            .unwrap();

        // Expected: speed = 0.02 * 1.5 = 0.03 along +Z (yaw = 0 -> cos=1, sin=0).
        // drag = 0.9f32 (sprinting).
        // v.z = 0.03 * 0.9f32 = 0.027.
        // shift dive adds -0.04 to y -> v.y = -0.04 * 0.8f32 = -0.032.
        // sprinting -> fluid_falling_adjusted_movement does NOT apply gravity.
        let expected_z = f64::from(0.02f32 * 1.5f32) * f64::from(0.9f32);
        let expected_y = -0.04 * f64::from(0.8f32);
        assert!(
            (lava_swimmer.velocity.z - expected_z).abs() < 1e-12,
            "got {}, expected {}",
            lava_swimmer.velocity.z,
            expected_z
        );
        assert!(
            (lava_swimmer.velocity.y - expected_y).abs() < 1e-12,
            "got {}, expected {}",
            lava_swimmer.velocity.y,
            expected_y
        );
        // `LivingEntityFluidSwimMixin` cancels travel before `float_in_liquid_while_ridden`.
        assert_eq!(lava_swimmer.ridden_float_calls, 0);

        // 2. Vanilla default in lava uses 0.5 lava drag and -gravity/4 (NOT water drag).
        let mut vanilla_lava = MockFluidWorld {
            sprinting: true,
            shift_down: true,
            swim_speed: 1.5,
            ..MockFluidWorld::default()
        };
        let mut direct_vanilla = vanilla_lava.clone();
        travel_in_fluid_with_modifiers(
            &mut vanilla_lava,
            Vec3::new(0.0, 0.0, 1.0),
            &EntityBodyModifiers::default(),
        )
        .unwrap();
        travel_in_fluid(&mut direct_vanilla, Vec3::new(0.0, 0.0, 1.0)).unwrap();
        assert_eq!(vanilla_lava.velocity, direct_vanilla.velocity);
        assert_eq!(vanilla_lava.ridden_float_calls, 1);

        // 3. Slow falling gravity cap (`FluidSwim.travelAsWater`: Math.min(gravity, 0.01)).
        assert_eq!(fluid_swim_effective_gravity(0.08, -0.01, true), 0.01);
        assert_eq!(fluid_swim_effective_gravity(0.08, 0.05, true), 0.08);
        assert_eq!(fluid_swim_effective_gravity(0.005, -0.01, true), 0.005);
    }

    // -----------------------------------------------------------------------
    // 5. LivingEntityFrictionMixin / Friction / HieEvents
    // -----------------------------------------------------------------------

    #[test]
    fn friction_override_and_hie_ice_match_java() {
        // `HieEvents.friction`:
        // Not on ice tag & no ice sheet -> None (vanilla preserved)
        assert_eq!(hie_ice_friction_override(false, false, true, false), None);
        // Hie-Hie user with userSlidesOnIce=false -> 0.6 (normal ground grip!)
        assert_eq!(
            hie_ice_friction_override(true, false, true, false),
            Some(FRICTION_GROUND)
        );
        assert_eq!(
            hie_ice_friction_override(false, true, true, false),
            Some(FRICTION_GROUND)
        );
        // Hie-Hie user with userSlidesOnIce=true on ice sheet -> 0.98
        assert_eq!(
            hie_ice_friction_override(false, true, true, true),
            Some(FRICTION_ICE)
        );
        // Non-user on ice sheet -> 0.98; non-user on normal ice block without sheet -> None (vanilla)
        assert_eq!(
            hie_ice_friction_override(false, true, false, false),
            Some(FRICTION_ICE)
        );
        assert_eq!(hie_ice_friction_override(true, false, false, false), None);

        // `Friction.of` chain takes first non-NaN value.
        assert_eq!(resolve_friction_chain(0.98, [f32::NAN, 0.6, 0.8]), 0.6);
        assert_eq!(resolve_friction_chain(0.98, [f32::NAN, f32::NAN]), 0.98);

        // `travel_in_air_with_modifiers` actually overrides ground block friction on ice!
        let mut vanilla_on_ice = MockFluidWorld {
            on_ground: true,
            block_friction: 0.98,
            ..MockFluidWorld::default()
        };
        let mut gripped_on_ice = vanilla_on_ice.clone();
        travel_in_air_with_modifiers(
            &mut vanilla_on_ice,
            Vec3::new(0.0, 0.0, 1.0),
            &EntityBodyModifiers::default(),
        )
        .unwrap();
        let hie_grip = EntityBodyModifiers {
            friction_override: Some(FRICTION_GROUND),
            ..EntityBodyModifiers::default()
        };
        travel_in_air_with_modifiers(&mut gripped_on_ice, Vec3::new(0.0, 0.0, 1.0), &hie_grip)
            .unwrap();

        // With friction 0.6 vs 0.98, ground acceleration is much higher (0.1 vs 0.1 * 0.216/0.98^3)
        // and final horizontal velocity differs substantially.
        assert!(
            (gripped_on_ice.velocity.z - vanilla_on_ice.velocity.z).abs() > 0.01,
            "grip={}, ice={}",
            gripped_on_ice.velocity.z,
            vanilla_on_ice.velocity.z
        );
        // Verify exact value for friction = 0.6: speed = 0.1, drag = 0.91 * 0.6 = 0.546.
        let expected_grip_z = f64::from(0.1f32) * f64::from(0.6f32 * 0.91f32);
        assert!((gripped_on_ice.velocity.z - expected_grip_z).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // 6. Weakness (sinking, submersion threshold, debuffs, seastone, AirTime)
    // -----------------------------------------------------------------------

    struct MockPlayerTravelWorld {
        position: Vec3,
        velocity: Vec3,
        swimming: bool,
        sprinting: bool,
        flying: bool,
        spectator: bool,
        water_blocks: Option<(bool, bool)>,
        above_ground_resets: usize,
    }
    impl PlayerTravelWorld for MockPlayerTravelWorld {
        type Error = std::convert::Infallible;
        fn is_passenger(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn is_swimming(&mut self) -> Result<bool, Self::Error> {
            Ok(self.swimming)
        }
        fn look_angle(&mut self) -> Result<Vec3, Self::Error> {
            Ok(Vec3::new(0.0, 0.0, 1.0))
        }
        fn jumping(&self) -> bool {
            false
        }
        fn position(&self) -> Vec3 {
            self.position
        }
        fn fluid_is_empty(&mut self, _position: BlockPos) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn velocity(&self) -> Vec3 {
            self.velocity
        }
        fn set_velocity(&mut self, velocity: Vec3) -> Result<(), Self::Error> {
            self.velocity = velocity;
            Ok(())
        }
        fn query_flying_ability(&mut self) -> Result<bool, Self::Error> {
            Ok(self.flying)
        }
        fn flying_ability(&self) -> bool {
            self.flying
        }
        fn ability_flying_speed(&mut self) -> Result<f32, Self::Error> {
            Ok(0.05)
        }
        fn sprinting(&mut self) -> Result<bool, Self::Error> {
            Ok(self.sprinting)
        }
        fn living_travel(&mut self, _input: Vec3) -> Result<(), Self::Error> {
            Ok(())
        }
        fn is_spectator(&self) -> bool {
            self.spectator
        }
        fn set_swimming(&mut self, value: bool) -> Result<(), Self::Error> {
            self.swimming = value;
            Ok(())
        }
        fn set_sprinting(&mut self, value: bool) -> Result<(), Self::Error> {
            self.sprinting = value;
            Ok(())
        }
        fn water_at(&mut self, position: BlockPos) -> Result<Option<bool>, Self::Error> {
            let feet = weakness_feet_block(self.position);
            Ok(self.water_blocks.map(|(feet_w, above_w)| {
                if position == feet {
                    feet_w
                } else if position == BlockPos::new(feet.x, feet.y + 1, feet.z) {
                    above_w
                } else {
                    false
                }
            }))
        }
        fn reset_above_ground_ticks(&mut self) -> Result<(), Self::Error> {
            self.above_ground_resets += 1;
            Ok(())
        }
    }

    #[test]
    fn weakness_sinking_submersion_debuffs_and_seastone_match_java() {
        // Feet sampling offset is +0.1 (`BlockPos.containing(x, y + 0.1, z)`).
        assert_eq!(
            weakness_feet_block(Vec3::new(1.5, 62.95, -2.2)),
            BlockPos::new(1, 63, -3)
        );
        assert_eq!(
            weakness_feet_block(Vec3::new(1.5, 62.85, -2.2)),
            BlockPos::new(1, 62, -3)
        );

        // Scalar threshold: strictly deeper than 1.0 block.
        assert!(!weakness_is_submerged_height(1.0));
        assert!(weakness_is_submerged_height(1.01));

        // Exact sinking math: `(x * 0.6, min(y, -0.08), z * 0.6)`.
        let sunk = weakness_sinking_velocity(Vec3::new(0.5, 0.2, -0.4), WEAKNESS_DEFAULT_SINK_SPEED);
        assert!((sunk.x - 0.3).abs() < 1e-12);
        assert!((sunk.y - (-0.08)).abs() < 1e-12);
        assert!((sunk.z - (-0.24)).abs() < 1e-12);

        // Already falling faster than -0.08 keeps the faster downward speed.
        let fast_fall =
            weakness_sinking_velocity(Vec3::new(0.0, -0.35, 0.0), WEAKNESS_DEFAULT_SINK_SPEED);
        assert!((fast_fall.y - (-0.35)).abs() < 1e-12);

        // `player_travel::travel_with_modifiers` applies sinking, clears swimming & sprinting,
        // and resets aboveGroundTickCount when requested.
        let mut player = MockPlayerTravelWorld {
            position: Vec3::new(0.5, 60.0, 0.5),
            velocity: Vec3::new(0.5, 0.1, -0.5),
            swimming: true,
            sprinting: true,
            flying: false,
            spectator: false,
            water_blocks: Some((true, true)),
            above_ground_resets: 0,
        };
        let mods = EntityBodyModifiers {
            devil_fruit_water_weakness: true,
            water_submersion: 1.8,
            reset_above_ground_ticks: true,
            ..EntityBodyModifiers::default()
        };
        travel_with_modifiers(&mut player, Vec3::ZERO, &mods).unwrap();
        assert!((player.velocity.x - 0.3).abs() < 1e-12);
        assert!((player.velocity.y - (-0.08)).abs() < 1e-12);
        assert!((player.velocity.z - (-0.3)).abs() < 1e-12);
        assert!(!player.swimming);
        assert!(!player.sprinting);
        assert_eq!(player.above_ground_resets, 1);

        // Shallow water (feet in water, block above is air) does NOT sink!
        let mut shallow = MockPlayerTravelWorld {
            position: Vec3::new(0.5, 60.0, 0.5),
            velocity: Vec3::new(0.5, 0.1, -0.5),
            swimming: false,
            sprinting: true,
            flying: false,
            spectator: false,
            water_blocks: Some((true, false)),
            above_ground_resets: 0,
        };
        travel_with_modifiers(&mut shallow, Vec3::ZERO, &mods).unwrap();
        assert!((shallow.velocity.x - 0.5).abs() < 1e-12);
        assert!((shallow.velocity.y - 0.1).abs() < 1e-12);
        assert!(shallow.sprinting);

        // `on_water_override` (Yomi-Yomi running across water) prevents sinking.
        let on_water = EntityBodyModifiers {
            on_water_override: true,
            ..mods
        };
        let mut runner = MockPlayerTravelWorld {
            position: Vec3::new(0.5, 60.0, 0.5),
            velocity: Vec3::new(0.5, 0.0, 0.0),
            swimming: false,
            sprinting: true,
            flying: false,
            spectator: false,
            water_blocks: Some((true, true)),
            above_ground_resets: 0,
        };
        travel_with_modifiers(&mut runner, Vec3::ZERO, &on_water).unwrap();
        assert!((runner.velocity.x - 0.5).abs() < 1e-12);
        assert!(runner.sprinting);

        // `Weakness.tickServer`: debuffs refreshed every 10 ticks (duration 40, Slowness III / Weakness II / Mining Fatigue II).
        assert!(weakness_tick_effects(false, 20).is_none());
        assert!(weakness_tick_effects(true, 15).is_none());
        let debuffs = weakness_tick_effects(true, 20).expect("debuffs on tick % 10 == 0");
        assert_eq!(debuffs[0].kind, WeaknessEffectKind::MovementSlowdown);
        assert_eq!(debuffs[0].duration_ticks, 40);
        assert_eq!(debuffs[0].amplifier, 2);
        assert!(debuffs[0].ambient && !debuffs[0].visible && debuffs[0].show_icon);
        assert_eq!(debuffs[1].kind, WeaknessEffectKind::Weakness);
        assert_eq!(debuffs[1].amplifier, 1);
        assert_eq!(debuffs[2].kind, WeaknessEffectKind::DigSlowdown);
        assert_eq!(debuffs[2].amplifier, 1);

        // `Weakness.touchesSeastone`: Jitte safe handle does NOT suppress holder by default!
        let cfg = SeastoneConfig::default();
        let jitte_held = SeastoneTouchState {
            main_hand_seastone: true,
            main_hand_is_jitte: true,
            main_hand_is_gear: true,
            ..SeastoneTouchState::default()
        };
        assert!(!weakness_touches_seastone(jitte_held, cfg));
        assert!(weakness_touches_seastone(
            jitte_held,
            SeastoneConfig {
                jitte_safe_handle: false,
                ..cfg
            }
        ));
        let seastone_armor = SeastoneTouchState {
            seastone_armor_pieces: 1,
            ..SeastoneTouchState::default()
        };
        assert!(weakness_touches_seastone(seastone_armor, cfg));
    }

    // -----------------------------------------------------------------------
    // 7. ClearEntityMixin & ClearLivingEntityMixin (StealthFlags)
    // -----------------------------------------------------------------------

    #[test]
    fn stealth_flags_match_clear_mixins_and_suke_events() {
        let group_a = [1u8; 16];
        let group_b = [2u8; 16];
        let flags = StealthFlags::suke_clear_hidden(group_a, true, false);
        assert_eq!(flags.clear_bits(), CLEAR_FLAG_INVISIBLE | CLEAR_FLAG_HIDDEN_ITEMS);
        assert!(flags.is_invisible(false, false));
        assert!(!flags.is_invisible(false, true)); // `self == Clear.drawnSolid`

        // Outsider cannot see target; spectator CAN see (spectatorsSee = true);
        // group owner & cloaked group ally CAN see; different cloaked group CANNOT see.
        let outsider = StealthViewer {
            uuid: [9u8; 16],
            ..StealthViewer::default()
        };
        let spectator = StealthViewer {
            uuid: [8u8; 16],
            is_spectator: true,
            ..StealthViewer::default()
        };
        let owner = StealthViewer {
            uuid: group_a,
            ..StealthViewer::default()
        };
        let ally = StealthViewer {
            uuid: [7u8; 16],
            is_invisible: true,
            cloak_group: group_a,
            ..StealthViewer::default()
        };
        let enemy_cloaked = StealthViewer {
            uuid: [6u8; 16],
            is_invisible: true,
            cloak_group: group_b,
            ..StealthViewer::default()
        };
        assert!(flags.is_invisible_to(false, false, None));
        assert!(flags.is_invisible_to(false, false, Some(outsider)));
        assert!(!flags.is_invisible_to(false, false, Some(spectator)));
        assert!(!flags.is_invisible_to(false, false, Some(owner)));
        assert!(!flags.is_invisible_to(false, false, Some(ally)));
        assert!(flags.is_invisible_to(false, false, Some(enemy_cloaked)));

        // Particle, fire, glow, and footstep suppression:
        assert!(!flags.is_currently_glowing(true));
        assert!(!flags.display_fire_animation(true));
        assert!(!flags.can_spawn_sprint_particle(true));
        assert!(!flags.can_spawn_water_splash_particle());
        assert!(!flags.can_spawn_potion_particles());
        assert!(!flags.can_spawn_eating_particles(true, false));
        assert!(flags.can_spawn_eating_particles(false, true)); // server side allowed
        assert!((flags.effective_footstep_volume(0.8) - 0.4).abs() < 1e-6);

        // Hidden items alone (visible player holding hidden weapon): hides eating crumbs on client when itemsHiddenHere.
        let hidden_item_only = StealthFlags::from_clear_bits(
            CLEAR_FLAG_HIDDEN_ITEMS,
            CLEAR_NOBODY_UUID,
            false,
            SUKE_DEFAULT_MUFFLE_STEPS,
            true,
        );
        assert!(!hidden_item_only.invisible);
        assert!(hidden_item_only.can_spawn_eating_particles(true, false));
        assert!(!hidden_item_only.can_spawn_eating_particles(true, true));

        // Mob visibility factor (`0.07` default).
        assert!((flags.effective_mob_visibility(1.0, Some(outsider), SUKE_DEFAULT_MOB_VISIBILITY) - 0.07).abs() < 1e-12);
        assert!((flags.effective_mob_visibility(0.05, Some(outsider), SUKE_DEFAULT_MOB_VISIBILITY) - 0.05).abs() < 1e-12);
        assert!((flags.effective_mob_visibility(1.0, Some(ally), SUKE_DEFAULT_MOB_VISIBILITY) - 1.0).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // 8. CrafterBlockMixin & CraftingMenuMixin (Knowledge gate)
    // -----------------------------------------------------------------------

    #[test]
    fn crafting_gate_matches_knowledge_allows() {
        let locked = Some("devilfruits:seastone_tools");
        let empty_ungated: [&str; 0] = [];

        // Unlocked recipe: always allowed (even in CrafterBlock).
        assert_eq!(
            check_crafter_block_gate(None, true, empty_ungated),
            CraftingGateResult::Allowed
        );

        // Locked recipe in CrafterBlock (`player == null`): denied when gated!
        assert_eq!(
            check_crafter_block_gate(locked, true, empty_ungated),
            CraftingGateResult::DeniedNoPlayer
        );
        // Allowed when `gateRecipes == false` or listed in `ungatedKnowledge`.
        assert_eq!(
            check_crafter_block_gate(locked, false, empty_ungated),
            CraftingGateResult::Allowed
        );
        assert_eq!(
            check_crafter_block_gate(locked, true, ["devilfruits:seastone_tools"]),
            CraftingGateResult::Allowed
        );

        // Locked recipe in CraftingMenu: allowed iff player learned the knowledge.
        assert_eq!(
            check_crafting_menu_gate(locked, true, empty_ungated, |_| false),
            CraftingGateResult::DeniedMissingKnowledge
        );
        assert_eq!(
            check_crafting_menu_gate(locked, true, empty_ungated, |id| {
                id == "devilfruits:seastone_tools"
            }),
            CraftingGateResult::Allowed
        );
    }

    // -----------------------------------------------------------------------
    // 9. WalkAnimationStateAccessor & ServerGamePacketListenerImplAccessor
    // -----------------------------------------------------------------------

    #[test]
    fn walk_animation_freeze_and_air_time_reset_match_accessors() {
        let frozen = WalkAnimationSnapshot::frozen_at(3.25, 0.75);
        assert_eq!(frozen.position, 3.25);
        assert_eq!(frozen.speed, 0.75);
        assert_eq!(frozen.speed_old, 0.75);

        let keep_airborne = EntityBodyModifiers {
            reset_above_ground_ticks: true,
            ..EntityBodyModifiers::default()
        };
        assert_eq!(effective_above_ground_ticks(79, &keep_airborne), 0);
        assert_eq!(
            effective_above_ground_ticks(79, &EntityBodyModifiers::default()),
            79
        );
    }
}
