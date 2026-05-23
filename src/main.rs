#![allow(static_mut_refs)]

mod game_state;
mod lua_api;

use macroquad::logging as log;
use macroquad::prelude::*;

use egui_macroquad::egui;
use game_state::*;
use mlua::Lua;

use std::sync::LazyLock;

// Набор звуков:         https://ci.itch.io/400-sounds-pack
// Палитра для спрайтов: https://coolors.co/30343f-fafaff-e4d9ff-273469-1e2749

#[macroquad::main(window_conf)]
async fn main() -> Result<(), macroquad::Error> {
  let mut state = State::with_grid_size(4, 4).await?;

  let camera_entity = {
    let centered_camera_pos =
      vec2(state.grid.width() as f32 / 2.0, state.grid.height() as f32 / 2.0);
    state.spawn_entity((Position(centered_camera_pos), ZoomFactor(3.0), CameraTag))
  };

  state.spawn_player_at(vec2(0.0, 0.0));

  let door = state.spawn_door_at(vec2(0.0, 1.0));
  state.spawn_pressure_plate(vec2(2.0, 0.0), Some(door));
  state.spawn_crate_at(vec2(1.0, 0.0));

  state.spawn_horizontal_left_edge_wall_at(vec2(1.0, 1.0));
  state.spawn_horizontal_wall_at(vec2(2.0, 1.0));

  let lua = lua_api::create().unwrap();

  loop {
    clear_background(BLACK);

    state.tick();

    let mut ui_wants_pointer_input = false;

    egui_macroquad::ui(|egui_ctx| {
      ui_wants_pointer_input = egui_ctx.wants_pointer_input();

      egui_ctx.set_pixels_per_point(2.5);

      setup_ui_layout(egui_ctx, &lua, &mut state);
    });

    if !ui_wants_pointer_input {
      update_camera(&mut state.world, camera_entity);
    }

    let camera = construct_camera(&state.world, camera_entity);

    set_camera(&camera);
    {
      draw_sprites(&state.world);
    }
    set_default_camera();

    draw_fps();
    egui_macroquad::draw();

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

fn setup_ui_layout(egui_ctx: &egui::Context, lua: &Lua, state: &mut State) {
  egui::Window::new("Debug window").show(egui_ctx, |ui| unsafe {
    static mut SELECTED: Direction = Direction::North;

    egui::ComboBox::from_label("Move direction").selected_text(format!("{:?}", SELECTED)).show_ui(
      ui,
      |ui| {
        ui.selectable_value(&mut SELECTED, Direction::North, "North");
        ui.selectable_value(&mut SELECTED, Direction::East, "East");
        ui.selectable_value(&mut SELECTED, Direction::South, "South");
        ui.selectable_value(&mut SELECTED, Direction::West, "West");
      },
    );

    if ui.button("Move player").clicked() {
      let selected_str: &'static str = SELECTED.into();

      lua_api::run(lua, state, format!("move_player(Direction.{})", selected_str)).unwrap();
    }

    if ui.button("Interact (South)").clicked() {
      lua_api::run(lua, state, "interact(Direction.South)").unwrap();
    }

    let debug_cfg = DebugConfig::get_mut();
    ui.checkbox(&mut debug_cfg.draw_sprite_outline, "Draw sprite outline");
  });
}

fn update_camera(world: &mut hecs::World, camera_entity: hecs::Entity) {
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

  let camera_pos = world.get::<&Position>(camera_entity).unwrap();
  camera.target = camera_pos.global();

  camera.zoom.x *= zoom_factor;
  camera.zoom.y *= -zoom_factor;

  camera
}

fn draw_sprites(world: &hecs::World) {
  for (pos, sprite) in world.query::<(&Position, &Sprite)>().iter() {
    let global_pos = pos.global();

    draw_texture(sprite, global_pos.x, global_pos.y, WHITE);

    if DebugConfig::get().draw_sprite_outline {
      draw_rectangle_lines(
        global_pos.x,
        global_pos.y,
        Grid::CELL_SIZE,
        Grid::CELL_SIZE,
        2.0,
        Color::from_hex(0x161d36),
      );
    }
  }
}

#[derive(Default)]
struct DebugConfig {
  draw_sprite_outline: bool,
}

impl DebugConfig {
  fn get_mut() -> &'static mut Self {
    static mut CFG: LazyLock<DebugConfig> = LazyLock::new(DebugConfig::default);
    unsafe { &mut CFG }
  }

  fn get() -> &'static Self {
    Self::get_mut()
  }
}
