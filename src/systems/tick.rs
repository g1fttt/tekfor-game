use crate::components::*;
use crate::lock_picking::LockKind;
use crate::states::gameplay::{GameEvent, Gameplay};
use crate::{scripting, utils};

use macroquad::logging as log;

pub fn fireball_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  let Ok((queue, dir)) = state
    .world_grid
    .query_one_mut::<(&mut ActionQueue, &Facing)>(this_entity)
    .map(|(queue, facing)| (queue, facing.into_inner()))
  else {
    return;
  };

  // NOTE: Сущность не удалится если она движется в сторону левой или верхней границы.
  //       Пока не знаю как это починить.
  queue.push_back(ActionKind::Move(MoveOptions {
    dir,
    can_push: false,
    despawn_if_collided: true,
  }));
}

pub fn fireball_thrower_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  let Ok((this_pos, facing_dir)) = state
    .world_grid
    .query_one::<(&Position, &Facing)>(this_entity)
    .get()
    .map(|(pos, facing)| (pos.into_inner(), facing.into_inner()))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, facing_dir);

  if state.world_grid.has_component_at::<&Obstacle>(new_pos.x, new_pos.y) {
    return;
  }

  let fireball = state.world_grid.spawn_fireball_at(new_pos, facing_dir);

  // Не даем самому первому фаерболу застыть на месте без анимации.
  if let Ok(queue) = state.world_grid.query_one_mut::<&mut ActionQueue>(fireball) {
    queue.push_back(ActionKind::Move(MoveOptions::new(facing_dir)));
  }
}

pub fn pressure_plate_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  let Ok(this_pos) = state.world_grid.get::<&Position>(this_entity).map(|pos| pos.into_inner())
  else {
    return;
  };

  let Some(mut this_cell_entities) = state.world_grid.get_cell(this_pos.x, this_pos.y) else {
    return;
  };

  let Ok(linked_entities) =
    state.world_grid.get::<&LinkedEntities>(this_entity).map(|le| le.strong_clone())
  else {
    return;
  };

  let is_anything_standing_on_plate = this_cell_entities
    .any(|&entity| state.world_grid.satisfies::<&Solid>(entity) && entity != this_entity);

  for &entity in linked_entities.iter() {
    let (is_locked, is_lock_basic) = match state.world_grid.get::<&Locked>(entity) {
      Ok(locked) => (true, locked.into_inner() == LockKind::Basic),
      Err(hecs::ComponentError::MissingComponent(_)) => (false, false),
      Err(hecs::ComponentError::NoSuchEntity) => unreachable!(),
    };

    let event = match (is_anything_standing_on_plate, is_locked, is_lock_basic) {
      (true, true, true) => {
        let _ = state.world_grid.remove_one::<Locked>(entity);
        GameEvent::DoorUnlock
      }
      (false, false, true) => {
        let _ = state.world_grid.insert_one(entity, Locked(LockKind::Basic));
        GameEvent::DoorLock
      }
      _ => continue,
    };

    state.game_events.push(event);
  }
}

pub fn door_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  if let Ok(lock_kind) = state.world_grid.get::<&Locked>(this_entity).map(|l| l.into_inner()) {
    match scripting::api::on_lock_pick(&state.lua, lock_kind) {
      Ok(true /* success */) => (),
      Ok(false) => {
        return log::info!("Invalid lock-picking result");
      }
      Err(err) => {
        return log::error!("Error occured during lock-picking process: {}", err);
      }
    }
  }
  state.game_events.push(GameEvent::DoorOpen(this_entity));
}

pub fn saw_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  let Ok((this_pos, bouncing_to)) = state
    .world_grid
    .query_one_mut::<(&Position, &Bouncing)>(this_entity)
    .map(|(pos, b)| (pos.into_inner(), b.to))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, bouncing_to);

  if let Ok(mut bouncing) = state.world_grid.get::<&mut Bouncing>(this_entity)
    && state.world_grid.has_component_at::<&Obstacle>(new_pos.x, new_pos.y)
  {
    let from = bouncing.from;

    bouncing.from = bouncing.to;
    bouncing.to = from;
  }

  if let Ok((queue, dir)) = state
    .world_grid
    .query_one_mut::<(&mut ActionQueue, &Bouncing)>(this_entity)
    .map(|(queue, bouncing)| (queue, bouncing.to))
  {
    queue.push_back(ActionKind::Move(MoveOptions::new(dir)));
  }
}

pub fn downstairs_handler(state: &mut Gameplay, this_entity: hecs::Entity) {
  let Ok(this_pos) = state.world_grid.get::<&Position>(this_entity).map(|pos| pos.into_inner())
  else {
    return;
  };

  let Some(cell_entities) = state.world_grid.get_cell(this_pos.x, this_pos.y) else {
    return;
  };

  for &entity in cell_entities {
    if !state.world_grid.satisfies::<&Player>(entity) {
      continue;
    }

    state.game_events.push(GameEvent::EntityWentDowntairs(entity));
  }
}
