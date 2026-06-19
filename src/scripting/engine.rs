use crate::components::{ActionType, Position};
use crate::core::Direction;
use crate::lock_picking::LockKind;
use crate::states::gameplay::GameEventType;
use crate::utils;

use mlua::prelude::*;
use serde::Serialize;
use strum::IntoEnumIterator;

pub fn create() -> LuaResult<Lua> {
  let lua = Lua::new();

  add_enum::<LockKind>(&lua)?;
  add_enum::<Direction>(&lua)?;
  add_enum::<ActionType>(&lua)?;
  add_enum::<GameEventType>(&lua)?;

  preload_module(&lua, "utils", preload_utils_module)?;

  Ok(lua)
}

fn preload_utils_module(lua: &Lua, module: &LuaTable) -> LuaResult<()> {
  let advance_pos_in_direction = lua.create_function(|lua, (pos, dir): (LuaValue, LuaValue)| {
    let pos = lua.from_value::<Position>(pos)?;
    let dir = lua.from_value::<Direction>(dir)?;

    let new_pos = utils::advance_pos_in_direction(pos.into_inner(), dir);

    lua.to_value(&Position(new_pos))
  })?;

  module.set("advance_pos_in_direction", advance_pos_in_direction)
}

fn add_enum<E>(lua: &Lua) -> LuaResult<()>
where
  E: IntoEnumIterator + Into<&'static str> + Serialize + 'static,
{
  let enum_table = lua.create_table()?;

  for variant in E::iter() {
    let value = lua.to_value(&variant)?;
    let key: &'static str = variant.into();

    enum_table.set(key, value)?;
  }

  let enum_name = crate::utils::type_name_str::<E>();

  lua.globals().set(enum_name, enum_table)
}

fn preload_module(
  lua: &Lua,
  key: impl IntoLua,
  loader: impl Fn(&Lua, &LuaTable) -> LuaResult<()> + 'static,
) -> LuaResult<()> {
  let package: LuaTable = lua.globals().get("package")?;
  let preload: LuaTable = package.get("preload")?;

  let loader = lua.create_function(move |lua, _: ()| {
    let module = lua.create_table()?;

    loader(lua, &module)?;

    Ok(module)
  })?;

  preload.set(key, loader)
}
