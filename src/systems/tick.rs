use crate::components::*;
use crate::states::gameplay::MoveOptions;
use crate::{WorldGrid, utils};

pub fn update_tickable(world_grid: &mut WorldGrid) {
  let tickable: Vec<(InteractableHandlerKind, _, _)> = world_grid
    .query::<(&Tickable, hecs::Entity)>()
    .iter()
    .map(|(tickable, entity)| (tickable.handler_kind, entity, tickable.linked_entity))
    .collect();

  for (handler_kind, entity, linked_entity) in tickable {
    handler_kind.to_fn()(world_grid, entity, linked_entity);
  }
}

pub fn fireball_handler(
  world_grid: &mut WorldGrid,
  this_entity: hecs::Entity,
  _: Option<hecs::Entity>,
) {
  let Ok(facing_dir) = world_grid.get::<&Facing>(this_entity).map(|facing| facing.0) else {
    return;
  };

  // NOTE: Сущность не удалится если она движется в сторону левой или верхней границы.
  //       Пока не знаю как это починить.
  if !world_grid.move_entity(this_entity, MoveOptions::new(facing_dir)) {
    let _ = world_grid.despawn_entity(this_entity);
  }
}

pub fn fireball_thrower_handler(
  world_grid: &mut WorldGrid,
  this_entity: hecs::Entity,
  _: Option<hecs::Entity>,
) {
  let Ok((this_pos, facing_dir)) = world_grid
    .query_one::<(&Position, &Facing)>(this_entity)
    .get()
    .map(|(pos, Facing(dir))| (pos.into_inner(), *dir))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, facing_dir);

  if world_grid.has_anything_solid_at(new_pos.x, new_pos.y) {
    return;
  }

  world_grid.spawn_fireball_at(new_pos, facing_dir);
}

pub fn pressure_plate_handler(
  world_grid: &mut WorldGrid,
  this_entity: hecs::Entity,
  linked_entity: Option<hecs::Entity>,
) {
  let Ok(this_pos) = world_grid.get::<&Position>(this_entity).map(|pos| pos.into_inner()) else {
    return;
  };

  let Some(this_cell_entities) = world_grid.get_cell(this_pos.x, this_pos.y) else {
    return;
  };

  let is_anything_standing_on_plate = this_cell_entities.iter().any(|&ent| {
    world_grid.satisfies::<hecs::Without<&Solid, &Animation>>(ent) && ent != this_entity
  });

  let Some(linked_entity) = linked_entity else {
    return;
  };

  if is_anything_standing_on_plate {
    let _ = world_grid.remove::<(Closed, Solid)>(linked_entity);
  } else {
    let _ = world_grid.insert(linked_entity, (Closed, Solid));
  }
}

pub fn door_handler(
  world_grid: &mut WorldGrid,
  this_entity: hecs::Entity,
  _: Option<hecs::Entity>,
) {
  if let Err(hecs::ComponentError::MissingComponent(_)) =
    world_grid.remove::<(Closed, Solid)>(this_entity)
  {
    world_grid.insert(this_entity, (Closed, Solid)).unwrap();
  }
}

pub fn saw_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  let Ok((this_pos, bouncing_to)) = world_grid
    .query_one_mut::<(&Position, &Bouncing)>(this_entity)
    .map(|(pos, b)| (pos.into_inner(), b.to))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, bouncing_to);

  if let Ok(mut bouncing) = world_grid.get::<&mut Bouncing>(this_entity)
    && world_grid.has_anything_solid_at(new_pos.x, new_pos.y)
  {
    let from = bouncing.from;

    bouncing.from = bouncing.to;
    bouncing.to = from;
  }

  if let Ok(dir) = world_grid.get::<&Bouncing>(this_entity).map(|b| b.to) {
    world_grid.move_entity(this_entity, MoveOptions::new(dir));
  }
}
