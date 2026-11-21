//! Entity swimming state transitions and the player's flying-ability guard.
use crate::grandline_hooks::EntityBodyModifiers;
use horizon_core::position::BlockPos;
pub trait SwimmingWorld {
    type Error;
    fn is_swimming(&mut self) -> Result<bool, Self::Error>;
    fn is_sprinting(&mut self) -> Result<bool, Self::Error>;
    fn is_in_water(&mut self) -> Result<bool, Self::Error>;
    fn is_under_water(&mut self) -> Result<bool, Self::Error>;
    fn is_passenger(&mut self) -> Result<bool, Self::Error>;
    fn was_eye_in_water(&self) -> bool;
    /// Direct blockPosition field, captured after preceding virtual queries.
    fn block_position(&self) -> BlockPos;
    fn water_at(&mut self, position: BlockPos) -> Result<bool, Self::Error>;
    fn set_swimming(&mut self, value: bool) -> Result<(), Self::Error>;
    /// Whether the entity is touching lava (Grand Line `EntitySwimMixin` / `FluidSwim`).
    fn is_in_lava(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    /// Whether the entity's eyes are inside lava (Grand Line `EntitySwimMixin` / `FluidSwim`).
    fn is_eye_in_lava(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    /// Mirrors `e instanceof LivingEntity` in `FluidSwim.updateSwimming`.
    fn is_living(&self) -> bool {
        true
    }
}
pub trait PlayerSwimmingWorld: SwimmingWorld {
    /// Direct abilities.flying field; do not dispatch getAbilities here.
    fn flying_ability(&self) -> bool;
}
pub fn under_water<W: SwimmingWorld>(world: &mut W) -> Result<bool, W::Error> {
    Ok(world.was_eye_in_water() && world.is_in_water()?)
}
pub fn shallow_water<W: SwimmingWorld>(world: &mut W) -> Result<bool, W::Error> {
    Ok(world.is_in_water()? && !world.is_under_water()?)
}
pub fn update<W: SwimmingWorld>(world: &mut W) -> Result<(), W::Error> {
    let next = if world.is_swimming()? {
        world.is_sprinting()? && world.is_in_water()? && !world.is_passenger()?
    } else {
        world.is_sprinting()?
            && world.is_under_water()?
            && !world.is_passenger()?
            && world.water_at(world.block_position())?
    };
    world.set_swimming(next)
}
pub fn update_player<W: PlayerSwimmingWorld>(world: &mut W) -> Result<(), W::Error> {
    if world.flying_ability() {
        world.set_swimming(false)
    } else {
        update(world)
    }
}
/// Mirrors `EntitySwimMixin` (`FluidSwim.updateSwimming`) and Devil Fruit water weakness
/// on top of vanilla [`update`]. Vanilla-neutral when `modifiers` is default.
pub fn update_with_modifiers<W: SwimmingWorld>(
    world: &mut W,
    modifiers: &EntityBodyModifiers,
) -> Result<(), W::Error> {
    if !modifiers.swim_in_lava && !modifiers.is_water_weakened() {
        return update(world);
    }
    let was_swimming = world.is_swimming()?;
    update(world)?;
    if modifiers.is_water_weakened() {
        return world.set_swimming(false);
    }
    if modifiers.swim_in_lava
        && !world.is_swimming()?
        && world.is_sprinting()?
        && !world.is_passenger()?
        && world.is_living()
    {
        let in_lava = if was_swimming {
            world.is_in_lava()?
        } else {
            world.is_eye_in_lava()?
        };
        if in_lava {
            world.set_swimming(true)?;
        }
    }
    Ok(())
}
/// Player variant of [`update_with_modifiers`], preserving vanilla's flying-ability guard
/// (`!(e instanceof Player p && p.getAbilities().flying)`).
pub fn update_player_with_modifiers<W: PlayerSwimmingWorld>(
    world: &mut W,
    modifiers: &EntityBodyModifiers,
) -> Result<(), W::Error> {
    if world.flying_ability() {
        world.set_swimming(false)
    } else {
        update_with_modifiers(world, modifiers)
    }
}
pub fn fluid_jump_threshold(eye_height: f32) -> f64 {
    if f64::from(eye_height) < 0.4 {
        0.0
    } else {
        0.4
    }
}
