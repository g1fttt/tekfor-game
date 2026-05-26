use crate::game::{ActionKind, Direction, State};

use mlua::MaybeSend;
use mlua::prelude::*;

use serde::Serialize;
use strum::IntoEnumIterator;

const STATE_KEY: &str = "__STATE";

pub fn create() -> LuaResult<Lua> {
  let lua = Lua::new();

  add_func(&lua, "move_player", |lua, state, dir: LuaValue| {
    state.push_player_action(ActionKind::Move(lua.from_value(dir)?));

    Ok(())
  })?;

  add_func(&lua, "interact", |lua, state, dir: LuaValue| {
    state.push_player_action(ActionKind::Interact(lua.from_value(dir)?));

    Ok(())
  })?;

  // add_func(&lua, "wait", |_, _, ()| Ok(()))?;

  add_enum::<Direction>(&lua)?;

  Ok(lua)
}

pub fn run<S>(lua: &Lua, state: &mut State, code: S) -> LuaResult<()>
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
  F: Fn(&Lua, &mut State, A) -> LuaResult<()> + MaybeSend + 'static,
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

  let enum_name = utils::type_name_str::<E>();

  lua.globals().set(enum_name, enum_table)
}

mod utils {
  #[test]
  fn type_name_str_test() {
    struct TestOuter;

    mod test_mod {
      pub struct TestInner;
    }

    assert_eq!(type_name_str::<TestOuter>(), "TestOuter");
    assert_eq!(type_name_str::<test_mod::TestInner>(), "TestInner");
  }

  pub fn type_name_str<T>() -> &'static str {
    std::any::type_name::<T>().split("::").last().unwrap()
  }
}
