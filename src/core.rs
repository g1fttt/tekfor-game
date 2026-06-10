use crate::components::*;
use crate::lock_picking::LockKind;
use crate::resources::{AssetManager, SpriteID};
use crate::scripting;
use crate::serialize::WorldInfo;

use mlua::Lua;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::audio::AudioContext;
use macroquad::experimental::camera::mouse::Camera;
use macroquad::prelude::*;

use std::ops::{Deref, DerefMut};

use std::collections::HashSet;
use std::collections::hash_set::Iter;

pub struct Game {
  pub asset_manager: AssetManager,
  pub lua: Lua,
  #[expect(dead_code, reason = "Отсутствие публичных методов")]
  audio_context: AudioContext,
  camera: Camera,
}

impl Game {
  pub async fn new() -> anyhow::Result<Self> {
    Ok(Self {
      asset_manager: AssetManager::load_all().await?,
      lua: scripting::engine::create()?,
      audio_context: AudioContext::new(),
      camera: Camera::new(Vec2::ZERO, 0.005),
    })
  }

  pub fn with_camera(
    &self,
    render_target: Option<RenderTarget>,
    f: impl Fn(&Game),
  ) -> Option<Texture2D> {
    let mut camera: Camera2D = (&self.camera).into();

    if render_target.is_none() {
      camera.zoom.y *= -1.0;
    }

    camera.render_target = render_target;

    set_camera(&camera);
    {
      f(self);
    }
    set_default_camera();

    camera.render_target.take().map(|rt| rt.texture)
  }

  pub fn update_camera(&mut self) {
    let (_, mouse_wheel_y) = mouse_wheel();

    if mouse_wheel_y != 0.0 {
      let base_factor = 1.05;

      let raw_mul_to_scale = match mouse_wheel_y > 0.0 {
        true => self.camera.scale * base_factor,
        false => self.camera.scale * (1.0 / base_factor),
      };

      let clamped_mul_to_scale = raw_mul_to_scale.clamp(0.001, 0.01);
      let safe_mul_to_scale = clamped_mul_to_scale / self.camera.scale;

      self.camera.scale_mul(Vec2::ZERO, safe_mul_to_scale);
    }

    self.camera.update(mouse_position_local(), is_mouse_button_down(MouseButton::Left));
  }
}

#[derive(Serialize, Deserialize, EnumIter, IntoStaticStr, Clone, Copy, PartialEq)]
pub enum Direction {
  North,
  East,
  South,
  West,
}

pub struct Grid {
  cells: Vec<HashSet<hecs::Entity>>,
  width: u32,
  height: u32,
}

impl Grid {
  pub const CELL_SIZE: f32 = 32.0;

  pub fn new(width: u32, height: u32, world: &mut hecs::World) -> Self {
    let capacity = (width * height) as usize;
    let mut cells = Vec::with_capacity(capacity);

    for _ in 0..capacity {
      cells.push(HashSet::with_capacity(1));
    }

    let mut this = Self { cells, width, height };

    for (pos, entity) in world.query_mut::<(&Position, hecs::Entity)>() {
      this.add_to_cell(entity, pos.x, pos.y);
    }
    this
  }

  pub fn resize(&mut self, new_width: u32, new_height: u32) {
    let new_capacity = (new_width * new_height) as usize;
    let old_capacity = self.cells.capacity();

    if new_capacity == old_capacity {
      return;
    }

    match new_capacity.checked_sub(old_capacity) {
      Some(to_alloc) => {
        self.cells.reserve(to_alloc);

        for _ in 0..to_alloc {
          self.cells.push(HashSet::with_capacity(1));
        }
      }
      None => {
        let to_trunc = old_capacity - new_capacity;

        self.cells.truncate(to_trunc);
        // NOTE: Возможно этот вызов избыточен.
        self.cells.shrink_to_fit();
      }
    }

    self.width = new_width;
    self.height = new_height;
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  pub fn get_cell(&self, x: u32, y: u32) -> Option<Iter<'_, hecs::Entity>> {
    self.index(x, y).map(|idx| self.cells[idx].iter())
  }

  pub fn add_to_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].insert(entity);
    }
  }

  pub fn remove_from_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].retain(|&e| e != entity);
    }
  }

  fn index(&self, x: u32, y: u32) -> Option<usize> {
    if x >= self.width || y >= self.height {
      return None;
    }
    Some((y * self.width + x) as usize)
  }
}

pub struct WorldGrid {
  grid: Grid,
  world: hecs::World,
}

impl WorldGrid {
  pub fn new(info: &WorldInfo, mut world: hecs::World) -> Self {
    let grid = Grid::new(info.width, info.height, &mut world);

    Self { grid, world }
  }

  pub fn width(&self) -> u32 {
    self.grid.width()
  }

  pub fn height(&self) -> u32 {
    self.grid.height()
  }

  pub fn resize(&mut self, new_width: u32, new_height: u32) {
    self.grid.resize(new_width, new_height);
  }

  pub fn get_cell(&self, x: u32, y: u32) -> Option<Iter<'_, hecs::Entity>> {
    self.grid.get_cell(x, y)
  }

  pub fn add_to_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    self.grid.add_to_cell(entity, x, y);
  }

  pub fn remove_from_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    self.grid.remove_from_cell(entity, x, y);
  }

  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x, pos.y);
    }
    entity
  }

  pub fn spawn_ground_at(&mut self, pos: UVec2, id: SpriteID) -> hecs::Entity {
    self.spawn_entity((Sprite(id), OnGrid, Position(pos)))
  }

  pub fn spawn_downstairs_at(&mut self, pos: UVec2, id: SpriteID) -> hecs::Entity {
    self.spawn_entity((
      Sprite(id),
      Downstairs,
      OnGrid,
      Position(pos),
      Tickable(InteractableHandlerKind::Downstairs),
    ))
  }

  pub fn spawn_saw_at(&mut self, pos: UVec2, from: Direction, to: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::Saw),
      Movable,
      OnGrid,
      CausesDeath,
      Position(pos),
      ActionQueue::default(),
      Bouncing { from, to },
      Tickable(InteractableHandlerKind::Saw),
    ))
  }

  pub fn spawn_player_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((
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
    ))
  }

  pub fn spawn_wall_at(&mut self, pos: UVec2, id: SpriteID) -> hecs::Entity {
    self.spawn_entity((Sprite(id), OnGrid, Obstacle, Position(pos)))
  }

  pub fn spawn_crate_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::Crate),
      ZIndex(1),
      OnGrid,
      Solid,
      Obstacle,
      Movable,
      Pushable,
      Position(pos),
    ))
  }

  pub fn spawn_fireball_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::Fireball),
      Movable,
      OnGrid,
      CausesDeath,
      Position(pos),
      ActionQueue::default(),
      Facing(dir),
      Tickable(InteractableHandlerKind::Fireball),
    ))
  }

  pub fn spawn_fireball_thrower_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::FireballThrower),
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(InteractableHandlerKind::FireballThrower),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: UVec2,
    linked_entities: Option<HashSet<hecs::Entity>>,
  ) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(SpriteID::PressurePlate),
      OnGrid,
      Position(pos),
      Tickable(InteractableHandlerKind::PressurePlate),
    ));

    if let Some(entities) = linked_entities {
      let _ = self.world.insert_one(entity, LinkedEntities::new(entities));
    }
    entity
  }

  pub fn spawn_door_at(&mut self, pos: UVec2, lock_kind: Option<LockKind>) -> hecs::Entity {
    let entity = self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(if lock_kind.is_some() { SpriteID::DoorLocked } else { SpriteID::DoorUnlocked }),
      OnGrid,
      Obstacle,
      Position(pos),
      InteractableHandlerKind::Door,
    ));

    if let Some(kind) = lock_kind {
      let _ = self.world.insert_one(entity, Locked(kind));
    }
    entity
  }

  pub fn despawn_entity(&mut self, entity: hecs::Entity) -> Result<(), hecs::NoSuchEntity> {
    if let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) {
      self.grid.remove_from_cell(entity, pos.x, pos.y);
    };

    for linked_entities in self
      .world
      .query_mut::<&mut LinkedEntities>()
      .into_iter()
      .filter_map(|linked_entities| linked_entities.get_mut())
    {
      linked_entities.retain(|&current_entity| current_entity != entity);
    }

    self.world.despawn(entity)
  }

  pub fn has_component_at<Q: hecs::Query>(&self, x: u32, y: u32) -> bool {
    let Some(mut cell_entities) = self.grid.get_cell(x, y) else {
      return false;
    };

    cell_entities.any(|&ent| self.world.satisfies::<Q>(ent))
  }
}

impl Default for WorldGrid {
  fn default() -> Self {
    Self::new(&WorldInfo::default(), hecs::World::new())
  }
}

impl DerefMut for WorldGrid {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.world
  }
}

impl Deref for WorldGrid {
  type Target = hecs::World;

  fn deref(&self) -> &Self::Target {
    &self.world
  }
}
