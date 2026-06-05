use crate::components::*;
use crate::{WorldGrid, utils};

pub fn update_tickable(world_grid: &mut WorldGrid) {
  let tickable: Vec<(InteractableHandlerKind, _)> = world_grid
    .query::<(&Tickable, hecs::Entity)>()
    .into_iter()
    .map(|(tickable, entity)| (tickable.handler(), entity))
    .collect();

  for (handler, entity) in tickable.into_iter() {
    handler.to_fn()(world_grid, entity);
  }
}

pub fn mark_dead(world_grid: &WorldGrid, to_despawn: &mut Vec<hecs::Entity>) {
  for (_, &pos) in world_grid.query::<(&CausesDeath, &Position)>().into_iter() {
    let Some(cell_entities) = world_grid.get_cell(pos.x, pos.y) else {
      continue;
    };

    for &entity in cell_entities {
      if !world_grid.satisfies::<&Mortal>(entity) {
        continue;
      }

      to_despawn.push(entity);
    }
  }
}

pub fn mark_went_downstairs(world_grid: &WorldGrid, to_despawn: &mut Vec<hecs::Entity>) {
  to_despawn.extend(
    world_grid
      .query::<(&WentDownstairs, &Player, hecs::Entity)>()
      .into_iter()
      .map(|(_, _, entity)| entity),
  );
}

pub fn fireball_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let Ok((queue, dir)) = world_grid
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

pub fn fireball_thrower_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let Ok((this_pos, facing_dir)) = world_grid
    .query_one::<(&Position, &Facing)>(this_entity)
    .get()
    .map(|(pos, facing)| (pos.into_inner(), facing.into_inner()))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, facing_dir);
  let has_obstacle_on_the_way =
    world_grid.has_component_at::<&Obstacle>(new_pos.x, new_pos.y).is_some_and(|obstacle| obstacle);

  if has_obstacle_on_the_way {
    return;
  }

  let fireball = world_grid.spawn_fireball_at(new_pos, facing_dir);

  // Не даем самому первому фаерболу застыть на месте без анимации.
  if let Ok(queue) = world_grid.query_one_mut::<&mut ActionQueue>(fireball) {
    queue.push_back(ActionKind::Move(MoveOptions::new(facing_dir)));
  }
}

pub fn pressure_plate_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let Ok(this_pos) = world_grid.get::<&Position>(this_entity).map(|pos| pos.into_inner()) else {
    return;
  };

  let Some(this_cell_entities) = world_grid.get_cell(this_pos.x, this_pos.y) else {
    return;
  };

  let is_anything_standing_on_plate = this_cell_entities
    .iter()
    .any(|&entity| world_grid.satisfies::<&Solid>(entity) && entity != this_entity);

  let Ok(linked_entities) =
    world_grid.get::<&LinkedEntities>(this_entity).map(|le| le.strong_clone())
  else {
    return;
  };

  for &entity in linked_entities.iter() {
    if is_anything_standing_on_plate {
      let _ = world_grid.remove_one::<Locked>(entity);
    } else {
      let _ = world_grid.insert_one(entity, Locked);
    }
  }
}

pub fn door_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let _ = world_grid.despawn_entity(this_entity);
}

pub fn saw_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let Ok((this_pos, bouncing_to)) = world_grid
    .query_one_mut::<(&Position, &Bouncing)>(this_entity)
    .map(|(pos, b)| (pos.into_inner(), b.to))
  else {
    return;
  };

  let new_pos = utils::advance_pos_in_direction(this_pos, bouncing_to);
  let has_obstacle_on_the_way =
    world_grid.has_component_at::<&Obstacle>(new_pos.x, new_pos.y).is_some_and(|obstacle| obstacle);

  if let Ok(mut bouncing) = world_grid.get::<&mut Bouncing>(this_entity)
    && has_obstacle_on_the_way
  {
    let from = bouncing.from;

    bouncing.from = bouncing.to;
    bouncing.to = from;
  }

  if let Ok((queue, dir)) = world_grid
    .query_one_mut::<(&mut ActionQueue, &Bouncing)>(this_entity)
    .map(|(queue, bouncing)| (queue, bouncing.to))
  {
    queue.push_back(ActionKind::Move(MoveOptions::new(dir)));
  }
}

pub fn downstairs_handler(world_grid: &mut WorldGrid, this_entity: hecs::Entity) {
  let Some(cell_entities) = world_grid
    .get::<&Position>(this_entity)
    .ok()
    .and_then(|this_pos| world_grid.get_cell(this_pos.x, this_pos.y))
  else {
    return;
  };

  if let Some(&player_at_downstairs) =
    cell_entities.iter().find(|&&entity| world_grid.satisfies::<&Player>(entity))
  {
    let _ = world_grid.insert_one(player_at_downstairs, WentDownstairs);
  }
}
