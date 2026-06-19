use crate::components::{ActionKind, CausesDeath, Mortal, MoveOptions, Position};
use crate::core::{Direction, WorldGrid, is_any_animation_active};
use crate::states::gameplay::{GameEvent, GameEventManager};

use hecs::Entity;
use macroquad::input::{KeyCode, get_last_key_pressed};

pub fn mark_dead(world_grid: &WorldGrid, game_events: &mut GameEventManager) {
  for (_, pos, attacker) in world_grid.query::<(&CausesDeath, &Position, Entity)>().into_iter() {
    let Ok(cell_entities) = world_grid.get_cell(pos.into_inner()) else {
      continue;
    };

    for &target in cell_entities {
      if !world_grid.satisfies::<&Mortal>(target) {
        continue;
      }

      game_events.add(GameEvent::EntityDeath { target, attacker })
    }
  }
}

pub fn update_input(world_grid: &mut WorldGrid) -> bool {
  let Some(key_pressed) = get_last_key_pressed() else {
    return false;
  };

  if is_any_animation_active(world_grid) {
    return false;
  }

  let move_dir = match key_pressed {
    KeyCode::W => Some(Direction::North),
    KeyCode::A => Some(Direction::West),
    KeyCode::S => Some(Direction::South),
    KeyCode::D => Some(Direction::East),
    _ => None,
  };

  if let Some(dir) = move_dir {
    world_grid.push_player_action(ActionKind::Move(MoveOptions {
      dir,
      can_push: true,
      despawn_if_collided: false,
    }));
  }
  true
}
