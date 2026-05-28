mod asset;
mod lua_api;
mod settings;
mod utils;
mod world;

pub use settings::*;

use egui_macroquad::egui;
use mlua::Lua;
use world::*;

use macroquad::experimental::camera::mouse::Camera;
use macroquad::prelude::*;

use std::fs;

// Набор звуков:         https://ci.itch.io/400-sounds-pack
// Палитра для спрайтов: ARQ4

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
  Settings::init_or_load()?;

  let mut state = State::with_grid_size(4, 4).await?;
  state.spawn_player_at(uvec2(1, 0));

  // let door = state.spawn_door_at(vec2(0.0, 1.0));
  // state.spawn_pressure_plate(vec2(2.0, 0.0), Some(door));
  state.spawn_crate_at(uvec2(1, 1));
  state.spawn_fireball_thrower_at(uvec2(0, 3), Direction::East);

  // state.spawn_horizontal_left_edge_wall_at(vec2(1.0, 1.0));
  // state.spawn_horizontal_wall_at(vec2(2.0, 1.0));
  state.spawn_right_lower_corner_wall_at(uvec2(3, 3));

  let lua = lua_api::create().unwrap();

  loop {
    clear_background(BLACK);

    state.do_tick();

    let mut ui_wants_pointer_input = false;

    egui_macroquad::ui(|egui_ctx| {
      ui_wants_pointer_input = egui_ctx.wants_pointer_input();

      egui_ctx.set_pixels_per_point(2.5);

      setup_debug_window(egui_ctx, &lua, &mut state);
      setup_settings_window(egui_ctx);
    });

    with_camera(&mut state, ui_wants_pointer_input, |state| {
      state.draw_sprites();
    });

    egui_macroquad::draw();

    draw_fps();

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

// TODO: Переместить в state.rs, или в самостоятельный файл.
fn with_camera(state: &mut State, ui_wants_pointer_input: bool, f: impl Fn(&mut State)) {
  update_camera(&mut state.camera, ui_wants_pointer_input);

  let mut camera: Camera2D = (&state.camera).into();

  camera.zoom.y *= -1.0;

  camera.target.x += Grid::CELL_SIZE * (state.grid.width() as f32 / 2.0);
  camera.target.y += Grid::CELL_SIZE * (state.grid.height() as f32 / 2.0);

  set_camera(&camera);
  {
    f(state);
  }
  set_default_camera();
}

fn update_camera(camera: &mut Camera, ui_wants_pointer_input: bool) {
  if ui_wants_pointer_input {
    return;
  }

  let (_, mouse_wheel_y) = mouse_wheel();

  if mouse_wheel_y != 0.0 {
    let base_factor = 1.05;

    let raw_mul_to_scale = match mouse_wheel_y > 0.0 {
      true => camera.scale * base_factor,
      false => camera.scale * (1.0 / base_factor),
    };

    let clamped_mul_to_scale = raw_mul_to_scale.clamp(0.001, 0.01);
    let safe_mul_to_scale = clamped_mul_to_scale / camera.scale;

    camera.scale_mul(Vec2::ZERO, safe_mul_to_scale);
  }

  camera.update(mouse_position_local(), is_mouse_button_down(MouseButton::Left));
}

#[allow(static_mut_refs)]
fn setup_debug_window(egui_ctx: &egui::Context, lua: &Lua, state: &mut State) {
  egui::Window::new("Debug window").resizable(false).show(egui_ctx, |ui| unsafe {
    static mut SCRIPT: Option<String> = None;

    egui::ComboBox::from_label("Script").selected_text(format!("{:?}", SCRIPT)).show_ui(ui, |ui| {
      for entry in fs::read_dir("scripts/").expect("Failed to list scripts") {
        let entry = entry.unwrap();

        let path = entry.path();
        if !path.is_file() {
          continue;
        }

        let selected_value = path.to_str().map(|path| path.to_owned());
        let text = path.file_name().and_then(|filename| filename.to_str()).unwrap();

        ui.selectable_value(&mut SCRIPT, selected_value, text);
      }
    });

    if ui.button("Execute").clicked() {
      let script_code = fs::read_to_string(SCRIPT.as_ref().unwrap()).unwrap();
      lua_api::run(lua, state, script_code).unwrap();
    }
  });
}

fn setup_settings_window(egui_ctx: &egui::Context) {
  egui::Window::new("Settings window").resizable(false).show(egui_ctx, |ui| {
    let mut settings = Settings::get_mut();

    ui.add(
      egui::Slider::new(&mut settings.animation_speed_multiplier, 1.0..=5.0)
        .text("Animation speed multiplier"),
    );

    if ui.button("Save").clicked() {
      let _ = settings.save();
    }
  });
}
