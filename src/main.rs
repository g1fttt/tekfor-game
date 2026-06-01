use egui_macroquad::egui;
use macroquad::prelude::*;
use mlua::Lua;

use tekfor_game::resources::Settings;
use tekfor_game::scripting;
use tekfor_game::{Game, GameState};

// Набор звуков:         https://ci.itch.io/400-sounds-pack
//                       https://nihil-existentia.itch.io/free-audio-asset-collection
// Палитра для спрайтов: ARQ4

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
  set_pc_assets_folder("assets");
  set_default_filter_mode(FilterMode::Nearest);

  Settings::init_or_load()?;

  let mut current_state = GameState::default();
  let mut state = Game::new().await?;
  let lua = scripting::engine::create()?;

  loop {
    clear_background(BLACK);

    let mut ui_wants_pointer_input = false;
    let mut ui_wants_keyboard_input = false;

    egui_macroquad::ui(|egui_ctx| {
      ui_wants_pointer_input = egui_ctx.wants_pointer_input();
      ui_wants_keyboard_input = egui_ctx.wants_keyboard_input();

      egui_ctx.set_pixels_per_point(screen_dpi_scale() * Settings::get().ui_scale_factor);

      draw_ui(&mut current_state, &lua, egui_ctx);
    });

    if !ui_wants_pointer_input {
      state.update_camera();
    }

    let ui_wants_input = ui_wants_pointer_input || ui_wants_keyboard_input;
    update_and_draw(&mut current_state, &state, ui_wants_input);

    egui_macroquad::draw();

    if Settings::get().show_frames_per_second {
      draw_fps();
    }

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

fn draw_ui(current_state: &mut GameState, lua: &Lua, egui_ctx: &egui::Context) {
  let maybe_new_state = match current_state {
    GameState::Menu(menu) => menu.draw_ui(egui_ctx),
    GameState::Editor(editor) => editor.draw_ui(egui_ctx),
    GameState::Gameplay(gameplay) => gameplay.draw_ui(lua, egui_ctx),
  };

  if let Some(new_state) = maybe_new_state {
    *current_state = new_state;
  }
}

fn update_and_draw(current_state: &mut GameState, state: &Game, ui_wants_input: bool) {
  match current_state {
    GameState::Menu(_) => (),
    GameState::Editor(editor) => {
      editor.update(ui_wants_input);
      editor.draw(state);
    }
    GameState::Gameplay(gameplay) => {
      gameplay.update();
      gameplay.draw(state);
    }
  }
}
