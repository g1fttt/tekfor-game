use crate::core::Direction;
use crate::resources::{Settings, SpriteID};
use crate::states::gameplay::Gameplay;
use crate::systems::tick::*;

use macroquad::math::{UVec2, Vec2};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use std::collections::VecDeque;
use std::sync::Arc;

macro_rules! deref_component {
  ($from:ty, $into:ty) => {
    impl std::ops::DerefMut for $from {
      fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
      }
    }

    impl std::ops::Deref for $from {
      type Target = $into;

      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }

    impl $from {
      #[allow(dead_code)]
      pub fn into_inner(self) -> $into {
        self.0
      }
    }
  };
}

#[derive(Serialize, Deserialize)]
pub enum AnimationKind {
  Move { start: Position, end: Position },
}

#[derive(Serialize, Deserialize)]
pub struct Animation {
  kind: AnimationKind,
  elapsed: f32,
  duration: f32,
}

impl Animation {
  pub fn new(kind: AnimationKind) -> Self {
    Self { kind, elapsed: 0.0, duration: 0.5 }
  }

  pub fn kind(&self) -> &AnimationKind {
    &self.kind
  }

  pub fn progress(&self) -> f32 {
    (self.elapsed / self.duration).clamp(0.0, 1.0)
  }

  pub fn is_finished(&self) -> bool {
    self.elapsed >= self.duration
  }

  /// Возвращает true, если анимация закончила проигрываться.
  pub fn update(&mut self, frame_time: f32) -> bool {
    self.elapsed += Settings::get().animation_speed_multiplier * frame_time;
    self.is_finished()
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ActionKind {
  Move(MoveOptions),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoveOptions {
  pub dir: Direction,
  pub can_push: bool,
  pub despawn_if_collided: bool,
}

impl MoveOptions {
  pub fn new(dir: Direction) -> Self {
    Self { dir, can_push: false, despawn_if_collided: false }
  }
}

#[derive(Serialize, Deserialize, Default)]
pub struct ActionQueue(VecDeque<ActionKind>);

// Перечисление объектов, которые в зависимости от состояния - могут иметь разные спрайты.
#[derive(EnumIter, IntoStaticStr, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum StatefulObjectKind {
  Door,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Position(#[serde(with = "crate::serialize::uvec2_serde")] pub UVec2);

impl Position {
  pub fn global(self) -> Vec2 {
    crate::utils::global_pos(self.into_inner())
  }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct ZIndex(pub u32);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Sprite(pub SpriteID);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Bouncing {
  pub from: Direction,
  pub to: Direction,
}

type InteractableHandler = fn(&mut Gameplay, this_entity: hecs::Entity);

#[derive(EnumIter, IntoStaticStr, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum InteractableHandlerKind {
  Fireball,
  FireballThrower,
  PressurePlate,
  Door,
  Saw,
  Downstairs,
}

impl InteractableHandlerKind {
  pub fn to_fn(self) -> InteractableHandler {
    match self {
      InteractableHandlerKind::Fireball => fireball_handler,
      InteractableHandlerKind::FireballThrower => fireball_thrower_handler,
      InteractableHandlerKind::PressurePlate => pressure_plate_handler,
      InteractableHandlerKind::Door => door_handler,
      InteractableHandlerKind::Saw => saw_handler,
      InteractableHandlerKind::Downstairs => downstairs_handler,
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct LinkedEntities(Arc<Vec<hecs::Entity>>);

impl LinkedEntities {
  pub fn new(entities: Vec<hecs::Entity>) -> Self {
    Self(Arc::new(entities))
  }

  pub fn strong_clone(&self) -> Arc<Vec<hecs::Entity>> {
    Arc::clone(self)
  }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Tickable(pub InteractableHandlerKind);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Facing(pub Direction);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Locked;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Movable;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Pushable;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct OnGrid;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Obstacle;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Solid;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct CausesDeath;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Mortal;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Player;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct WentDownstairs;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Downstairs;

deref_component!(Position, UVec2);
deref_component!(ZIndex, u32);
deref_component!(Sprite, SpriteID);
deref_component!(ActionQueue, VecDeque<ActionKind>);
deref_component!(Tickable, InteractableHandlerKind);
deref_component!(Facing, Direction);
deref_component!(LinkedEntities, Arc<Vec<hecs::Entity>>);
