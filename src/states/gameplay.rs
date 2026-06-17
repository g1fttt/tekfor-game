use crate::components::*;
use crate::core::{Direction, Game, WorldGrid};
use crate::lock_picking::LockKind;
use crate::resources::*;
use crate::serialize::WorldInfo;
use crate::states::PlannedGameState;
use crate::systems::draw::*;
use crate::systems::lua::*;
use crate::utils;

use egui_macroquad::egui;
use hecs::{Entity, World};
use mlua::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr};

use macroquad::audio::play_sound_once;
use macroquad::logging as log;
use macroquad::prelude::*;

use std::collections::HashMap;
use std::ops::Deref;

pub struct Gameplay {
  pub world_grid: WorldGrid,
  pub game_events: GameEventManager,
  lua_ctx: LuaContext,
  asset_manager: AssetManager,
  script_path: Option<String>,
  player_entities: Vec<Entity>,
  tick_state: TickState,
  is_level_finished: bool,
  should_return_to_menu: bool,
  hit_intensity: f32,
}

impl Gameplay {
  pub fn new(lua: Lua, asset_manager: AssetManager, info: WorldInfo, world: World) -> Self {
    let mut world_grid = WorldGrid::new(&info, world);

    let player_entities = world_grid
      .query_mut::<(&Player, Entity)>()
      .into_iter()
      .map(|(_player, entity)| entity)
      .collect();

    Self {
      world_grid,
      game_events: GameEventManager::new(),
      lua_ctx: LuaContext::new(lua),
      asset_manager,
      script_path: None,
      player_entities,
      tick_state: TickState::ProcessingLogic,
      is_level_finished: false,
      should_return_to_menu: false,
      hit_intensity: 0.0,
    }
  }

  pub fn draw_ui(&mut self, egui_ctx: &egui::Context) {
    self.draw_gameplay_ui(egui_ctx);

    if self.is_level_finished {
      self.draw_level_finished_ui(egui_ctx);
    }
  }

  fn draw_level_finished_ui(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Level finished").resizable(false).show(egui_ctx, |ui| {
      ui.label("Gratulerer!");
    });
  }

  fn draw_gameplay_ui(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Gameplay").resizable(false).show(egui_ctx, |ui| {
      if ui.button("Return to main menu").clicked() {
        self.should_return_to_menu = true;
      }

      ui.separator();

      let selected_text = format!("{:?}", self.script_path);

      egui::ComboBox::from_label("Script").selected_text(selected_text).show_ui(ui, |ui| {
        utils::with_entries_in("scripts/", |path, filename| {
          ui.selectable_value(&mut self.script_path, Some(path), filename);
        })
      });
    });
  }

  pub fn draw(&self, state: &Game) {
    let screen_texture = render_target(screen_width() as u32, screen_height() as u32);

    let render_target = state.with_camera(Some(screen_texture), || {
      draw_sprites(&self.world_grid, &self.asset_manager);
    });

    if let Some(rt) = render_target {
      draw_crt_effect(&self.asset_manager, &rt, self.hit_intensity);
    }
  }

  pub fn planned(&self) -> Option<PlannedGameState> {
    if self.should_return_to_menu {
      return Some(PlannedGameState::Menu);
    }

    None
  }

  pub fn update(&mut self) -> LuaResult<()> {
    self.update_effects();

    update_sprites(&self.world_grid);

    match self.tick_state {
      TickState::ProcessingLogic => {
        self.do_logical_tick();
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

  fn update_lua(&mut self) -> LuaResult<()> {
    update_entities_lua(&self.world_grid, &mut self.lua_ctx, &self.asset_manager)?;
    call_entity_lua_update(&mut self.world_grid, &mut self.game_events, &self.lua_ctx)
  }

  fn do_logical_tick(&mut self) {
    if let Err(err) = self.update_lua() {
      log::error!("Error occured during `update_lua` call: {}", err);
    };

    self.mark_dead();
    self.process_events();
  }

  fn mark_dead(&mut self) {
    for (_, &pos, attacker) in
      self.world_grid.query::<(&CausesDeath, &Position, Entity)>().into_iter()
    {
      let Some(cell_entities) = self.world_grid.get_cell(pos.x, pos.y) else {
        continue;
      };

      for &target in cell_entities {
        if !self.world_grid.satisfies::<&Mortal>(target) {
          continue;
        }

        self.game_events.add(GameEvent::EntityDeath { target, attacker })
      }
    }
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

    for (queue, entity) in self.world_grid.query::<(&mut ActionQueue, Entity)>().iter() {
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

  fn process_events(&mut self) {
    while let Some(event) = self.game_events.take_last() {
      let sound_id = match event {
        GameEvent::DoorLock(entity) => {
          let _ = self.world_grid.insert_one(entity, Locked(LockKind::Basic));

          Some(SoundID::Lock)
        }
        GameEvent::DoorUnlock(entity) => {
          let _ = self.world_grid.remove_one::<Locked>(entity);

          Some(SoundID::Unlock)
        }
        GameEvent::DoorOpen(entity) => {
          let _ = self.world_grid.despawn_entity(entity);

          Some(SoundID::DoorOpen)
        }
        GameEvent::EntityWentDowntairs(entity) => {
          let entity_sprite_name = utils::entity_sprite_text_default(&self.world_grid, entity);

          log::info!("{} went downstairs", entity_sprite_name);

          let _ = self.world_grid.despawn_entity(entity);

          self.is_level_finished = true;

          Some(SoundID::LevelFinished)
        }
        GameEvent::EntityInteracted(entity) => {
          let result = call_entity_lua_interact(
            &mut self.world_grid,
            &mut self.game_events,
            &self.lua_ctx,
            entity,
          );

          if let Err(err) = result {
            log::error!("Error occured during `call_entity_lua_interact` call: {}", err);
          }
          None
        }
        GameEvent::EntityDeath { target, attacker } => {
          let target_sprite_name = utils::entity_sprite_text_default(&self.world_grid, target);
          let attacker_sprite_name = utils::entity_sprite_text_default(&self.world_grid, attacker);

          log::info!("{} was killed by {}", target_sprite_name, attacker_sprite_name);

          let _ = self.world_grid.despawn_entity(target);

          self.hit_intensity = 1.0;

          Some(SoundID::Death)
        }
      };

      if let Some(id) = sound_id {
        play_sound_once(self.asset_manager.get_sound(id));
      }
    }
  }

  fn move_entity(&mut self, entity: Entity, opts: MoveOptions) {
    if !self.world_grid.satisfies::<(&Movable, &OnGrid)>(entity) {
      return;
    }

    let Ok(pos) = self.world_grid.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
      return;
    };

    let new_pos = utils::advance_pos_in_direction(pos, opts.dir);

    let Some(cell_entities): Option<Vec<Entity>> =
      self.world_grid.get_cell(new_pos.x, new_pos.y).map(|it| it.cloned().collect())
    else {
      return;
    };

    for cell_entity in cell_entities {
      // FIXME:
      if self.world_grid.satisfies::<&Intelligent>(entity) {
        self.game_events.add(GameEvent::EntityInteracted(cell_entity))
      }

      if opts.can_push && self.world_grid.satisfies::<&Pushable>(cell_entity) {
        self.move_entity(entity, MoveOptions::new(opts.dir));
      }
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

  fn move_entity_to_pos(&mut self, entity: Entity, x: u32, y: u32) -> bool {
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

enum TickState {
  ProcessingLogic,
  WaitingForAction,
  Animating,
}

fn draw_crt_effect(assets: &impl MaterialProvider, render_target: &Texture2D, intensity: f32) {
  let width = screen_width();
  let height = screen_height();

  let crt = assets.get_material(MaterialID::CRT);
  crt.set_uniform("Resolution", vec2(width, height));
  crt.set_uniform("Intensity", intensity);
  crt.set_uniform("CrtIntensity", Settings::get().crt_intensity);

  gl_use_material(crt);
  {
    draw_texture_ex(
      render_target,
      0.0,
      height,
      WHITE,
      DrawTextureParams { dest_size: Some(vec2(width, -height)), ..Default::default() },
    );
  }
  gl_use_default_material();
}

pub struct EntityLuaApi {
  pub update: Option<LuaFunction>,
  pub interact: Option<LuaFunction>,
}

pub struct LuaContext {
  lua: Lua,
  pub entities_api: HashMap<Entity, EntityLuaApi>,
}

impl LuaContext {
  fn new(lua: Lua) -> Self {
    Self { lua, entities_api: HashMap::new() }
  }
}

impl Deref for LuaContext {
  type Target = Lua;

  fn deref(&self) -> &Self::Target {
    &self.lua
  }
}

#[derive(Serialize, Deserialize, EnumDiscriminants, Debug, PartialEq)]
#[strum_discriminants(derive(Serialize, IntoStaticStr, EnumIter))]
#[strum_discriminants(name(GameEventType))]
#[serde(tag = "type", content = "data")]
pub enum GameEvent {
  DoorLock(Entity),
  DoorUnlock(Entity),
  DoorOpen(Entity),
  EntityWentDowntairs(Entity),
  EntityInteracted(Entity),
  EntityDeath { target: Entity, attacker: Entity },
}

#[derive(Serialize, Deserialize)]
pub struct GameEventManager {
  events: Vec<GameEvent>,
}

impl GameEventManager {
  pub fn new() -> Self {
    Self { events: Vec::new() }
  }

  pub fn add(&mut self, event: GameEvent) {
    self.events.push(event)
  }

  pub fn take_last(&mut self) -> Option<GameEvent> {
    self.events.pop()
  }

  pub fn as_slice(&self) -> &[GameEvent] {
    self.events.as_slice()
  }
}

impl Default for GameEventManager {
  fn default() -> Self {
    Self::new()
  }
}
