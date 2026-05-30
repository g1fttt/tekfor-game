pub mod components;
pub mod resources;
pub mod scripting;
pub mod serialize;
pub mod states;
pub mod systems;
pub mod utils;

use components::Position;
use resources::AssetManager;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use states::editor::Editor;
use states::gameplay::Gameplay;
use states::menu::Menu;

use macroquad::experimental::camera::mouse::Camera;
use macroquad::logging as log;
use macroquad::prelude::*;

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
