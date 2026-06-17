use crate::states::gameplay::{GameEvent, GameEventManager};

use mlua::prelude::*;

impl LuaUserData for GameEventManager {
  fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
    methods.add_method_mut("add", |lua, this, game_event: LuaValue| {
      this.add(lua.from_value::<GameEvent>(game_event)?);

      Ok(())
    });

    methods.add_method("iter", |lua, this, ()| lua.to_value(this.as_slice()))
  }
}
