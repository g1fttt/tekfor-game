use crate::components::*;
use crate::core::WorldGrid;
use crate::resources::ScriptProvider;
use crate::states::gameplay::*;

use hecs::{Entity, World};
use mlua::prelude::*;

pub fn update_entities_lua(
  world: &World,
  lua: &mut LuaContext,
  assets: &impl ScriptProvider,
) -> LuaResult<()> {
  for (script_id, entity) in world
    .query::<(&Script, Entity)>()
    .into_iter()
    .map(|(script, entity)| (script.into_inner(), entity))
  {
    if lua.entities_api.contains_key(&entity) {
      continue;
    }

    let script_bytecode = assets.get_script(script_id);
    let module = lua.load(script_bytecode).eval::<LuaTable>()?;

    let update = module.get::<LuaFunction>("update").ok();
    let interact = module.get::<LuaFunction>("interact").ok();

    lua.entities_api.insert(entity, EntityLuaApi { update, interact });
  }
  Ok(())
}

pub fn call_entity_lua_update(
  world_grid: &mut WorldGrid,
  game_events: &mut GameEventManager,
  lua: &LuaContext,
) -> LuaResult<()> {
  for (&entity, entity_api) in lua.entities_api.iter() {
    let Some(update_fn) = entity_api.update.as_ref() else {
      continue;
    };

    call_entity_lua_fn(world_grid, game_events, entity, lua, update_fn)?;
  }
  Ok(())
}

pub fn call_entity_lua_interact(
  world_grid: &mut WorldGrid,
  game_events: &mut GameEventManager,
  lua: &LuaContext,
  entity: Entity,
) -> LuaResult<()> {
  let Some(interact_fn) = lua.entities_api.get(&entity).and_then(|api| api.interact.as_ref())
  else {
    return Ok(());
  };

  call_entity_lua_fn(world_grid, game_events, entity, lua, interact_fn)
}

fn call_entity_lua_fn(
  world_grid: &mut WorldGrid,
  game_events: &mut GameEventManager,
  entity: Entity,
  lua: &Lua,
  func: &LuaFunction,
) -> LuaResult<()> {
  lua.scope(|scope| {
    let world_grid = scope.create_userdata_ref_mut(world_grid)?;
    let game_events = scope.create_userdata_ref_mut(game_events)?;
    let entity = lua.to_value(&entity)?;

    func.call::<()>((world_grid, game_events, entity))
  })
}
