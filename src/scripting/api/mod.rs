mod game_events;
mod world;

#[cfg(test)]
mod tests {
  use crate::components::Position;
  use crate::core::WorldGrid;
  use crate::scripting::engine::create as create_lua;
  use crate::serialize::WorldInfo;
  use crate::states::gameplay::{GameEvent, GameEventManager};

  use hecs::World;
  use macroquad::math::{UVec2, uvec2};
  use mlua::prelude::*;

  fn with_world_grid<R: FromLuaMulti>(
    code: &str,
  ) -> LuaResult<(Lua, WorldGrid, GameEventManager, R)> {
    let lua = create_lua()?;

    let mut world_grid = WorldGrid::new(&WorldInfo::new(32, 32), World::new());
    let mut game_events = GameEventManager::new();

    let result = lua.scope(|scope| {
      let scoped_world_grid = scope.create_userdata_ref_mut(&mut world_grid)?;
      let scoped_game_events = scope.create_userdata_ref_mut(&mut game_events)?;

      let globals = lua.globals();

      globals.set("world_grid", scoped_world_grid)?;
      globals.set("game_events", scoped_game_events)?;

      let result = lua.load(code).eval()?;

      globals.raw_remove("world_grid")?;
      globals.raw_remove("game_events")?;

      Ok(result)
    })?;

    Ok((lua, world_grid, game_events, result))
  }

  #[test]
  fn spawn_and_get_pos() -> LuaResult<()> {
    const CODE: &str = r#"
      local fireball = world_grid:spawn_fireball({ x = 10, y = 20 }, Direction.East)
      return world_grid:get_pos(fireball)
    "#;

    let (lua, _, _, lua_pos) = with_world_grid::<LuaValue>(CODE)?;
    let pos = lua.from_value::<Position>(lua_pos)?.into_inner();

    assert_eq!(pos, UVec2 { x: 10, y: 20 });

    Ok(())
  }

  #[test]
  fn spawn_and_add_action() -> LuaResult<()> {
    const CODE: &str = r#"
      local fireball = world_grid:spawn_fireball({ x = 0, y = 0 }, Direction.East)

      world_grid:add_action(fireball, {
        type = ActionType.Move,
        data = { dir = Direction.East },
      })
    "#;

    with_world_grid::<()>(CODE)?;

    Ok(())
  }

  #[test]
  fn add_game_event() -> LuaResult<()> {
    const CODE: &str = r#"
      local door = world_grid:spawn_unlocked_door({ x = 1, y = 2 })

      game_events:add({ type = GameEventType.DoorLock, data = door })
      game_events:add({ type = GameEventType.DoorUnlock, data = door })
      game_events:add({ type = GameEventType.DoorOpen, data = door })

      return game_events:iter()
    "#;

    let (lua, world_grid, _, lua_game_events) = with_world_grid(CODE)?;
    let game_events = lua.from_value::<Vec<GameEvent>>(lua_game_events)?;

    let door = *world_grid.get_cell(uvec2(1, 2)).ok().and_then(|mut it| it.next()).unwrap();

    assert_eq!(
      game_events,
      vec![GameEvent::DoorLock(door), GameEvent::DoorUnlock(door), GameEvent::DoorOpen(door)]
    );

    Ok(())
  }

  #[test]
  fn spawn_and_get_linked_entities() -> LuaResult<()> {
    const CODE: &str = r#"
      local door = world_grid:spawn_unlocked_door({ x = 1, y = 2 })

      return world_grid:get_linked_entities(door)
    "#;

    let (_, _, _, linked_entities) = with_world_grid::<LuaValue>(CODE)?;

    assert_eq!(linked_entities, LuaNil);

    Ok(())
  }
}
