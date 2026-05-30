use super::api;
use crate::Direction;
use crate::states::gameplay::Gameplay;

use serde::Serialize;
use strum::IntoEnumIterator;

use mlua::MaybeSend;
use mlua::prelude::*;

const STATE_KEY: &str = "__STATE";

pub fn create() -> LuaResult<Lua> {
  let lua = Lua::new();

  add_func(&lua, "move_player", api::move_player)?;
  add_func(&lua, "interact", api::interact)?;
  add_func(&lua, "wait", api::wait)?;

  add_enum::<Direction>(&lua)?;

  Ok(lua)
}

pub fn run<S>(lua: &Lua, state: &mut Gameplay, code: S) -> LuaResult<()>
where
  S: AsRef<str>,
{
  lua.scope(move |scope| {
    let state = scope.create_any_userdata_ref_mut(state)?;

    lua.set_named_registry_value(STATE_KEY, state)?;
    lua.load(code.as_ref()).exec()?;
    lua.unset_named_registry_value(STATE_KEY)
  })
}

fn add_func<F, A>(lua: &Lua, name: &str, f: F) -> LuaResult<()>
where
  F: Fn(&Lua, &mut Gameplay, A) -> LuaResult<()> + MaybeSend + 'static,
  A: FromLuaMulti,
{
  let func = lua.create_function(move |lua, args| {
    let state_raw = lua.named_registry_value::<LuaAnyUserData>(STATE_KEY)?;
    state_raw.borrow_mut_scoped(|state| f(lua, state, args))
  })?;

  lua.globals().set(name, func)
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
