use crate::components::*;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use rmp_serde::decode::Error as DecodeError;
use rmp_serde::encode::Error as EncodeError;

pub fn serialize_world_info(info: &WorldInfo, world: &hecs::World) -> Result<Vec<u8>, EncodeError> {
  let mut full_info = WorldInfoFull::new(info.clone());

  let mut serializer = rmp_serde::Serializer::new(&mut full_info.world_bytes);
  hecs::serialize::column::serialize(world, &mut WorldContextSerialize, &mut serializer)?;

  rmp_serde::to_vec_named(&full_info)
}

pub fn deserialize_world_info(bytes: &[u8]) -> Result<(WorldInfo, hecs::World), DecodeError> {
  let full_info: WorldInfoFull = rmp_serde::from_slice(bytes)?;

  let mut deserializer = rmp_serde::Deserializer::new(full_info.world_bytes.as_slice());
  let world = hecs::serialize::column::deserialize(
    &mut WorldContextDeserialize::default(),
    &mut deserializer,
  )?;

  let info = WorldInfo::from(full_info);

  Ok((info, world))
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
  pub width: u32,
  pub height: u32,
}

impl WorldInfo {
  pub fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }
}

impl From<WorldInfoFull> for WorldInfo {
  fn from(full: WorldInfoFull) -> Self {
    Self::new(full.info.width, full.info.height)
  }
}

#[derive(Serialize, Deserialize)]
struct WorldInfoFull {
  #[serde(flatten)]
  info: WorldInfo,
  world_bytes: Vec<u8>,
}

impl WorldInfoFull {
  fn new(info: WorldInfo) -> Self {
    Self { info, world_bytes: Vec::new() }
  }
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
  Bouncing,
  CausesDeath,
  Mortal,
  Weighted,
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
  ComponentID::Bouncing => Bouncing,
  ComponentID::CausesDeath => CausesDeath,
  ComponentID::Mortal => Mortal,
  ComponentID::Weighted => Weighted,
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
  ComponentID::Bouncing => Bouncing,
  ComponentID::CausesDeath => CausesDeath,
  ComponentID::Mortal => Mortal,
  ComponentID::Weighted => Weighted,
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
