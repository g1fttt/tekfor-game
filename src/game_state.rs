use macroquad::logging as log;
use macroquad::prelude::*;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

pub struct State {
  pub grid: Grid,
  pub world: hecs::World,
  asset_manager: AssetManager,
  player_entity: Option<hecs::Entity>,
}

impl State {
  pub async fn with_grid_size(width: u32, height: u32) -> Result<Self, macroquad::Error> {
    let grid = Grid::new(width, height);
    let world = hecs::World::new();

    let asset_manager = AssetManager::load_all().await?;

    Ok(Self { grid, world, asset_manager, player_entity: None })
  }

  pub fn move_entity(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    let max_horizontal_index = self.grid.width() - 1;
    let max_vertical_index = self.grid.height() - 1;

    let new_x = x.clamp(0, max_horizontal_index);
    let new_y = y.clamp(0, max_vertical_index);

    if self.has_anything_solid_at(new_x, new_y) {
      return;
    }

    let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    else {
      return;
    };

    self.grid.remove_from_cell(entity, entity_pos.x as u32, entity_pos.y as u32);
    self.grid.add_to_cell(entity, new_x, new_y);

    entity_pos.x = new_x as f32;
    entity_pos.y = new_y as f32;
  }

  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x as u32, pos.y as u32);
    }
    entity
  }

  pub fn spawn_player_at(&mut self, pos: Vec2) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::Player)),
      Solid,
      Movable,
      OnGrid,
      PlayerTag,
      Position(pos),
    ));
    self.player_entity.replace(entity);
    entity
  }

  pub fn spawn_horizontal_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontal)
  }

  pub fn spawn_horizontal_left_edge_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontalLeftEdge)
  }

  pub fn spawn_crate_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::Crate)),
      OnGrid,
      Solid,
      Movable,
      Pushable,
      Position(pos),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: Vec2,
    linked_entity: Option<hecs::Entity>,
  ) -> hecs::Entity {
    fn handler(state: &mut State, this_entity: hecs::Entity, linked_entity: Option<hecs::Entity>) {
      let Ok(this_pos) = state.world.query_one::<&Position>(this_entity).get().cloned() else {
        return;
      };

      let Some(this_cell_entities) = state.grid.get_cell(this_pos.x as u32, this_pos.y as u32)
      else {
        return;
      };

      let is_anything_standing_on_plate = this_cell_entities
        .iter()
        .filter(|&&ent| state.world.satisfies::<&Solid>(ent))
        .any(|&ent| ent != this_entity);

      if let Ok(mut this) = state.world.get::<&mut Pressable>(this_entity) {
        this.is_pressed = is_anything_standing_on_plate;
      }

      let Some(linked_entity) = linked_entity else {
        return;
      };

      if is_anything_standing_on_plate {
        let _ = state.world.remove::<(Closed, Solid)>(linked_entity);
      } else {
        let _ = state.world.insert(linked_entity, (Closed, Solid));
      }
    }

    self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::PressurePlate)),
      OnGrid,
      Position(pos),
      Pressable { is_pressed: false, interactable: Interactable { linked_entity, handler } },
    ))
  }

  pub fn spawn_door_at(&mut self, pos: Vec2) -> hecs::Entity {
    fn handler(state: &mut State, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
      if let Err(hecs::ComponentError::MissingComponent(_)) =
        state.world.remove::<(Closed, Solid)>(this_entity)
      {
        state.world.insert(this_entity, (Closed, Solid)).unwrap();
      }
    }

    self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(self.asset_manager.get(AssetID::DoorClosed)),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable { linked_entity: None, handler },
    ))
  }

  fn spawn_wall_at(&mut self, pos: Vec2, id: AssetID) -> hecs::Entity {
    self.spawn_entity((Sprite(self.asset_manager.get(id)), OnGrid, Solid, Position(pos)))
  }

  fn has_anything_solid_at(&self, x: u32, y: u32) -> bool {
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

    let (new_x, new_y) = advance_pos_in_direction((x, y), dir);

    if self.has_anything_solid_at(new_x, new_y) {
      return;
    }

    pushable_entities.into_iter().for_each(|ent| self.move_entity(ent, new_x, new_y));
  }
}

impl State {
  pub fn tick(&mut self) {
    self.update_pressure_plates();
    self.update_sprites();
  }

  fn update_pressure_plates(&mut self) {
    let pressable: Vec<(Pressable, hecs::Entity)> = self
      .world
      .query::<(&Pressable, hecs::Entity)>()
      .iter()
      .map(|(p, e)| (p.clone(), e))
      .collect();

    for (plate, entity) in pressable {
      (plate.interactable.handler)(self, entity, plate.interactable.linked_entity)
    }
  }

  fn update_sprites(&mut self) {
    let mut stateful_sprited_objects =
      self.world.query::<(&StatefulObjectKind, &mut Sprite, hecs::Entity)>();

    for (kind, sprite, entity) in stateful_sprited_objects.iter() {
      let asset_id = match (kind, self.world.satisfies::<&Closed>(entity)) {
        (StatefulObjectKind::Door, true) => AssetID::DoorClosed,
        (StatefulObjectKind::Door, false) => AssetID::DoorOpen,
      };

      let new_texture = self.asset_manager.get(asset_id);

      if sprite.0.raw_miniquad_id() != new_texture.raw_miniquad_id() {
        sprite.0 = new_texture;
      }
    }
  }
}

// API
impl State {
  pub fn player_pos(&mut self) -> Option<(u32, u32)> {
    let mut query = self.world.query_one::<&Position>(self.player_entity?);

    query.get().map(|pos| (pos.x as u32, pos.y as u32)).ok()
  }

  pub fn move_player(&mut self, dir: Direction) {
    let Some((player_entity, (pos_x, pos_y))) = self.player_entity.zip(self.player_pos()) else {
      return;
    };

    if !self.world.satisfies::<(&Movable, &OnGrid)>(player_entity) {
      return;
    }

    let (new_player_pos_x, new_player_pos_y) = advance_pos_in_direction((pos_x, pos_y), dir);

    self.push_entities_if_any(new_player_pos_x, new_player_pos_y, dir);
    self.move_entity(player_entity, new_player_pos_x, new_player_pos_y);
  }

  pub fn interact(&mut self, dir: Direction) {
    let Some((pos_x, pos_y)) = self.player_pos() else {
      return;
    };

    let (target_pos_x, target_pos_y) = advance_pos_in_direction((pos_x, pos_y), dir);

    let Some(cell_entities) = self.grid.get_cell(target_pos_x, target_pos_y) else {
      return;
    };

    let interactable_entities: Vec<(Interactable, hecs::Entity)> = cell_entities
      .iter()
      .filter_map(|&ent| {
        let inter = self.world.get::<&Interactable>(ent).ok()?;
        let inter = (*inter).clone();

        Some((inter, ent))
      })
      .collect();

    for (inter, entity) in interactable_entities {
      (inter.handler)(self, entity, inter.linked_entity);
    }
  }
}

fn advance_pos_in_direction((pos_x, pos_y): (u32, u32), dir: Direction) -> (u32, u32) {
  let (dest_x, dest_y) = match dir {
    Direction::North => (None, Some(pos_y.saturating_sub(1))),
    Direction::East => (Some(pos_x + 1), None),
    Direction::South => (None, Some(pos_y + 1)),
    Direction::West => (Some(pos_x.saturating_sub(1)), None),
  };

  let new_pos_x = dest_x.unwrap_or(pos_x);
  let new_pos_y = dest_y.unwrap_or(pos_y);

  (new_pos_x, new_pos_y)
}

#[derive(Serialize, Deserialize, EnumIter, IntoStaticStr, Clone, Copy, Debug, PartialEq)]
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

  pub fn new(width: u32, height: u32) -> Self {
    let capacity = (width * height) as usize;
    let mut cells = Vec::with_capacity(capacity);

    for _ in 0..capacity {
      cells.push(Vec::with_capacity(1));
    }
    Self { cells, width, height }
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  fn index(&self, x: u32, y: u32) -> Option<usize> {
    if x >= self.width || y >= self.height {
      return None;
    }
    Some((y * self.width + x) as usize)
  }

  fn get_cell(&self, x: u32, y: u32) -> Option<&[hecs::Entity]> {
    self.index(x, y).map(|idx| self.cells[idx].as_slice())
  }

  fn add_to_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].push(entity);
    }
  }

  fn remove_from_cell(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].retain(|&e| e != entity);
    }
  }
}

macro_rules! deref {
  ($from:tt, $into:tt) => {
    impl DerefMut for $from {
      fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
      }
    }

    impl Deref for $from {
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

enum StatefulObjectKind {
  Door,
}

#[derive(Clone, Copy)]
pub struct Position(pub Vec2);

impl Position {
  pub fn global(self) -> Vec2 {
    vec2(self.0.x * Grid::CELL_SIZE, self.0.y * Grid::CELL_SIZE)
  }
}

#[derive(Clone, Copy)]
pub struct ZoomFactor(pub f32);

#[derive(Clone)]
pub struct Sprite(pub Texture2D);

#[derive(Clone)]
pub struct Interactable {
  pub linked_entity: Option<hecs::Entity>,
  pub handler: fn(&mut State, hecs::Entity, Option<hecs::Entity>),
}

#[derive(Clone)]
pub struct Pressable {
  is_pressed: bool,
  interactable: Interactable,
}

pub struct Closed;
pub struct Movable;
pub struct Pushable;
pub struct OnGrid;
pub struct Solid;

struct PlayerTag;
pub struct CameraTag;

deref!(Position, Vec2);
deref!(ZoomFactor, f32);
deref!(Sprite, Texture2D);

#[derive(PartialEq, Eq, Hash)]
enum AssetID {
  Player,
  DoorClosed,
  DoorOpen,
  WallHorizontal,
  WallHorizontalLeftEdge,
  PressurePlate,
  Crate,
}

struct AssetManager {
  textures: HashMap<AssetID, Texture2D>,
}

impl AssetManager {
  #[rustfmt::skip]
  async fn load_all() -> Result<Self, macroquad::Error> {
    let mut textures = HashMap::new();

    textures.insert(AssetID::Player, load_texture("assets/textures/player.png").await?);
    textures.insert(AssetID::DoorClosed, load_texture("assets/textures/door-closed.png").await?);
    textures.insert(AssetID::DoorOpen, load_texture("assets/textures/door-open.png").await?);
    textures.insert(AssetID::WallHorizontal, load_texture("assets/textures/wall-horizontal.png").await?);
    textures.insert(AssetID::WallHorizontalLeftEdge, load_texture("assets/textures/wall-horizontal-left-edge.png").await?);
    textures.insert(AssetID::PressurePlate, load_texture("assets/textures/pressure-plate.png").await?);
    textures.insert(AssetID::Crate, load_texture("assets/textures/crate.png").await?);

    textures.values().for_each(|tex| tex.set_filter(FilterMode::Nearest));

    Ok(Self { textures })
  }

  fn get(&self, id: AssetID) -> Texture2D {
    self.textures.get(&id).expect("Unknown asset id").clone()
  }
}
