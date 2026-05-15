use macroquad::logging as log;
use macroquad::prelude::*;

use macroquad::ui::widgets::Window;
use macroquad::ui::{Skin, Ui, hash, root_ui};

use std::ops::{Deref, DerefMut};

// TODO: Сильно разграничить внутренние и внешние координаты.
//       Свести к минимуму каст u32 в f32 и наоборот.

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
  let global_ui_skin = ui_skin(&mut root_ui());

  let mut world = hecs::World::new();
  let mut grid = Grid::new(3, 3);

  let centered_camera_pos = vec2((grid.width / 2) as f32, (grid.height / 2) as f32);
  let camera_entity =
    spawn_entity(&mut world, None, (Position(centered_camera_pos), ZoomFactor(2.0), CameraTag));

  let player_entity = spawn_entity(
    &mut world,
    Some(&mut grid),
    (Position(vec2(0.0, 0.0)), Sprite(Texture2D::empty()), PlayerTag),
  );

  loop {
    clear_background(BLUE);

    // Test
    if let Ok(pos) = world.get::<&Position>(player_entity).map(|p| p.into_inner())
      && is_key_pressed(KeyCode::Space)
    {
      move_entity(&mut world, &mut grid, player_entity, (pos.x + 1.0) as u32, (pos.y + 1.0) as u32);
    }

    draw_ui(&global_ui_skin);

    update_camera(&mut world, camera_entity);
    let camera = construct_camera(&world, camera_entity);

    set_camera(&camera);
    {
      draw_sprites(&world);
    }
    set_default_camera();

    next_frame().await;
  }
}

fn window_conf() -> Conf {
  Conf {
    window_title: String::from("Tekfor game"),
    high_dpi: true,
    fullscreen: true,
    ..Default::default()
  }
}

struct Grid {
  cells: Vec<Vec<hecs::Entity>>,
  width: u32,
  height: u32,
}

impl Grid {
  const CELL_SIZE: f32 = 32.0;

  fn new(width: u32, height: u32) -> Self {
    let capacity = (width * height) as usize;
    let mut cells = Vec::with_capacity(capacity);

    for _ in 0..capacity {
      cells.push(Vec::with_capacity(1));
    }

    Self { cells, width, height }
  }

  fn index(&self, x: u32, y: u32) -> Option<usize> {
    if x >= self.width || y >= self.height {
      return None;
    }
    Some((y * self.width + x) as usize)
  }

  fn add_to_cell(&mut self, x: u32, y: u32, entity: hecs::Entity) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].push(entity);
    }
  }

  fn remove_from_cell(&mut self, x: u32, y: u32, entity: hecs::Entity) {
    if let Some(idx) = self.index(x, y) {
      self.cells[idx].retain(|&e| e != entity);
    }
  }
}

fn move_entity(world: &mut hecs::World, grid: &mut Grid, entity: hecs::Entity, x: u32, y: u32) {
  if let Ok(mut pos) = world.get::<&mut Position>(entity) {
    grid.remove_from_cell(pos.x as u32, pos.y as u32, entity);

    pos.x = x as f32;
    pos.y = y as f32;

    grid.add_to_cell(x, y, entity);
  }
}

fn spawn_entity(
  world: &mut hecs::World,
  grid: Option<&mut Grid>,
  components: impl hecs::DynamicBundle,
) -> hecs::Entity {
  let entity = world.spawn(components);

  if let Ok(pos) = world.get::<&Position>(entity)
    && let Some(g) = grid
  {
    g.add_to_cell(pos.x as u32, pos.y as u32, entity);
  }

  entity
}

fn update_camera(world: &mut hecs::World, camera_entity: hecs::Entity) {
  if is_ui_active() {
    return;
  }

  let Ok((zoom_factor, camera_pos)) =
    world.query_one_mut::<(&mut ZoomFactor, &mut Position)>(camera_entity)
  else {
    return;
  };

  handle_zoom_factor(zoom_factor, 1.0, 5.0);

  if is_mouse_button_down(MouseButton::Left) {
    let mouse_delta = mouse_delta_position();
    let speed = 15.0 / zoom_factor.into_inner();

    camera_pos.x += mouse_delta.x * speed;
    camera_pos.y += mouse_delta.y * speed;
  }
}

fn handle_zoom_factor(zoom_factor: &mut f32, min: f32, max: f32) {
  let mut factor = *zoom_factor;
  let (_, wheel_y) = mouse_wheel();

  if wheel_y.abs() > 0.01 {
    let speed = 1.0 + (wheel_y.abs() * 0.01);

    if wheel_y > 0.0 {
      factor *= speed;
    } else {
      factor /= speed;
    }
  }

  *zoom_factor = factor.clamp(min, max);
}

fn construct_camera(world: &hecs::World, camera_entity: hecs::Entity) -> Camera2D {
  let zoom_factor = world.get::<&ZoomFactor>(camera_entity).map(|zf| zf.into_inner()).unwrap();

  let display_rect = Rect::new(0.0, 0.0, screen_width(), screen_height());
  let mut camera = Camera2D::from_display_rect(display_rect);

  // TODO: Zoom on player
  let camera_pos = world.get::<&Position>(camera_entity).unwrap();
  camera.target = camera_pos.global();

  camera.zoom.x *= zoom_factor;
  camera.zoom.y *= -zoom_factor;

  camera
}

fn is_ui_active() -> bool {
  root_ui().is_mouse_over(mouse_position().into())
}

fn draw_sprites(world: &hecs::World) {
  for (pos, sprite) in world.query::<(&Position, &Sprite)>().without::<&CameraTag>().iter() {
    let global_pos = pos.global();

    draw_texture(sprite, global_pos.x, global_pos.y, WHITE);
    draw_rectangle_lines(global_pos.x, global_pos.y, Grid::CELL_SIZE, Grid::CELL_SIZE, 2.0, BLACK);
  }
}

fn draw_ui(skin: &Skin) {
  root_ui().push_skin(skin);

  draw_fps();

  Window::new(hash!(), vec2(470.0, 50.0), vec2(300.0, 300.0)).ui(&mut root_ui(), |ui| {
    ui.label(None, "Test label");

    if ui.button(None, "Test button") {
      log::info!("Test button was pressed");
    }
  });

  root_ui().pop_skin();
}

fn ui_skin(ui: &mut Ui) -> Skin {
  // const TINY5_REGULAR_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/Tiny5-Regular.ttf");

  // const LABEL_FONT_SIZE: u16 = 48;
  // const BUTTON_FONT_SIZE: u16 = 32;

  // let mut tiny_font = load_ttf_font_from_bytes(TINY5_REGULAR_FONT_BYTES).unwrap();
  // tiny_font.set_filter(FilterMode::Nearest);

  let label_style = ui.style_builder().font_size(48).build();

  let button_style = ui
    .style_builder()
    .font_size(32)
    .color(Color::from_hex(0xDEE2E6))
    .color_hovered(Color::from_hex(0xCED4DA))
    .color_clicked(Color::from_hex(0xADB5BD))
    .margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
    .build();

  Skin { label_style, button_style, ..ui.default_skin() }
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
      fn into_inner(self) -> $into {
        self.0
      }
    }
  };
}

#[derive(Clone, Copy)]
struct Position(Vec2);

impl Position {
  fn global(self) -> Vec2 {
    vec2(self.0.x * Grid::CELL_SIZE, self.0.y * Grid::CELL_SIZE)
  }
}

#[derive(Clone, Copy)]
struct ZoomFactor(f32);

#[derive(Clone)]
struct Sprite(Texture2D);

struct CameraTag;
struct PlayerTag;

deref!(Position, Vec2);
deref!(ZoomFactor, f32);
deref!(Sprite, Texture2D);
