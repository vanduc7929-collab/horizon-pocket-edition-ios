//! LivingEntity water/lava travel and fluid escape/buoyancy velocity updates.
use crate::{
    grandline_hooks::{
        EntityBodyModifiers, FLUID_SWIM_AIR_EFFICIENCY_SCALE, FLUID_SWIM_BASE_SPEED,
        FLUID_SWIM_CLIMBABLE_Y, FLUID_SWIM_DOLPHINS_GRACE_DRAG, FLUID_SWIM_EFFICIENCY_DRAG_TARGET,
        FLUID_SWIM_SHIFT_DIVE_DELTA_Y, FLUID_SWIM_SPRINT_DRAG, FLUID_SWIM_VERTICAL_DRAG,
    },
    living_air::{LivingAirWorld, move_relative},
};
use horizon_core::geometry::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TravelFluidTag {
    Lava,
    EntityFloatable,
}
pub trait LivingFluidWorld: LivingAirWorld {
    fn in_water(&mut self) -> Result<bool, Self::Error>;
    fn sprinting(&mut self) -> Result<bool, Self::Error>;
    fn water_slow_down(&mut self) -> Result<f32, Self::Error>;
    fn water_movement_efficiency(&mut self) -> Result<f64, Self::Error>;
    fn dolphins_grace(&mut self) -> Result<bool, Self::Error>;
    fn fluid_height(&mut self, tag: TravelFluidTag) -> Result<f64, Self::Error>;
    fn fluid_jump_threshold(&mut self) -> Result<f64, Self::Error>;
    fn is_free(&mut self, offset: Vec3) -> Result<bool, Self::Error>;
    fn is_vehicle(&mut self) -> Result<bool, Self::Error>;
    fn can_float_while_ridden(&mut self) -> Result<bool, Self::Error>;
    /// NeoForge `SWIM_SPEED` attribute value (vanilla default `1.0`), used by `FluidSwim.travelAsWater`.
    fn swim_speed(&mut self) -> Result<f64, Self::Error> {
        Ok(1.0)
    }
    /// Whether the entity is holding shift (`Entity.isShiftKeyDown`), used for diving in lava.
    fn shift_key_down(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
    /// Whether the entity is currently in lava (`Entity.isInLava`).
    fn in_lava(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

pub fn fluid_falling_adjusted_movement<W: LivingFluidWorld>(
    world: &mut W,
    gravity: f64,
    falling: bool,
    mut velocity: Vec3,
) -> Result<Vec3, W::Error> {
    if gravity != 0.0 && !world.sprinting()? {
        velocity.y = if falling
            && (velocity.y - 0.005).abs() >= 0.003
            && (velocity.y - gravity / 16.0).abs() < 0.003
        {
            -0.003
        } else {
            velocity.y - gravity / 16.0
        };
    }
    Ok(velocity)
}
pub fn jump_out_of_fluid<W: LivingFluidWorld>(world: &mut W, old_y: f64) -> Result<(), W::Error> {
    let mut velocity = world.velocity();
    if world.horizontal_collision() {
        let probe = Vec3::new(
            velocity.x,
            velocity.y + f64::from(0.6f32) - world.y() + old_y,
            velocity.z,
        );
        if world.is_free(probe)? {
            velocity.y = f64::from(0.3f32);
            world.set_velocity(velocity)?;
        }
    }
    Ok(())
}
pub fn travel_in_water<W: LivingFluidWorld>(
    world: &mut W,
    input: Vec3,
    gravity: f64,
    falling: bool,
    old_y: f64,
) -> Result<(), W::Error> {
    let mut slowdown = if world.sprinting()? {
        0.9f32
    } else {
        world.water_slow_down()?
    };
    let mut speed = 0.02f32;
    let mut efficiency = world.water_movement_efficiency()? as f32;
    if !world.on_ground()? {
        efficiency *= 0.5;
    }
    if efficiency > 0.0 {
        slowdown += (0.54600006f32 - slowdown) * efficiency;
        speed += (world.speed()? - speed) * efficiency;
    }
    if world.dolphins_grace()? {
        slowdown = 0.96;
    }
    move_relative(world, input, speed)?;
    world.move_self(world.velocity())?;
    let mut velocity = world.velocity();
    if world.horizontal_collision() && world.on_climbable()? {
        velocity.y = 0.2;
    }
    velocity = Vec3::new(
        velocity.x * f64::from(slowdown),
        velocity.y * f64::from(0.8f32),
        velocity.z * f64::from(slowdown),
    );
    let velocity = fluid_falling_adjusted_movement(world, gravity, falling, velocity)?;
    world.set_velocity(velocity)?;
    jump_out_of_fluid(world, old_y)
}
pub fn travel_in_lava<W: LivingFluidWorld>(
    world: &mut W,
    input: Vec3,
    gravity: f64,
    falling: bool,
    old_y: f64,
) -> Result<(), W::Error> {
    move_relative(world, input, 0.02)?;
    world.move_self(world.velocity())?;
    let height = world.fluid_height(TravelFluidTag::Lava)?;
    if height <= world.fluid_jump_threshold()? {
        let velocity = world.velocity();
        world.set_velocity(Vec3::new(
            velocity.x * 0.5,
            velocity.y * f64::from(0.8f32),
            velocity.z * 0.5,
        ))?;
        let velocity = fluid_falling_adjusted_movement(world, gravity, falling, world.velocity())?;
        world.set_velocity(velocity)?;
    } else {
        world.set_velocity(world.velocity().scale(0.5))?;
    }
    if gravity != 0.0 {
        let velocity = world.velocity();
        world.set_velocity(Vec3::new(
            velocity.x + 0.0,
            velocity.y + -gravity / 4.0,
            velocity.z + 0.0,
        ))?;
    }
    jump_out_of_fluid(world, old_y)
}
pub fn float_in_liquid_while_ridden<W: LivingFluidWorld>(
    world: &mut W,
    tag: TravelFluidTag,
) -> Result<(), W::Error> {
    if world.is_vehicle()? && world.can_float_while_ridden()? {
        let threshold = world.fluid_jump_threshold()?;
        if world.fluid_height(tag)? > threshold {
            let velocity = world.velocity();
            world.set_velocity(Vec3::new(
                velocity.x + 0.0,
                velocity.y + f64::from(0.04f32),
                velocity.z + 0.0,
            ))?;
        }
    }
    Ok(())
}
pub fn travel_in_fluid<W: LivingFluidWorld>(world: &mut W, input: Vec3) -> Result<(), W::Error> {
    let falling = world.velocity().y <= 0.0;
    let old_y = world.y();
    let gravity = world.effective_gravity()?;
    if world.in_water()? {
        travel_in_water(world, input, gravity, falling, old_y)?;
    } else {
        travel_in_lava(world, input, gravity, falling, old_y)?;
    }
    float_in_liquid_while_ridden(world, TravelFluidTag::EntityFloatable)
}
/// Exact port of `FluidSwim.travelAsWater(LivingEntity, Vec3, float)` invoked by
/// `LivingEntityFluidSwimMixin` when swimming in lava as if it were water.
pub fn travel_in_lava_as_water<W: LivingFluidWorld>(
    world: &mut W,
    input: Vec3,
    gravity: f64,
    falling: bool,
    old_y: f64,
) -> Result<(), W::Error> {
    let mut drag = if world.sprinting()? {
        FLUID_SWIM_SPRINT_DRAG
    } else {
        world.water_slow_down()?
    };
    let mut speed = FLUID_SWIM_BASE_SPEED;
    let mut efficiency = world.water_movement_efficiency()? as f32;
    if !world.on_ground()? {
        efficiency *= FLUID_SWIM_AIR_EFFICIENCY_SCALE;
    }
    if efficiency > 0.0 {
        drag += (FLUID_SWIM_EFFICIENCY_DRAG_TARGET - drag) * efficiency;
        speed += (world.speed()? - speed) * efficiency;
    }
    if world.dolphins_grace()? {
        drag = FLUID_SWIM_DOLPHINS_GRACE_DRAG;
    }
    speed *= world.swim_speed()? as f32;
    if world.shift_key_down()? && !world.on_ground()? {
        let v = world.velocity();
        world.set_velocity(Vec3::new(
            v.x,
            v.y + FLUID_SWIM_SHIFT_DIVE_DELTA_Y,
            v.z,
        ))?;
    }
    move_relative(world, input, speed)?;
    world.move_self(world.velocity())?;
    let mut velocity = world.velocity();
    if world.horizontal_collision() && world.on_climbable()? {
        velocity.y = FLUID_SWIM_CLIMBABLE_Y;
    }
    velocity = Vec3::new(
        velocity.x * f64::from(drag),
        velocity.y * f64::from(FLUID_SWIM_VERTICAL_DRAG),
        velocity.z * f64::from(drag),
    );
    let velocity = fluid_falling_adjusted_movement(world, gravity, falling, velocity)?;
    world.set_velocity(velocity)?;
    jump_out_of_fluid(world, old_y)
}
/// Mirrors `LivingEntityFluidSwimMixin` on top of [`travel_in_fluid`].
/// When `modifiers.swim_in_lava` is `false`, delegates directly to vanilla [`travel_in_fluid`].
pub fn travel_in_fluid_with_modifiers<W: LivingFluidWorld>(
    world: &mut W,
    input: Vec3,
    modifiers: &EntityBodyModifiers,
) -> Result<(), W::Error> {
    if !modifiers.swim_in_lava {
        return travel_in_fluid(world, input);
    }
    let falling = world.velocity().y <= 0.0;
    let old_y = world.y();
    let gravity = world.effective_gravity()?;
    if world.in_water()? {
        travel_in_water(world, input, gravity, falling, old_y)?;
        float_in_liquid_while_ridden(world, TravelFluidTag::EntityFloatable)
    } else {
        travel_in_lava_as_water(world, input, gravity, falling, old_y)
    }
}
