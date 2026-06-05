pub mod components;
pub mod lock_picking;
pub mod resources;
pub mod scripting;
pub mod serialize;
pub mod states;
pub mod systems;
pub mod utils;

use components::*;
use resources::{AssetManager, SpriteID};
use serde::{Deserialize, Serialize};
use serialize::WorldInfo;
use strum::{EnumIter, IntoStaticStr};

use states::editor::Editor;
use states::gameplay::Gameplay;
use states::menu::Menu;

use macroquad::experimental::camera::mouse::Camera;
use macroquad::prelude::*;

use std::ops::{Deref, DerefMut};

pub struct Game {
  pub asset_manager: AssetManager,
  pub camera: Camera,
}

impl Game {
  pub async fn new() -> Result<Self, macroquad::Error> {
    let asset_manager = AssetManager::load_all().await?;
    let camera = Camera::new(Vec2::ZERO, 0.005);

    Ok(Self { asset_manager, camera })
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

pub enum GameState {
  Menu(Menu),
  Editor(Box<Editor>),
  Gameplay(Box<Gameplay>),
}

impl Default for GameState {
  fn default() -> Self {
    Self::Menu(Menu::default())
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
  cells: Vec<Vec<hecs::Entity>>,
  width: u32,
  height: u32,
}

impl Grid {
  pub const CELL_SIZE: f32 = 32.0;

  pub fn new(width: u32, height: u32, world: &mut hecs::World) -> Self {
    let capacity = (width * height) as usize;
    let mut cells = Vec::with_capacity(capacity);

    for _ in 0..capacity {
      cells.push(Vec::with_capacity(1));
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
          self.cells.push(Vec::with_capacity(1));
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

  pub fn get_cell_owned(&self, x: u32, y: u32) -> Option<Vec<hecs::Entity>> {
    self.get_cell(x, y).map(|cell| cell.to_owned())
  }

  pub fn get_cell(&self, x: u32, y: u32) -> Option<&[hecs::Entity]> {
    self.index(x, y).map(|idx| self.cells[idx].as_slice())
  }

  pub fn add_to_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].push(entity);
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

  pub fn get_cell(&self, x: u32, y: u32) -> Option<&[hecs::Entity]> {
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

  pub fn spawn_ground_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((Sprite(SpriteID::Ground), OnGrid, Position(pos)))
  }

  pub fn spawn_downstairs_at(&mut self, pos: UVec2, id: SpriteID) -> hecs::Entity {
    self.spawn_entity((
      Sprite(id),
      Downstairs,
      OnGrid,
      Position(pos),
      Tickable::new(InteractableHandlerKind::Downstairs),
    ))
  }

  pub fn spawn_saw_at(&mut self, pos: UVec2, from: Direction, to: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::Saw),
      Movable,
      OnGrid,
      Obstacle,
      CausesDeath,
      Position(pos),
      ActionQueue::default(),
      Bouncing { from, to },
      Tickable::new(InteractableHandlerKind::Saw),
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
      Tickable::new(InteractableHandlerKind::Fireball),
    ))
  }

  pub fn spawn_fireball_thrower_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(SpriteID::FireballThrower),
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable::new(InteractableHandlerKind::FireballThrower),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: UVec2,
    linked_entities: Option<Vec<hecs::Entity>>,
  ) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(SpriteID::PressurePlate),
      OnGrid,
      Position(pos),
      Tickable::new(InteractableHandlerKind::PressurePlate),
    ));

    if let Some(entities) = linked_entities {
      let _ = self.world.insert_one(entity, LinkedEntities::new(entities));
    }
    entity
  }

  pub fn spawn_door_at(&mut self, pos: UVec2, is_locked: bool) -> hecs::Entity {
    let entity = self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(if is_locked { SpriteID::DoorLocked } else { SpriteID::DoorUnlocked }),
      OnGrid,
      Obstacle,
      Position(pos),
      Interactable(InteractableHandlerKind::Door),
    ));

    if is_locked {
      let _ = self.world.insert_one(entity, Locked);
    }
    entity
  }

  pub fn despawn_entity(&mut self, entity: hecs::Entity) -> Result<(), hecs::NoSuchEntity> {
    if let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) {
      self.grid.remove_from_cell(entity, pos.x, pos.y);
    };
    self.world.despawn(entity)
  }

  pub fn has_component_at<Q: hecs::Query>(&self, x: u32, y: u32) -> Option<bool> {
    let cell_entities = self.grid.get_cell(x, y)?;

    Some(cell_entities.iter().any(|&ent| self.world.satisfies::<Q>(ent)))
  }

  fn push_entities(&mut self, entities: &[hecs::Entity], dir: Direction) {
    for &entity in entities.iter() {
      if !self.world.satisfies::<(&Movable, &Pushable)>(entity) {
        continue;
      }

      self.move_entity(entity, MoveOptions::new(dir));
    }
  }

  fn interact_with_entities(&mut self, entities: &[hecs::Entity]) {
    for &entity in entities.iter() {
      let Ok(handler) = self.world.get::<&InteractableHandlerKind>(entity).map(|h| h.to_fn())
      else {
        continue;
      };

      handler(self, entity);
    }
  }

  fn move_entity(&mut self, entity: hecs::Entity, opts: MoveOptions) {
    if !self.world.satisfies::<(&Movable, &OnGrid)>(entity) {
      return;
    }

    let Ok(new_pos) = self
      .world
      .get::<&Position>(entity)
      .map(|pos| utils::advance_pos_in_direction(pos.into_inner(), opts.dir))
    else {
      return;
    };

    let Some(cell_entities) = self.grid.get_cell_owned(new_pos.x, new_pos.y) else {
      return;
    };

    self.interact_with_entities(&cell_entities);

    if opts.can_push {
      self.push_entities(&cell_entities, opts.dir);
    }

    if !self.move_entity_to_pos(entity, new_pos.x, new_pos.y) && opts.despawn_if_collided {
      let _ = self.despawn_entity(entity);
    }
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) -> bool {
    if self.has_component_at::<&Obstacle>(x, y).is_none_or(|obstacle| obstacle) {
      return false;
    }

    let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    else {
      return false;
    };

    self.grid.remove_from_cell(entity, entity_pos.x, entity_pos.y);
    self.grid.add_to_cell(entity, x, y);

    let start = *entity_pos;
    {
      entity_pos.x = x;
      entity_pos.y = y;
    }
    let end = Position(uvec2(x, y));

    let _ = self.world.insert_one(entity, Animation::new(AnimationKind::Move { start, end }));

    true
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
