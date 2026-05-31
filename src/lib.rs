pub mod components;
pub mod resources;
pub mod scripting;
pub mod serialize;
pub mod states;
pub mod systems;
pub mod utils;

use components::*;
use resources::{AssetID, AssetManager};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use states::editor::Editor;
use states::gameplay::{Gameplay, MoveOptions};
use states::menu::Menu;

use macroquad::experimental::camera::mouse::Camera;
use macroquad::logging as log;
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

  pub fn with_camera(&self, grid_size: Option<(u32, u32)>, f: impl Fn(&Game)) {
    let mut camera: Camera2D = (&self.camera).into();

    camera.zoom.y *= -1.0;

    // NOTE: Потенциально бесполезный код.
    //
    // Нужно сделать уровни, и уже только потом решать.
    if let Some((grid_width, grid_height)) = grid_size {
      camera.target.x += Grid::CELL_SIZE * (grid_width as f32 / 2.0);
      camera.target.y += Grid::CELL_SIZE * (grid_height as f32 / 2.0);
    }

    set_camera(&camera);
    {
      f(self);
    }
    set_default_camera();
  }

  pub fn update_camera(&mut self, ui_wants_pointer_input: bool) {
    if ui_wants_pointer_input {
      return;
    }

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
    #[allow(clippy::default_constructed_unit_structs)]
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

    log::debug!("Allocated {} bytes for grid", capacity * size_of::<Vec<hecs::Entity>>());

    let mut this = Self { cells, width, height };

    for (pos, entity) in world.query_mut::<(&Position, hecs::Entity)>() {
      this.add_to_cell(entity, pos.x, pos.y);
    }
    this
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
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
  pub fn new(width: u32, height: u32, mut world: hecs::World) -> Self {
    let grid = Grid::new(width, height, &mut world);

    Self { grid, world }
  }

  pub fn with_world(world: hecs::World) -> Self {
    Self::new(32, 32, world)
  }

  pub fn width(&self) -> u32 {
    self.grid.width()
  }

  pub fn height(&self) -> u32 {
    self.grid.height()
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

  pub fn spawn_saw_at(&mut self, pos: UVec2, from: Direction, to: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Saw),
      Bouncing { from, to },
      Solid,
      Movable,
      OnGrid,
      Position(pos),
      Tickable(Interactable { linked_entity: None, handler_kind: InteractableHandlerKind::Saw }),
    ))
  }

  pub fn spawn_player_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Player),
      ZIndex(1),
      Solid,
      Movable,
      OnGrid,
      Player,
      Position(pos),
      ActionQueue::default(),
    ))
  }

  pub fn spawn_wall_at(&mut self, pos: UVec2, id: AssetID) -> hecs::Entity {
    self.spawn_entity((Sprite(id), OnGrid, Solid, Position(pos)))
  }

  pub fn spawn_crate_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((Sprite(AssetID::Crate), OnGrid, Solid, Movable, Pushable, Position(pos)))
  }

  pub fn spawn_fireball_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Fireball),
      Movable,
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::Fireball,
      }),
    ))
  }

  pub fn spawn_fireball_thrower_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::FireballThrower),
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::FireballThrower,
      }),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: UVec2,
    linked_entity: Option<hecs::Entity>,
  ) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::PressurePlate),
      OnGrid,
      Position(pos),
      Tickable(Interactable {
        linked_entity,
        handler_kind: InteractableHandlerKind::PressurePlate,
      }),
    ))
  }

  pub fn spawn_door_at(&mut self, pos: UVec2, is_open: bool) -> hecs::Entity {
    self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(if is_open { AssetID::DoorOpen } else { AssetID::DoorClosed }),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable { linked_entity: None, handler_kind: InteractableHandlerKind::Door },
    ))
  }

  pub fn despawn_entity(&mut self, entity: hecs::Entity) -> Result<(), hecs::NoSuchEntity> {
    if let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) {
      self.grid.remove_from_cell(entity, pos.x, pos.y);
    };
    self.world.despawn(entity)
  }

  pub fn move_entity(&mut self, entity: hecs::Entity, opts: MoveOptions) -> bool {
    if !self.world.satisfies::<(&Movable, &OnGrid)>(entity) {
      return false;
    }

    let Ok(new_pos) = self
      .world
      .get::<&Position>(entity)
      .map(|pos| utils::advance_pos_in_direction(pos.into_inner(), opts.dir))
    else {
      return false;
    };

    if opts.push {
      self.push_entities_if_any(new_pos.x, new_pos.y, opts.dir);
    }

    self.move_entity_to_pos(entity, new_pos.x, new_pos.y)
  }

  pub fn interact(&mut self, entity: hecs::Entity, dir: Direction) {
    let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
      return;
    };

    let target_pos = utils::advance_pos_in_direction(pos, dir);

    let Some(cell_entities) = self.grid.get_cell(target_pos.x, target_pos.y) else {
      return;
    };

    let interactable_entities: Vec<(InteractableHandlerKind, _, _)> = cell_entities
      .iter()
      .filter_map(|&entity| {
        let interactable = self.world.get::<&Interactable>(entity).ok()?;

        Some((interactable.handler_kind, entity, interactable.linked_entity))
      })
      .collect();

    for (handler_kind, entity, linked_entity) in interactable_entities {
      handler_kind.to_fn()(self, entity, linked_entity);
    }
  }

  pub fn has_anything_solid_at(&self, x: u32, y: u32) -> bool {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return false;
    };

    cell_entities.iter().any(|&ent| self.world.satisfies::<&Solid>(ent))
  }

  fn push_entities_if_any(&mut self, x: u32, y: u32, dir: Direction) {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return;
    };

    let pushable_entities: Vec<hecs::Entity> = cell_entities
      .iter()
      .filter(|&&ent| self.world.satisfies::<(&Movable, &Pushable)>(ent))
      .cloned()
      .collect();

    if pushable_entities.is_empty() {
      return;
    }

    pushable_entities.into_iter().for_each(|ent| {
      self.move_entity(ent, MoveOptions::new(dir));
    });
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) -> bool {
    let is_out_of_bounds = self.grid.get_cell(x, y).is_none();

    if is_out_of_bounds || self.has_anything_solid_at(x, y) {
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
    Self::with_world(hecs::World::new())
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
