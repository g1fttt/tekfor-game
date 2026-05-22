use macroquad::logging as log;
use macroquad::prelude::*;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use std::ops::{Deref, DerefMut};

pub struct State {
  pub grid: Grid,
  pub world: hecs::World,
  player_entity: Option<hecs::Entity>,
}

impl State {
  pub fn with_grid_size(width: u32, height: u32) -> Self {
    let grid = Grid::new(width, height);
    let world = hecs::World::new();

    Self { grid, world, player_entity: None }
  }

  pub fn move_entity(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    let max_horizontal_index = self.grid.width() - 1;
    let max_vertical_index = self.grid.height() - 1;

    let x = x.clamp(0, max_horizontal_index);
    let y = y.clamp(0, max_vertical_index);

    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return;
    };

    let has_solid_entities = cell_entities.iter().any(|&ent| self.world.satisfies::<&Solid>(ent));
    if has_solid_entities {
      log::debug!("Found solid entity at x={} y={}. Discarding move attempt.", x, y);

      return;
    }

    if let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    {
      self.grid.remove_from_cell(entity, entity_pos.x as u32, entity_pos.y as u32);

      entity_pos.x = x as f32;
      entity_pos.y = y as f32;

      self.grid.add_to_cell(entity, x, y);
    }
  }

  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x as u32, pos.y as u32);
    }
    entity
  }

  pub fn spawn_player_at(&mut self, pos: Vec2) -> hecs::Entity {
    let texture_bytes = include_bytes!("../assets/textures/player.png");
    let texture = load_texture_from_mem(texture_bytes);

    let entity = self.spawn_entity((Sprite(texture), Movable, OnGrid, PlayerTag, Position(pos)));
    self.player_entity.replace(entity);

    entity
  }

  pub fn spawn_wall_at(&mut self, pos: Vec2, texture_bytes: &[u8]) -> hecs::Entity {
    let texture = load_texture_from_mem(texture_bytes);

    self.spawn_entity((Sprite(texture), OnGrid, Solid, Position(pos)))
  }

  pub fn spawn_horizontal_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, include_bytes!("../assets/textures/wall-horizontal.png"))
  }

  pub fn spawn_horizontal_left_edge_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, include_bytes!("../assets/textures/wall-horizontal-left-edge.png"))
  }

  pub fn spawn_door_at(&mut self, pos: Vec2) -> hecs::Entity {
    let texture_bytes = include_bytes!("../assets/textures/door.png");
    let texture = load_texture_from_mem(texture_bytes);

    self.spawn_entity((
      Sprite(texture),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable {
        linked_entity: None,
        handler: |state: &mut State, this_entity: hecs::Entity, _| {
          if let Err(hecs::ComponentError::MissingComponent(_)) =
            state.world.remove::<(Closed, Solid)>(this_entity)
          {
            state.world.insert(this_entity, (Closed, Solid)).unwrap();
          }
        },
      },
    ))
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

    let (new_pos_x, new_pos_y) = advance_pos_in_direction((pos_x, pos_y), dir);

    self.move_entity(player_entity, new_pos_x, new_pos_y);
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

fn load_texture_from_mem(bytes: &[u8]) -> Texture2D {
  let texture = Texture2D::from_file_with_format(bytes, None);
  texture.set_filter(FilterMode::Nearest);
  texture
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

pub struct Closed;
pub struct Movable;
pub struct OnGrid;
pub struct Solid;

struct PlayerTag;
pub struct CameraTag;

deref!(Position, Vec2);
deref!(ZoomFactor, f32);
deref!(Sprite, Texture2D);
