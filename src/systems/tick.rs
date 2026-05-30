use crate::components::*;
use crate::states::gameplay::{Gameplay, MoveOptions};

pub fn update_tickable(state: &mut Gameplay) {
  let tickable: Vec<(InteractableHandlerKind, _, _)> = state
    .world
    .query::<(&Tickable, hecs::Entity)>()
    .iter()
    .map(|(tickable, entity)| (tickable.handler_kind, entity, tickable.linked_entity))
    .collect();

  for (handler_kind, entity, linked_entity) in tickable {
    handler_kind.to_fn()(state, entity, linked_entity);
  }
}

pub fn fireball_handler(state: &mut Gameplay, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  let Ok(facing_dir) = state.world.get::<&Facing>(this_entity).map(|facing| facing.0) else {
    return;
  };

  // NOTE: Сущность не удалится если она движется в сторону левой или верхней границы.
  //       Пока не знаю как это починить.
  //
  // FIXME: Нужно сделать вспомогательную функцию despawn,
  //        которая будет удалять сущность в том числе и на сетке.
  if !state.move_entity(this_entity, MoveOptions::new(facing_dir)) {
    let _ = state.world.despawn(this_entity);
  }
}

pub fn fireball_thrower_handler(
  state: &mut Gameplay,
  this_entity: hecs::Entity,
  _: Option<hecs::Entity>,
) {
  let Ok((this_pos, facing_dir)) = state
    .world
    .query_one::<(&Position, &Facing)>(this_entity)
    .get()
    .map(|(pos, Facing(dir))| (pos.into_inner(), *dir))
  else {
    return;
  };

  let new_pos = crate::utils::advance_pos_in_direction(this_pos, facing_dir);

  if state.has_anything_solid_at(new_pos.x, new_pos.y) {
    return;
  }

  state.spawn_fireball_at(new_pos, facing_dir);
}

pub fn pressure_plate_handler(
  state: &mut Gameplay,
  this_entity: hecs::Entity,
  linked_entity: Option<hecs::Entity>,
) {
  let Ok(this_pos) = state.world.get::<&Position>(this_entity).map(|pos| pos.into_inner()) else {
    return;
  };

  let Some(this_cell_entities) = state.grid.get_cell(this_pos.x, this_pos.y) else {
    return;
  };

  let is_anything_standing_on_plate = this_cell_entities.iter().any(|&ent| {
    state.world.satisfies::<hecs::Without<&Solid, &Animation>>(ent) && ent != this_entity
  });

  let Some(linked_entity) = linked_entity else {
    return;
  };

  if is_anything_standing_on_plate {
    let _ = state.world.remove::<(Closed, Solid)>(linked_entity);
  } else {
    let _ = state.world.insert(linked_entity, (Closed, Solid));
  }
}

pub fn door_handler(state: &mut Gameplay, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  if let Err(hecs::ComponentError::MissingComponent(_)) =
    state.world.remove::<(Closed, Solid)>(this_entity)
  {
    state.world.insert(this_entity, (Closed, Solid)).unwrap();
  }
}
