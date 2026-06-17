use crate::components::ActionType;
use crate::core::Direction;
use crate::lock_picking::LockKind;
use crate::states::gameplay::GameEventType;

use macroquad::logging as log;
use mlua::prelude::*;
use serde::Serialize;
use strum::IntoEnumIterator;

use std::fs;
use std::path::Path;

pub fn create() -> LuaResult<Lua> {
  let lua = Lua::new();

  add_enum::<LockKind>(&lua)?;
  add_enum::<Direction>(&lua)?;
  add_enum::<ActionType>(&lua)?;
  add_enum::<GameEventType>(&lua)?;

  preload_module(&lua, "utils", "api_modules/utils.lua")?;

  Ok(lua)
}

pub fn call_func<R: FromLuaMulti>(
  lua: &Lua,
  key: impl IntoLua,
  args: impl IntoLuaMulti,
) -> LuaResult<R> {
  lua.globals().get::<LuaFunction>(key)?.call(args)
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

fn preload_module(lua: &Lua, key: impl IntoLua, path: impl AsRef<Path>) -> LuaResult<()> {
  let package: LuaTable = lua.globals().get("package")?;
  let preload: LuaTable = package.get("preload")?;

  let bytes = match fs::read(path.as_ref()) {
    Ok(b) => b,
    Err(err) => {
      log::error!("Unable to find provided module: {}", err);
      return Ok(());
    }
  };

  let loader = lua.create_function(move |lua, _: ()| lua.load(bytes.clone()).eval::<LuaValue>())?;

  preload.set(key, loader)
}
