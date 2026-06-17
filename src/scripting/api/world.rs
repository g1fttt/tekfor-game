use crate::components::*;
use crate::core::{Direction, WorldGrid};

use hecs::{ComponentRef, Entity};
use mlua::prelude::*;
use serde::Serialize;

use std::ops::Deref;

impl WorldGrid {
  fn get_component<'a, T>(&'a self, lua: &Lua, lua_entity: LuaValue) -> LuaResult<LuaValue>
  where
    T: ComponentRef<'a>,
    <T as ComponentRef<'a>>::Ref: Deref,
    <<T as ComponentRef<'a>>::Ref as Deref>::Target: Serialize,
  {
    let entity = lua.from_value::<Entity>(lua_entity)?;

    match self.get::<T>(entity).ok() {
      Some(comp) => lua.to_value(&*comp),
      None => Ok(LuaNil),
    }
  }
}

impl LuaUserData for WorldGrid {
  fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
    methods.add_method("has_obstacle_at", |lua, this, pos: LuaValue| {
      let pos = lua.from_value::<Position>(pos)?;

      Ok(this.has_component_at::<&Obstacle>(pos.x, pos.y))
    });

    methods.add_method("is_player", |lua, this, entity: LuaValue| {
      let entity = lua.from_value::<Entity>(entity)?;

      Ok(this.satisfies::<&Player>(entity))
    });

    methods.add_method("is_solid", |lua, this, entity: LuaValue| {
      let entity = lua.from_value::<Entity>(entity)?;

      Ok(this.satisfies::<&Solid>(entity))
    });

    methods.add_method_mut(
      "spawn_fireball",
      |lua, this, (pos, facing_dir): (LuaValue, LuaValue)| {
        let pos = lua.from_value::<Position>(pos)?;
        let facing_dir = lua.from_value::<Direction>(facing_dir)?;

        let entity = this.spawn_entity(fireball_template(pos.into_inner(), facing_dir));

        lua.to_value(&entity)
      },
    );

    methods.add_method_mut("spawn_unlocked_door", |lua, this, pos: LuaValue| {
      let pos = lua.from_value::<Position>(pos)?;

      let entity = this.spawn_entity(door_template(pos.into_inner(), false));

      lua.to_value(&entity)
    });

    methods.add_method_mut("switch_bouncing_dir", |lua, this, entity: LuaValue| {
      let entity = lua.from_value::<Entity>(entity)?;

      if let Ok(mut bouncing) = this.get::<&mut Bouncing>(entity) {
        let temp = bouncing.from;

        bouncing.from = bouncing.to;
        bouncing.to = temp;

        return lua.to_value(&*bouncing);
      }
      Ok(LuaNil)
    });

    methods.add_method_mut("add_action", |lua, this, (entity, action): (LuaValue, LuaValue)| {
      let entity = lua.from_value::<Entity>(entity)?;

      if let Ok(queue) = this.query_one_mut::<&mut ActionQueue>(entity) {
        let action_kind = lua.from_value::<ActionKind>(action)?;

        queue.push_back(action_kind);
      }
      Ok(())
    });

    methods.add_method("get_cell", |lua, this, pos: LuaValue| {
      let pos = lua.from_value::<Position>(pos)?;

      let entities: Option<Vec<Entity>> =
        this.get_cell(pos.x, pos.y).map(|it| it.cloned().collect());

      lua.to_value(&entities)
    });

    methods.add_method("get_lock_kind", |lua, this, entity: LuaValue| {
      this.get_component::<&Locked>(lua, entity)
    });

    methods.add_method("get_pos", |lua, this, entity: LuaValue| {
      this.get_component::<&Position>(lua, entity)
    });

    methods.add_method("get_facing_dir", |lua, this, entity: LuaValue| {
      this.get_component::<&Facing>(lua, entity)
    });

    methods.add_method("get_bouncing", |lua, this, entity: LuaValue| {
      this.get_component::<&Bouncing>(lua, entity)
    });

    methods.add_method("get_linked_entities", |lua, this, entity: LuaValue| {
      this.get_component::<&LinkedEntities>(lua, entity)
    })
  }
}
