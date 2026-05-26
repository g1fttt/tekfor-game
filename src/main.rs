#![allow(static_mut_refs)]

mod asset;
mod game;
mod lua_api;

use egui_macroquad::egui;
use game::*;
use macroquad::prelude::*;
use mlua::Lua;

use std::fs;

// Набор звуков:         https://ci.itch.io/400-sounds-pack
// Палитра для спрайтов: https://coolors.co/30343f-fafaff-e4d9ff-273469-1e2749

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
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

    state.do_tick();

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
      state.draw_sprites();
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

#[derive(Clone, Copy)]
pub struct ZoomFactor(pub f32);
pub struct CameraTag;

deref_component!(ZoomFactor, f32);
