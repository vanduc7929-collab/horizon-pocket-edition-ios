//! Player-specific swimming and ability-flight behavior around living travel.
use crate::grandline_hooks::{
    EntityBodyModifiers, effective_is_pushed_by_fluid, weakness_feet_block,
    weakness_is_submerged_blocks, weakness_sinking_velocity,
};
use horizon_core::{geometry::Vec3, position::BlockPos};
pub trait PlayerTravelWorld {
    type Error;
    fn is_passenger(&mut self) -> Result<bool, Self::Error>;
    fn is_swimming(&mut self) -> Result<bool, Self::Error>;
    fn look_angle(&mut self) -> Result<Vec3, Self::Error>;
    fn jumping(&self) -> bool;
    fn position(&self) -> Vec3;
    fn fluid_is_empty(&mut self, position: BlockPos) -> Result<bool, Self::Error>;
    fn velocity(&self) -> Vec3;
    fn set_velocity(&mut self, velocity: Vec3) -> Result<(), Self::Error>;
    /// Virtual getAbilities(), as distinct from direct abilities-field reads.
    fn query_flying_ability(&mut self) -> Result<bool, Self::Error>;
    fn flying_ability(&self) -> bool;
    fn ability_flying_speed(&mut self) -> Result<f32, Self::Error>;
    fn sprinting(&mut self) -> Result<bool, Self::Error>;
    fn living_travel(&mut self, input: Vec3) -> Result<(), Self::Error>;
    /// Mirrors `Player.isSpectator` for `Weakness.applySinking`.
    fn is_spectator(&self) -> bool {
        false
    }
    /// Mirrors `Entity.setSwimming` for `Weakness.applySinking`.
    fn set_swimming(&mut self, _value: bool) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Mirrors `Entity.setSprinting` for `Weakness.applySinking`.
    fn set_sprinting(&mut self, _value: bool) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Optional block-level water fluid query for exact `Weakness.isSubmerged` (`feet` + `feet.above()`).
    /// Returning `Ok(None)` falls back to `modifiers.water_submersion > 1.0`.
    fn water_at(&mut self, _position: BlockPos) -> Result<Option<bool>, Self::Error> {
        Ok(None)
    }
    /// Mirrors `ServerGamePacketListenerImplAccessor#devilfruits$setAboveGroundTickCount(0)`.
    fn reset_above_ground_ticks(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
pub fn travel<W: PlayerTravelWorld>(world: &mut W, input: Vec3) -> Result<(), W::Error> {
    if world.is_passenger()? {
        return world.living_travel(input);
    }
    if world.is_swimming()? {
        let look_y = world.look_angle()?.y;
        let scale = if look_y < -0.2 { 0.085 } else { 0.06 };
        let apply = if look_y <= 0.0 || world.jumping() {
            true
        } else {
            let p = world.position();
            !world.fluid_is_empty(BlockPos::new(
                p.x.floor() as i32,
                (p.y + 1.0 - 0.1).floor() as i32,
                p.z.floor() as i32,
            ))?
        };
        if apply {
            let v = world.velocity();
            world.set_velocity(Vec3::new(
                v.x + 0.0,
                v.y + (look_y - v.y) * scale,
                v.z + 0.0,
            ))?;
        }
    }
    if world.query_flying_ability()? {
        let y = world.velocity().y;
        world.living_travel(input)?;
        let v = world.velocity();
        world.set_velocity(Vec3::new(v.x, y * 0.6, v.z))?;
    } else {
        world.living_travel(input)?;
    }
    Ok(())
}
pub fn flying_speed<W: PlayerTravelWorld>(world: &mut W) -> Result<f32, W::Error> {
    if world.flying_ability() && !world.is_passenger()? {
        let sprinting = world.sprinting()?;
        let speed = world.ability_flying_speed()?;
        Ok(if sprinting { speed * 2.0 } else { speed })
    } else {
        Ok(if world.sprinting()? {
            0.025999999f32
        } else {
            0.02
        })
    }
}
pub fn affected_by_fluids<W: PlayerTravelWorld>(world: &W) -> bool {
    !world.flying_ability()
}
/// Mirrors `PlayerMixin#devilfruits$immovableBody` (`Player.isPushedByFluid`).
pub fn is_pushed_by_fluid_with_modifiers(
    vanilla: bool,
    modifiers: &EntityBodyModifiers,
) -> bool {
    effective_is_pushed_by_fluid(vanilla, modifiers)
}
/// Mirrors `Weakness.applySinking(Player)`: forces downward sinking, horizontal damping (`* 0.6`),
/// and disables swimming and sprinting when submerged past the water weakness threshold.
/// Returns `true` if sinking was applied.
pub fn apply_water_weakness_sinking<W: PlayerTravelWorld>(
    world: &mut W,
    modifiers: &EntityBodyModifiers,
) -> Result<bool, W::Error> {
    if !modifiers.devil_fruit_water_weakness
        || modifiers.on_water_override
        || world.flying_ability()
        || world.is_spectator()
    {
        return Ok(false);
    }
    let feet = weakness_feet_block(world.position());
    let submerged = if let Some(feet_water) = world.water_at(feet)? {
        let above_water = if feet_water {
            world
                .water_at(BlockPos::new(feet.x, feet.y + 1, feet.z))?
                .unwrap_or(false)
        } else {
            false
        };
        weakness_is_submerged_blocks(feet_water, above_water)
    } else {
        modifiers.is_water_weakened()
    };
    if !submerged {
        return Ok(false);
    }
    let motion = world.velocity();
    let sink = modifiers.effective_sink_speed();
    world.set_velocity(weakness_sinking_velocity(motion, sink))?;
    world.set_swimming(false)?;
    world.set_sprinting(false)?;
    Ok(true)
}
/// Runs vanilla [`travel`], then applies Grand Line post-travel hooks:
/// `AirTime.keepAirborne` (`reset_above_ground_ticks`) and `Weakness.applySinking`.
/// 100% vanilla-equivalent when `modifiers` is default.
pub fn travel_with_modifiers<W: PlayerTravelWorld>(
    world: &mut W,
    input: Vec3,
    modifiers: &EntityBodyModifiers,
) -> Result<(), W::Error> {
    travel(world, input)?;
    if modifiers.reset_above_ground_ticks {
        world.reset_above_ground_ticks()?;
    }
    let _ = apply_water_weakness_sinking(world, modifiers)?;
    Ok(())
}
