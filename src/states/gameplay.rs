use crate::components::*;
use crate::core::{Direction, Game, GameState, WorldGrid};
use crate::resources::{AssetManager, MaterialID, Settings, SoundID};
use crate::serialize::WorldInfo;
use crate::states::menu::Menu;
use crate::systems::draw::*;
use crate::{scripting, utils};

use egui_macroquad::egui;
use mlua::Lua;

use macroquad::audio::play_sound_once;
use macroquad::logging as log;
use macroquad::prelude::*;

use std::fs;

pub struct Gameplay {
  pub world_grid: WorldGrid,
  pub game_events: Vec<GameEvent>,
  script_path: Option<String>,
  player_entities: Vec<hecs::Entity>,
  tick_state: TickState,
  is_level_finished: bool,
  hit_intensity: f32,
  abyss: Abyss,
}

impl Gameplay {
  pub fn new(info: WorldInfo, world: hecs::World) -> Self {
    let mut world_grid = WorldGrid::new(&info, world);

    let player_entities = world_grid
      .query_mut::<(&Player, hecs::Entity)>()
      .into_iter()
      .map(|(_, entity)| entity)
      .collect();

    Self {
      world_grid,
      game_events: Vec::new(),
      script_path: None,
      player_entities,
      tick_state: TickState::ProcessingLogic,
      is_level_finished: false,
      hit_intensity: 0.0,
      abyss: Abyss::default(),
    }
  }

  pub fn draw_ui(&mut self, egui_ctx: &egui::Context) -> Option<GameState> {
    if let state = self.draw_gameplay_ui(egui_ctx)
      && state.is_some()
    {
      return state;
    }

    if self.is_level_finished {
      self.draw_level_finished_ui(egui_ctx);
    }
    None
  }

  fn draw_level_finished_ui(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Level finished").resizable(false).show(egui_ctx, |ui| {
      ui.label("Gratulerer!");
    });
  }

  fn draw_gameplay_ui(&mut self, egui_ctx: &egui::Context) -> Option<GameState> {
    egui::Window::new("Gameplay")
      .resizable(false)
      .show(egui_ctx, |ui| {
        if ui.button("Return to main menu").clicked() {
          let menu = Menu::default();

          return Some(GameState::Menu(menu));
        }

        ui.separator();

        let selected_text = format!("{:?}", self.script_path);

        egui::ComboBox::from_label("Script").selected_text(selected_text).show_ui(ui, |ui| {
          utils::with_entries_in("scripts/", |path, filename| {
            ui.selectable_value(&mut self.script_path, Some(path), filename);
          })
        });

        None
      })
      .and_then(|resp| resp.inner)
      .unwrap()
  }

  pub fn draw(&self, state: &Game) {
    let screen_texture = render_target(screen_width() as u32, screen_height() as u32);

    let render_target = state.with_camera(Some(screen_texture), |state| {
      draw_sprites(&self.world_grid, &state.asset_manager);
    });

    if let Some(rt) = render_target {
      self.draw_crt_effect(&rt, &state.asset_manager);
    }
  }

  fn draw_crt_effect(&self, render_target: &Texture2D, asset_manager: &AssetManager) {
    let width = screen_width();
    let height = screen_height();

    let crt = asset_manager.get_material(MaterialID::CRT);
    crt.set_uniform("Resolution", vec2(width, height));
    crt.set_uniform("Intensity", self.hit_intensity);
    crt.set_uniform("CrtIntensity", Settings::get().crt_intensity);

    gl_use_material(crt);

    draw_texture_ex(
      render_target,
      0.0,
      height,
      WHITE,
      DrawTextureParams { dest_size: Some(vec2(width, -height)), ..Default::default() },
    );

    gl_use_default_material();
  }

  pub fn update(&mut self, state: &Game) -> mlua::Result<()> {
    self.update_effects();

    update_sprites(&self.world_grid);

    match self.tick_state {
      TickState::ProcessingLogic => {
        self.do_logical_tick(&state.asset_manager);
        self.update_lua(&state.lua)?;

        self.tick_state = TickState::WaitingForAction;
      }
      TickState::WaitingForAction => {
        if self.update_input() && self.process_actions() {
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
    }
    Ok(())
  }

  pub fn push_player_action(&mut self, action_kind: ActionKind) {
    for mut queue in self
      .player_entities
      .iter()
      .filter_map(|&entity| self.world_grid.get::<&mut ActionQueue>(entity).ok())
    {
      queue.push_back(action_kind.clone());
    }
  }

  fn do_logical_tick(&mut self, asset_manager: &AssetManager) {
    self.update_tickable();
    self.process_events(asset_manager);

    let mut entities_to_despawn = Vec::new();

    self.mark_dead(&mut entities_to_despawn);
    self.mark_went_downstairs(&mut entities_to_despawn);

    for entity in entities_to_despawn.into_iter() {
      self.pre_entity_despawn(entity);

      let _ = self.world_grid.despawn_entity(entity);
    }
  }

  fn update_tickable(&mut self) {
    let tickable: Vec<(InteractableHandlerKind, _)> = self
      .world_grid
      .query::<(&Tickable, hecs::Entity)>()
      .into_iter()
      .map(|(tickable, entity)| (tickable.into_inner(), entity))
      .collect();

    for (handler, entity) in tickable.into_iter() {
      handler.to_fn()(self, entity);
    }
  }

  fn mark_dead(&self, to_despawn: &mut Vec<hecs::Entity>) {
    for (_, &pos) in self.world_grid.query::<(&CausesDeath, &Position)>().into_iter() {
      let Some(cell_entities) = self.world_grid.get_cell(pos.x, pos.y) else {
        continue;
      };

      for &entity in cell_entities {
        if !self.world_grid.satisfies::<&Mortal>(entity) {
          continue;
        }

        to_despawn.push(entity);
      }
    }
  }

  fn mark_went_downstairs(&self, to_despawn: &mut Vec<hecs::Entity>) {
    let mut went_downstairs_query =
      self.world_grid.query::<(&WentDownstairs, &Player, hecs::Entity)>();

    to_despawn.extend(went_downstairs_query.into_iter().map(|(_, _, entity)| entity));
  }

  fn pre_entity_despawn(&mut self, entity: hecs::Entity) {
    if self.world_grid.satisfies::<&WentDownstairs>(entity) {
      self.is_level_finished = true;
    } else if self.world_grid.satisfies::<&Player>(entity) {
      self.hit_intensity = 1.0;

      if let Some(entity_index) =
        self.player_entities.iter().position(|&player_entity| player_entity == entity)
      {
        self.player_entities.remove(entity_index);
      }
    }
  }

  fn update_lua(&mut self, lua: &Lua) -> mlua::Result<()> {
    let Some(ref path) = self.script_path else {
      return Ok(());
    };

    match fs::read(path) {
      Ok(bytes) => {
        lua.load(bytes).exec()?;

        scripting::api::on_abyss_call(lua, &mut self.abyss)?;
      }
      Err(err) => log::error!("Failed to read currently selected script: {}", err),
    }
    Ok(())
  }

  fn update_input(&mut self) -> bool {
    let Some(key_pressed) = get_last_key_pressed() else {
      return false;
    };

    if is_any_animation_active(&self.world_grid) {
      return false;
    }

    let move_dir = match key_pressed {
      KeyCode::W => Some(Direction::North),
      KeyCode::A => Some(Direction::West),
      KeyCode::S => Some(Direction::South),
      KeyCode::D => Some(Direction::East),
      _ => None,
    };

    if let Some(dir) = move_dir {
      self.push_player_action(ActionKind::Move(MoveOptions {
        dir,
        can_push: true,
        despawn_if_collided: false,
      }));
    }
    true
  }

  fn update_effects(&mut self) {
    if self.hit_intensity > 0.001 {
      self.hit_intensity *= 0.05f32.powf(get_frame_time());
    } else {
      self.hit_intensity = 0.0;
    }
  }

  fn process_actions(&mut self) -> bool {
    let mut actions = Vec::new();

    for (queue, entity) in self.world_grid.query::<(&mut ActionQueue, hecs::Entity)>().iter() {
      if let Some(action_kind) = queue.pop_front() {
        actions.push((action_kind, entity));
      }
    }

    if actions.is_empty() {
      return false;
    }

    for (action_kind, entity) in actions {
      match action_kind {
        ActionKind::Move(opts) => self.move_entity(entity, opts),
      }
    }
    true
  }

  fn process_events(&mut self, asset_manager: &AssetManager) {
    while let Some(event) = self.game_events.pop() {
      let sound_id = match event {
        GameEvent::DoorLock => SoundID::Lock,
        GameEvent::DoorUnlock => SoundID::Unlock,
        GameEvent::DoorOpen => SoundID::DoorOpen,
      };

      play_sound_once(asset_manager.get_sound(sound_id));
    }
  }

  fn move_entity(&mut self, entity: hecs::Entity, opts: MoveOptions) {
    if !self.world_grid.satisfies::<(&Movable, &OnGrid)>(entity) {
      return;
    }

    let Ok(pos) = self.world_grid.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
      return;
    };

    let new_pos = utils::advance_pos_in_direction(pos, opts.dir);

    let Some(cell_entities) = self.world_grid.get_cell_owned(new_pos.x, new_pos.y) else {
      return;
    };

    self.interact_with_entities(&cell_entities);

    if opts.can_push {
      self.push_entities(&cell_entities, opts.dir);
    }

    let entity_move_success = self.move_entity_to_pos(entity, new_pos.x, new_pos.y);

    if entity_move_success {
      let start = Position(pos);
      let end = Position(new_pos);

      let move_animation = AnimationKind::Move { start, end };

      let _ = self.world_grid.insert_one(entity, Animation::new(move_animation));
    } else if !entity_move_success && opts.despawn_if_collided {
      let _ = self.world_grid.despawn_entity(entity);
    }
  }

  fn push_entities(&mut self, entities: &[hecs::Entity], dir: Direction) {
    for &entity in entities.iter() {
      if !self.world_grid.satisfies::<(&Movable, &Pushable)>(entity) {
        continue;
      }

      self.move_entity(entity, MoveOptions::new(dir));
    }
  }

  fn interact_with_entities(&mut self, entities: &[hecs::Entity]) {
    for &entity in entities.iter() {
      let Ok(handler) = self.world_grid.get::<&InteractableHandlerKind>(entity).map(|h| h.to_fn())
      else {
        continue;
      };

      handler(self, entity);
    }
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) -> bool {
    if self.world_grid.has_component_at::<&Obstacle>(x, y) {
      return false;
    }

    let Ok(pos) = self
      .world_grid
      .query_one_mut::<(&Position, &Movable, &OnGrid)>(entity)
      .map(|(pos, _, _)| pos.into_inner())
    else {
      return false;
    };

    self.world_grid.remove_from_cell(entity, pos.x, pos.y);
    self.world_grid.add_to_cell(entity, x, y);

    if let Ok(mut pos) = self.world_grid.get::<&mut Position>(entity) {
      pos.x = x;
      pos.y = y;
    }
    true
  }
}

#[derive(Default)]
pub struct Abyss {}

pub enum GameEvent {
  DoorLock,
  DoorUnlock,
  DoorOpen,
}

enum TickState {
  ProcessingLogic,
  WaitingForAction,
  Animating,
}
