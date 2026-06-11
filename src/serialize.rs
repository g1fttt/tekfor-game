use crate::components::*;
use crate::resources::SpriteID;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::logging as log;
use macroquad::math::UVec2;

use rmp_serde::decode::Error as DecodeError;
use rmp_serde::encode::Error as EncodeError;

use std::path::Path;
use std::{fs, io};

const WORLD_FORMAT_VERSION: u16 = 1;

pub fn save_world(path: impl AsRef<Path>, info: &WorldInfo, world: &hecs::World) -> io::Result<()> {
  let bytes = serialize_world_info(info, world).unwrap();
  fs::write(path, bytes)
}

pub fn load_world(path: impl AsRef<Path>) -> io::Result<(WorldInfo, hecs::World)> {
  let bytes = fs::read(path)?;
  let (info, mut world) = deserialize_world_info(&bytes).unwrap();

  if info.format_version != WORLD_FORMAT_VERSION {
    todo!("Implement migration logic");
  }

  let mut cmd_buf = hecs::CommandBuffer::new();

  for (sprite_id, pos, entity) in world
    .query::<(&Sprite, &Position, &OnGrid, hecs::Entity)>()
    .into_iter()
    .map(|(s, p, _, e)| (s.into_inner(), p.into_inner(), e))
  {
    let entity_ref = world.entity(entity).unwrap();

    match patch_missing_components(sprite_id, pos, entity_ref) {
      Ok(mut entity_builder) => cmd_buf.insert(entity, entity_builder.build()),
      Err(MissingMandatoryComponent) => {
        let sprite_text: &'static str = sprite_id.into();
        log::error!("Missing mandatory component on {}", sprite_text)
      }
    }
  }

  cmd_buf.run_on(&mut world);

  Ok((info, world))
}

struct MissingMandatoryComponent;

fn patch_missing_components(
  sprite_id: SpriteID,
  pos: UVec2,
  entity_ref: hecs::EntityRef,
) -> Result<hecs::EntityBuilder, MissingMandatoryComponent> {
  let mut entity_builder = hecs::EntityBuilder::new();

  match sprite_id {
    SpriteID::WallHorizontal
    | SpriteID::WallHorizontalLeftEdge
    | SpriteID::WallHorizontalRightEdge
    | SpriteID::WallLeftLowerCorner
    | SpriteID::WallLeftUpperCorner
    | SpriteID::WallRightLowerCorner
    | SpriteID::WallRightUpperCorner
    | SpriteID::WallVertical
    | SpriteID::WallVerticalLeftSplit
    | SpriteID::WallVerticalRightSplit
    | SpriteID::WallHorizontalUpperSplit
    | SpriteID::WallHorizontalLowerSplit
    | SpriteID::WallVerticalTopEdge
    | SpriteID::WallVerticalBottomEdge => entity_builder.add_bundle(wall_template(pos, sprite_id)),
    SpriteID::Crate => entity_builder.add_bundle(crate_template(pos)),
    SpriteID::Player => entity_builder.add_bundle(player_template(pos)),
    SpriteID::DoorUnlocked | SpriteID::DoorLocked => {
      let is_locked = sprite_id == SpriteID::DoorLocked;
      entity_builder.add_bundle(door_template(pos, is_locked))
    }
    SpriteID::DownstairsHorizontalUpper => {
      entity_builder.add_bundle(downstairs_template(pos, sprite_id))
    }
    SpriteID::PressurePlate => entity_builder.add_bundle(pressure_plate_template(pos)),
    SpriteID::Saw => {
      let bouncing = entity_ref.get::<&Bouncing>().ok_or(MissingMandatoryComponent)?;
      entity_builder.add_bundle(saw_template(pos, bouncing.from, bouncing.to))
    }
    SpriteID::Fireball => {
      let facing = entity_ref.get::<&Facing>().ok_or(MissingMandatoryComponent)?;
      entity_builder.add_bundle(fireball_template(pos, facing.into_inner()))
    }
    SpriteID::FireballThrower => {
      let facing = entity_ref.get::<&Facing>().ok_or(MissingMandatoryComponent)?;
      entity_builder.add_bundle(fireball_thrower_template(pos, facing.into_inner()))
    }
  };
  Ok(entity_builder)
}

fn serialize_world_info(info: &WorldInfo, world: &hecs::World) -> Result<Vec<u8>, EncodeError> {
  let mut full_info = WorldInfoFull::new(info.clone());

  let mut serializer = rmp_serde::Serializer::new(&mut full_info.world_bytes);
  hecs::serialize::column::serialize(world, &mut WorldContextSerialize, &mut serializer)?;

  rmp_serde::to_vec_named(&full_info)
}

fn deserialize_world_info(bytes: &[u8]) -> Result<(WorldInfo, hecs::World), DecodeError> {
  let full_info: WorldInfoFull = rmp_serde::from_slice(bytes)?;

  let mut deserializer = rmp_serde::Deserializer::new(full_info.world_bytes.as_slice());
  let world = hecs::serialize::column::deserialize(
    &mut WorldContextDeserialize::default(),
    &mut deserializer,
  )?;

  let info = WorldInfo::from(full_info);

  Ok((info, world))
}

const fn default_format_version() -> u16 {
  WORLD_FORMAT_VERSION
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
  #[serde(default = "default_format_version")]
  format_version: u16,
  pub width: u32,
  pub height: u32,
}

impl WorldInfo {
  pub fn new(width: u32, height: u32) -> Self {
    Self { format_version: WORLD_FORMAT_VERSION, width, height }
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
enum ComponentID {
  Animation,
  ActionQueue,
  StatefulObjectKind,
  Position,
  ZIndex,
  Sprite,
  InteractableHandlerKind,
  Tickable,
  Facing,
  Locked,
  Movable,
  Pushable,
  OnGrid,
  Solid,
  Player,
  Bouncing,
  CausesDeath,
  Mortal,
  Obstacle,
  Downstairs,
  LinkedEntities,
  Intelligent,
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
  ComponentID::InteractableHandlerKind => InteractableHandlerKind,
  ComponentID::Tickable => Tickable,
  ComponentID::Facing => Facing,
  ComponentID::Locked => Locked,
  ComponentID::Movable => Movable,
  ComponentID::Pushable => Pushable,
  ComponentID::OnGrid => OnGrid,
  ComponentID::Solid => Solid,
  ComponentID::Player => Player,
  ComponentID::Bouncing => Bouncing,
  ComponentID::CausesDeath => CausesDeath,
  ComponentID::Mortal => Mortal,
  ComponentID::Obstacle => Obstacle,
  ComponentID::Downstairs => Downstairs,
  ComponentID::LinkedEntities => LinkedEntities,
  ComponentID::Intelligent => Intelligent,
);

impl_deserialize_context!(
  WorldContextDeserialize,
  ComponentID::Animation => Animation,
  ComponentID::ActionQueue => ActionQueue,
  ComponentID::StatefulObjectKind => StatefulObjectKind,
  ComponentID::Position => Position,
  ComponentID::ZIndex => ZIndex,
  ComponentID::Sprite => Sprite,
  ComponentID::InteractableHandlerKind => InteractableHandlerKind,
  ComponentID::Tickable => Tickable,
  ComponentID::Facing => Facing,
  ComponentID::Locked => Locked,
  ComponentID::Movable => Movable,
  ComponentID::Pushable => Pushable,
  ComponentID::OnGrid => OnGrid,
  ComponentID::Solid => Solid,
  ComponentID::Player => Player,
  ComponentID::Bouncing => Bouncing,
  ComponentID::CausesDeath => CausesDeath,
  ComponentID::Mortal => Mortal,
  ComponentID::Obstacle => Obstacle,
  ComponentID::Downstairs => Downstairs,
  ComponentID::LinkedEntities => LinkedEntities,
  ComponentID::Intelligent => Intelligent,
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
