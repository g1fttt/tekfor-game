pub mod components;
pub mod core;
pub mod lock_picking;
pub mod resources;
pub mod scripting;
pub mod serialize;
pub mod states;
pub mod systems;
pub mod utils;

use core::Game;
use egui_macroquad::egui;
use resources::Settings;
use states::{GameState, PlannedGameState};

use states::editor::Editor;
use states::gameplay::Gameplay;
use states::menu::Menu;

use macroquad::miniquad::conf::{AppleGfxApi, Platform};
use macroquad::miniquad::date::now;
use macroquad::prelude::*;
use macroquad::rand::srand;

// Набор звуков:         https://ci.itch.io/400-sounds-pack
//                       https://nihil-existentia.itch.io/free-audio-asset-collection
// Палитра для спрайтов: ARQ4

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
  set_pc_assets_folder("assets");
  set_default_filter_mode(FilterMode::Nearest);

  srand(now() as u64);

  Settings::init_or_load()?;

  let mut current_state = GameState::default();
  let mut state = Game::new().await?;

  loop {
    state.handle_screen_resize();

    let mut ui_wants_pointer_input = false;
    let mut ui_wants_keyboard_input = false;

    egui_macroquad::ui(|egui_ctx| {
      ui_wants_pointer_input = egui_ctx.wants_pointer_input();
      ui_wants_keyboard_input = egui_ctx.wants_keyboard_input();

      egui_ctx.set_pixels_per_point(screen_dpi_scale() * Settings::get().ui_scale_factor);

      draw_ui(&mut current_state, egui_ctx);
    });

    if !ui_wants_pointer_input {
      state.update_camera();
    }

    let ui_wants_input = ui_wants_pointer_input || ui_wants_keyboard_input;
    update_and_draw(&mut current_state, &mut state, ui_wants_input)?;

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
    platform: Platform { apple_gfx_api: AppleGfxApi::OpenGl, ..Default::default() },
    ..Default::default()
  }
}

fn draw_ui(current_state: &mut GameState, egui_ctx: &egui::Context) {
  match current_state {
    GameState::Menu(menu) => menu.draw_ui(egui_ctx),
    GameState::Editor(editor) => editor.draw_ui(egui_ctx),
    GameState::Gameplay(gameplay) => gameplay.draw_ui(egui_ctx),
  }
}

fn update_and_draw(
  current_state: &mut GameState,
  state: &mut Game,
  ui_wants_input: bool,
) -> mlua::Result<()> {
  clear_background(DARKGRAY);

  let planned_state = match current_state {
    GameState::Menu(menu) => menu.planned(),
    GameState::Editor(editor) => {
      editor.update(ui_wants_input);
      editor.draw(state);
      editor.planned()
    }
    GameState::Gameplay(gameplay) => {
      gameplay.update()?;
      gameplay.draw(state);
      gameplay.planned()
    }
  };

  let Some(planned) = planned_state else {
    return Ok(());
  };

  *current_state = match planned {
    PlannedGameState::Menu => GameState::Menu(Menu::default()),
    PlannedGameState::Editor => {
      GameState::Editor(Box::new(Editor::new(state.asset_manager.strong_clone())))
    }
    PlannedGameState::Gameplay(world_info) => {
      let lua = state.lua.clone();
      let (info, world) = *world_info;

      GameState::Gameplay(Box::new(Gameplay::new(
        lua,
        state.asset_manager.strong_clone(),
        info,
        world,
      )))
    }
  };
  Ok(())
}
