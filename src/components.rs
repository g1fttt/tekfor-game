use crate::resources::{AssetID, Settings};
use crate::systems::tick;
use crate::{Direction, WorldGrid};

use macroquad::math::{UVec2, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use strum::{EnumIter, IntoStaticStr};

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
    self.elapsed += frame_time * Settings::get().animation_speed_multiplier;
    self.is_finished()
  }
}

#[derive(Serialize, Deserialize)]
pub enum ActionKind {
  Move(Direction),
  Interact(Direction),
  NoOp,
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
pub struct Sprite(pub AssetID);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Bouncing {
  pub from: Direction,
  pub to: Direction,
}

type InteractableHandler =
  fn(&mut WorldGrid, this_entity: hecs::Entity, linked_entity: Option<hecs::Entity>);

#[derive(EnumIter, IntoStaticStr, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum InteractableHandlerKind {
  Fireball,
  FireballThrower,
  PressurePlate,
  Door,
  Saw,
}

impl InteractableHandlerKind {
  pub fn to_fn(self) -> InteractableHandler {
    match self {
      InteractableHandlerKind::Fireball => tick::fireball_handler,
      InteractableHandlerKind::FireballThrower => tick::fireball_thrower_handler,
      InteractableHandlerKind::PressurePlate => tick::pressure_plate_handler,
      InteractableHandlerKind::Door => tick::door_handler,
      InteractableHandlerKind::Saw => tick::saw_handler,
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Interactable {
  pub linked_entity: Option<hecs::Entity>,
  pub handler_kind: InteractableHandlerKind,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tickable(pub Interactable);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Facing(pub Direction);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Closed;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Movable;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Pushable;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct OnGrid;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Solid;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Player;

deref_component!(Position, UVec2);
deref_component!(ZIndex, u32);
deref_component!(Sprite, AssetID);
deref_component!(ActionQueue, VecDeque<ActionKind>);
deref_component!(Tickable, Interactable);
deref_component!(Facing, Direction);
