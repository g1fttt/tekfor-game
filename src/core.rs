use crate::components::*;
use crate::resources::AssetManager;
use crate::scripting;
use crate::serialize::WorldInfo;

use hecs::{DynamicBundle, Entity, NoSuchEntity, World};
use mlua::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::audio::AudioContext;
use macroquad::experimental::camera::mouse::Camera;
use macroquad::prelude::*;

use std::ops::{Deref, DerefMut};

use std::collections::HashSet;
use std::collections::hash_set::Iter;

pub const CELL_SIZE: f32 = 32.0;

pub struct Game {
  pub lua: Lua,
  pub asset_manager: AssetManager,
  #[expect(dead_code, reason = "Отсутствие публичных методов")]
  audio_context: AudioContext,
  camera: Camera,
}

impl Game {
  pub async fn new() -> anyhow::Result<Self> {
    let lua = scripting::engine::create()?;

    Ok(Self {
      asset_manager: AssetManager::load_all(&lua).await?,
      lua,
      audio_context: AudioContext::new(),
      camera: Camera::new(Vec2::ZERO, 0.005),
    })
  }

  pub fn with_camera(
    &self,
    render_target: Option<RenderTarget>,
    f: impl Fn(),
  ) -> Option<Texture2D> {
    let mut camera: Camera2D = (&self.camera).into();

    if render_target.is_none() {
      camera.zoom.y *= -1.0;
    }

    camera.render_target = render_target;

    set_camera(&camera);
    {
      f();
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

struct Grid {
  cells: Vec<HashSet<Entity>>,
  width: u32,
  height: u32,
}

impl Grid {
  fn new(width: u32, height: u32, world: &mut World) -> Self {
    let capacity = (width * height) as usize;
    let mut cells = Vec::with_capacity(capacity);

    for _ in 0..capacity {
      cells.push(HashSet::with_capacity(1));
    }

    let mut this = Self { cells, width, height };

    for (pos, entity) in world.query_mut::<(&Position, Entity)>() {
      this.add_to_cell(entity, pos.x, pos.y);
    }
    this
  }

  fn resize(&mut self, new_width: u32, new_height: u32) {
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

  fn width(&self) -> u32 {
    self.width
  }

  fn height(&self) -> u32 {
    self.height
  }

  fn get_cell(&self, x: u32, y: u32) -> Option<Iter<'_, Entity>> {
    self.index(x, y).map(|idx| self.cells[idx].iter())
  }

  fn add_to_cell(&mut self, entity: Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].insert(entity);
    }
  }

  fn remove_from_cell(&mut self, entity: Entity, x: u32, y: u32) {
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
  world: World,
}

impl WorldGrid {
  pub fn new(info: &WorldInfo, mut world: World) -> Self {
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

  pub fn get_cell(&self, x: u32, y: u32) -> Option<Iter<'_, Entity>> {
    self.grid.get_cell(x, y)
  }

  pub fn add_to_cell(&mut self, entity: Entity, x: u32, y: u32) {
    self.grid.add_to_cell(entity, x, y);
  }

  pub fn remove_from_cell(&mut self, entity: Entity, x: u32, y: u32) {
    self.grid.remove_from_cell(entity, x, y);
  }

  pub fn spawn_entity(&mut self, components: impl DynamicBundle) -> Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one_mut::<(&Position, &OnGrid)>(entity) {
      self.grid.add_to_cell(entity, pos.x, pos.y);
    }
    entity
  }

  pub fn despawn_entity(&mut self, entity: Entity) -> Result<(), NoSuchEntity> {
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
    Self::new(&WorldInfo::default(), World::new())
  }
}

impl DerefMut for WorldGrid {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.world
  }
}

impl Deref for WorldGrid {
  type Target = World;

  fn deref(&self) -> &Self::Target {
    &self.world
  }
}
