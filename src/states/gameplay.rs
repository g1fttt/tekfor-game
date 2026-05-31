use crate::components::*;
use crate::states::menu::Menu;
use crate::{Direction, Game, GameState, WorldGrid, scripting, utils};

use crate::systems::draw::*;
use crate::systems::tick::*;

use egui_macroquad::egui;
use mlua::Lua;

use macroquad::logging as log;
use macroquad::prelude::*;

use std::fs;

pub struct Gameplay {
  pub world_grid: WorldGrid,
  script_path: Option<String>,
  player_entity: Option<hecs::Entity>,
  tick_state: TickState,
}

impl Gameplay {
  pub fn with_world(world: hecs::World) -> Self {
    // TODO: Генерировать сетку динамически
    let mut world_grid = WorldGrid::with_world(world);

    let player_entity = world_grid
      .query_mut::<(&Player, hecs::Entity)>()
      .into_iter()
      .map(|(_, entity)| entity)
      .next();

    Self { world_grid, script_path: None, player_entity, tick_state: TickState::ProcessingLogic }
  }

  pub fn draw_ui(&mut self, lua: &Lua, egui_ctx: &egui::Context) -> Option<GameState> {
    let result = egui::Window::new("Debug window")
      .resizable(false)
      .show(egui_ctx, |ui| {
        if ui.button("Return to main menu").clicked() {
          let menu = Menu::default();

          return Ok(Some(GameState::Menu(menu)));
        }

        ui.separator();

        let selected_text = format!("{:?}", self.script_path);

        egui::ComboBox::from_label("Script").selected_text(selected_text).show_ui(ui, |ui| {
          utils::with_entries_in("scripts/", |path, filename| {
            ui.selectable_value(&mut self.script_path, Some(path), filename);
          })
        });

        if let Some(ref script) = self.script_path
          && ui.button("Execute").clicked()
        {
          let script_code = fs::read_to_string(script)?;
          scripting::engine::run(lua, self, script_code)?;
        }
        Ok::<Option<GameState>, anyhow::Error>(None)
      })
      .unwrap();

    result.inner.unwrap().inspect_err(|err| log::error!("{}", err)).unwrap()
  }

  pub fn draw(&self, state: &Game) {
    state.with_camera(None, |state| {
      draw_sprites(&self.world_grid, &state.asset_manager);
    });
  }

  pub fn update(&mut self) {
    update_sprites(&self.world_grid);

    match self.tick_state {
      TickState::WaitingForAction => {
        if self.process_actions() {
          self.tick_state = TickState::Animating;
        }
      }
      TickState::Animating => {
        if is_any_animation_active(&self.world_grid) {
          update_animations(&mut self.world_grid);
        } else {
          self.tick_state = TickState::ProcessingLogic;
        }
      }
      TickState::ProcessingLogic => {
        self.do_logical_tick();
        self.tick_state = TickState::WaitingForAction;
      }
    }
  }

  pub fn push_player_action(&mut self, action_kind: ActionKind) {
    let Some(entity) = self.player_entity else {
      return;
    };

    if let Ok(mut action_queue) = self.world_grid.get::<&mut ActionQueue>(entity) {
      action_queue.push_back(action_kind);
    }
  }

  fn do_logical_tick(&mut self) {
    // Тут можно (и нужно) обновлять логическое состояние мира:
    // * Нажимные плиты
    // * Враги
    // * И т.д.

    update_death_causers(&mut self.world_grid);
    update_tickable(&mut self.world_grid);
  }

  fn process_actions(&mut self) -> bool {
    let mut actions = Vec::new();

    for (queue, entity) in self.world_grid.query::<(&mut ActionQueue, hecs::Entity)>().iter() {
      if self.world_grid.satisfies::<&Animation>(entity) {
        continue;
      }

      if let Some(action_kind) = queue.pop_front() {
        actions.push((action_kind, entity));
      }
    }

    if actions.is_empty() {
      return false;
    }

    for (action_kind, entity) in actions {
      match action_kind {
        ActionKind::Move(dir) => {
          self.world_grid.move_entity(entity, MoveOptions { dir, push: true });
        }
        ActionKind::Interact(dir) => self.world_grid.interact(entity, dir),
        ActionKind::NoOp => (),
      }
    }
    true
  }
}

pub struct MoveOptions {
  pub dir: Direction,
  pub push: bool,
}

impl MoveOptions {
  pub fn new(dir: Direction) -> Self {
    Self { dir, push: false }
  }
}

#[derive(Debug)]
enum TickState {
  WaitingForAction,
  Animating,
  ProcessingLogic,
}
