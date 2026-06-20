use crate::core::{CELL_SIZE, Direction};
use crate::lock_picking::LockKind;
use crate::resources::{ScriptID, Settings, SpriteID};

use hecs::{DynamicBundle, Entity};
use macroquad::math::{UVec2, Vec2};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr};

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

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

#[derive(Serialize, Deserialize, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(Serialize, IntoStaticStr, EnumIter))]
#[strum_discriminants(name(ActionType))]
#[serde(tag = "type", content = "data")]
pub enum ActionKind {
  Move(MoveOptions),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoveOptions {
  pub dir: Direction,
  #[serde(default)]
  pub can_push: bool,
  #[serde(default)]
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
#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum StatefulObjectKind {
  Door,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Position(#[serde(with = "crate::serialize::uvec2_serde")] pub UVec2);

impl Position {
  pub fn global(self) -> Vec2 {
    crate::utils::global_pos(self.into_inner())
  }

  pub fn global_centered(self) -> Vec2 {
    self.global() + CELL_SIZE / 2.0
  }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct ZIndex(pub u32);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Sprite(pub SpriteID);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Script(pub ScriptID);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Bouncing {
  pub from: Direction,
  pub to: Direction,
}

// NOTE: Стоит использовать Arc если планируется многопоточность.
//
/// # Safety
/// Тип не является потокобезопасным, ведь использует `Rc` во внутренней реализации,
/// но при этом этот тип реализует такие типажи как: `Sync` и `Send`,
/// чтобы компилятор думал будто это потокобезопасный тип.
#[derive(Serialize, Deserialize)]
pub struct LinkedEntities(Rc<HashSet<Entity>>);

impl LinkedEntities {
  pub fn new(entities: HashSet<Entity>) -> Self {
    Self(Rc::new(entities))
  }

  pub fn get_mut(&mut self) -> Option<&mut HashSet<Entity>> {
    Rc::get_mut(&mut self.0)
  }

  pub fn get(&self) -> &HashSet<Entity> {
    &self.0
  }
}

unsafe impl Sync for LinkedEntities {}
unsafe impl Send for LinkedEntities {}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Facing(pub Direction);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Locked(pub LockKind);

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
pub struct Downstairs;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Intelligent;

deref_component!(Position, UVec2);
deref_component!(ZIndex, u32);
deref_component!(Sprite, SpriteID);
deref_component!(Script, ScriptID);
deref_component!(ActionQueue, VecDeque<ActionKind>);
deref_component!(Facing, Direction);
deref_component!(Locked, LockKind);

pub fn downstairs_template(pos: UVec2, sprite_id: SpriteID) -> impl DynamicBundle {
  (Sprite(sprite_id), Downstairs, OnGrid, Position(pos), Script(ScriptID::Downstairs))
}

pub fn saw_template(pos: UVec2, from: Direction, to: Direction) -> impl DynamicBundle {
  (
    Sprite(SpriteID::Saw),
    Movable,
    OnGrid,
    CausesDeath,
    Position(pos),
    ActionQueue::default(),
    Bouncing { from, to },
    Script(ScriptID::Saw),
  )
}

pub fn player_template(pos: UVec2) -> impl DynamicBundle {
  (
    Sprite(SpriteID::Player),
    ZIndex(1),
    Solid,
    Movable,
    OnGrid,
    Player,
    Mortal,
    Intelligent,
    Position(pos),
    ActionQueue::default(),
  )
}

pub fn wall_template(pos: UVec2, sprite_id: SpriteID) -> impl DynamicBundle {
  (Sprite(sprite_id), OnGrid, Obstacle, Position(pos))
}

pub fn crate_template(pos: UVec2) -> impl DynamicBundle {
  (Sprite(SpriteID::Crate), ZIndex(1), OnGrid, Solid, Obstacle, Movable, Pushable, Position(pos))
}

pub fn fireball_template(pos: UVec2, facing_dir: Direction) -> impl DynamicBundle {
  (
    Sprite(SpriteID::Fireball),
    Movable,
    OnGrid,
    CausesDeath,
    Position(pos),
    ActionQueue::default(),
    Facing(facing_dir),
    Script(ScriptID::Fireball),
  )
}

pub fn fireball_thrower_template(pos: UVec2, facing_dir: Direction) -> impl DynamicBundle {
  (
    Sprite(SpriteID::FireballThrower),
    OnGrid,
    Position(pos),
    Facing(facing_dir),
    Script(ScriptID::FireballThrower),
  )
}

pub fn pressure_plate_template(pos: UVec2) -> impl DynamicBundle {
  (Sprite(SpriteID::PressurePlate), OnGrid, Position(pos), Script(ScriptID::PressurePlate))
}

pub fn door_template(pos: UVec2, is_locked: bool) -> impl DynamicBundle {
  (
    StatefulObjectKind::Door,
    Sprite(if is_locked { SpriteID::DoorLocked } else { SpriteID::DoorUnlocked }),
    OnGrid,
    Obstacle,
    Position(pos),
    Script(ScriptID::Door),
  )
}
