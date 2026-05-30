use crate::world::*;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

pub fn serialize_as_binary(world: &hecs::World) -> bincode::Result<Vec<u8>> {
  let mut buf = Vec::new();

  let mut serializer = bincode::Serializer::new(&mut buf, bincode::DefaultOptions::new());
  hecs::serialize::column::serialize(world, &mut WorldContextSerialize, &mut serializer)?;

  Ok(buf)
}

pub fn deserialize_from_binary(buf: &[u8]) -> bincode::Result<hecs::World> {
  let mut deserializer = bincode::Deserializer::with_reader(buf, bincode::DefaultOptions::new());
  hecs::serialize::column::deserialize(&mut WorldContextDeserialize::default(), &mut deserializer)
}

#[derive(EnumIter, IntoStaticStr, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ComponentID {
  Animation,
  ActionQueue,
  StatefulObjectKind,
  Position,
  ZIndex,
  Sprite,
  Interactable,
  Tickable,
  Facing,
  Closed,
  Movable,
  Pushable,
  OnGrid,
  Solid,
  Player,
}

macro_rules! impl_serialize_context {
  ($context_name:ident, $($comp_id:expr => $comp_type:ty),* $(,)?) => {
    struct $context_name;

    impl hecs::serialize::column::SerializeContext for $context_name {
      fn component_count(&self, archetype: &hecs::Archetype) -> usize {
        archetype
          .component_types()
          .filter(|&t| $(t == std::any::TypeId::of::<$comp_type>() ||)* false)
          .count()
      }

      fn serialize_component_ids<S: serde::ser::SerializeTuple>(
        &mut self,
        archetype: &hecs::Archetype,
        mut out: S,
      ) -> Result<S::Ok, S::Error> {
        $(
          hecs::serialize::column::try_serialize_id::<$comp_type, _, _>(archetype, &$comp_id, &mut out)?;
        )*
        out.end()
      }

      fn serialize_components<S: serde::ser::SerializeTuple>(
        &mut self,
        archetype: &hecs::Archetype,
        mut out: S,
      ) -> Result<S::Ok, S::Error> {
        $(
          hecs::serialize::column::try_serialize::<$comp_type, _>(archetype, &mut out)?;
        )*
        out.end()
      }
    }
  };
}

macro_rules! impl_deserialize_context {
  ($context_name:ident, $($comp_id:pat => $comp_type:ty),* $(,)?) => {
    #[derive(Default)]
    struct $context_name {
      components: Vec<ComponentID>,
    }

    impl hecs::serialize::column::DeserializeContext for $context_name {
      fn deserialize_component_ids<'de, A>(
        &mut self,
        mut seq: A,
      ) -> Result<hecs::ColumnBatchType, A::Error>
      where
        A: serde::de::SeqAccess<'de>,
      {
        self.components.clear();

        let mut batch = hecs::ColumnBatchType::new();

        while let Some(id) = seq.next_element()? {
          match id {
            $(
              $comp_id => {
               batch.add::<$comp_type>();
              }
            )*
          }
          self.components.push(id);
        }
        Ok(batch)
      }

      fn deserialize_components<'de, A>(
        &mut self,
        entity_count: u32,
        mut seq: A,
        batch: &mut hecs::ColumnBatchBuilder,
      ) -> Result<(), A::Error>
      where
        A: serde::de::SeqAccess<'de>,
      {
        for component in self.components.iter() {
          match *component {
            $(
              $comp_id => {
                hecs::serialize::column::deserialize_column::<$comp_type, _>(entity_count, &mut seq, batch)?;
              }
            )*
          }
        }
        Ok(())
      }
    }
  };
}

impl_serialize_context!(
  WorldContextSerialize,
  ComponentID::Animation => Animation,
  ComponentID::ActionQueue => ActionQueue,
  ComponentID::StatefulObjectKind => StatefulObjectKind,
  ComponentID::Position => Position,
  ComponentID::ZIndex => ZIndex,
  ComponentID::Sprite => Sprite,
  ComponentID::Interactable => Interactable,
  ComponentID::Tickable => Tickable,
  ComponentID::Facing => Facing,
  ComponentID::Closed => Closed,
  ComponentID::Movable => Movable,
  ComponentID::Pushable => Pushable,
  ComponentID::OnGrid => OnGrid,
  ComponentID::Solid => Solid,
  ComponentID::Player => Player,
);

impl_deserialize_context!(
  WorldContextDeserialize,
  ComponentID::Animation => Animation,
  ComponentID::ActionQueue => ActionQueue,
  ComponentID::StatefulObjectKind => StatefulObjectKind,
  ComponentID::Position => Position,
  ComponentID::ZIndex => ZIndex,
  ComponentID::Sprite => Sprite,
  ComponentID::Interactable => Interactable,
  ComponentID::Tickable => Tickable,
  ComponentID::Facing => Facing,
  ComponentID::Closed => Closed,
  ComponentID::Movable => Movable,
  ComponentID::Pushable => Pushable,
  ComponentID::OnGrid => OnGrid,
  ComponentID::Solid => Solid,
  ComponentID::Player => Player,
);

pub(super) mod uvec2_serde {
  use macroquad::math::{UVec2, uvec2};
  use serde::{Deserialize, Deserializer, Serialize, Serializer};

  #[derive(Serialize, Deserialize)]
  struct Shadow {
    x: u32,
    y: u32,
  }

  pub fn serialize<S>(value: &UVec2, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let shadow = Shadow { x: value.x, y: value.y };
    shadow.serialize(serializer)
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<UVec2, D::Error>
  where
    D: Deserializer<'de>,
  {
    let shadow = Shadow::deserialize(deserializer)?;
    Ok(uvec2(shadow.x, shadow.y))
  }
}
