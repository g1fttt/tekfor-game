use crate::components::*;
use crate::resources::AssetID;
use crate::states::menu::Menu;
use crate::{Direction, Game, GameState, Grid, scripting, utils};

use crate::systems::draw::*;
use crate::systems::tick::update_tickable;

use egui_macroquad::egui;
use mlua::Lua;

use macroquad::logging as log;
use macroquad::prelude::*;

use std::fs;

pub struct Gameplay {
  pub grid: Grid,
  pub world: hecs::World,
  script_path: Option<String>,
  player_entity: Option<hecs::Entity>,
}

impl Gameplay {
  pub fn with_world(mut world: hecs::World) -> Self {
    // TODO: Генерировать сетку динамически
    let grid = Grid::new(32, 32, &mut world);

    let player_entity =
      world.query_mut::<(&Player, hecs::Entity)>().into_iter().map(|(_, entity)| entity).next();

    Self { grid, world, script_path: None, player_entity }
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
      draw_sprites(&self.world, &state.asset_manager);
    });
  }

  pub fn update(&mut self) {
    update_sprites(&self.world);
    update_animations(&mut self.world);

    let is_any_action_started = self.process_actions();

    if is_any_action_started {
      self.do_logical_tick();
    }
  }

  /// Вызывается после каждого действия, а также после окончания самой последней анимации.
  ///
  /// Если действий было 4, то эта функция вызовется 5 раз.
  fn do_logical_tick(&mut self) {
    // Тут можно (и нужно) обновлять логическое состояние мира:
    // * Нажимные плиты
    // * Враги
    // * И т.д.

    update_tickable(self);
  }

  fn process_actions(&mut self) -> bool {
    let mut actions = Vec::new();

    for (action_queue, entity) in self.world.query::<(&mut ActionQueue, hecs::Entity)>().iter() {
      if self.world.satisfies::<&Animation>(entity) {
        continue;
      }

      if let Some(action_kind) = action_queue.pop_front() {
        actions.push((action_kind, entity));
      }
    }

    if actions.is_empty() {
      return false;
    }

    for (action_kind, entity) in actions {
      match action_kind {
        ActionKind::Move(dir) => {
          self.move_entity(entity, MoveOptions { dir, push: true });
        }
        ActionKind::Interact(dir) => self.interact(entity, dir),
        ActionKind::NoOp => (),
      }
    }
    true
  }
}

impl Gameplay {
  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x, pos.y);
    }
    entity
  }

  pub fn spawn_player_at(&mut self, pos: UVec2) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(AssetID::Player),
      ZIndex(1),
      Solid,
      Movable,
      OnGrid,
      Player,
      Position(pos),
      ActionQueue::default(),
    ));

    self.player_entity.replace(entity);

    entity
  }

  pub fn spawn_horizontal_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontal)
  }

  pub fn spawn_horizontal_left_edge_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontalLeftEdge)
  }

  pub fn spawn_right_lower_corner_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallRightLowerCorner)
  }

  fn spawn_wall_at(&mut self, pos: UVec2, id: AssetID) -> hecs::Entity {
    self.spawn_entity((Sprite(id), OnGrid, Solid, Position(pos)))
  }

  pub fn spawn_crate_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((Sprite(AssetID::Crate), OnGrid, Solid, Movable, Pushable, Position(pos)))
  }

  pub fn spawn_fireball_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Dummy),
      Movable,
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::Fireball,
      }),
    ))
  }

  pub fn spawn_fireball_thrower_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Dummy),
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::FireballThrower,
      }),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: UVec2,
    linked_entity: Option<hecs::Entity>,
  ) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::PressurePlate),
      OnGrid,
      Position(pos),
      Tickable(Interactable {
        linked_entity,
        handler_kind: InteractableHandlerKind::PressurePlate,
      }),
    ))
  }

  pub fn spawn_door_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(AssetID::DoorClosed),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable { linked_entity: None, handler_kind: InteractableHandlerKind::Door },
    ))
  }

  pub fn has_anything_solid_at(&self, x: u32, y: u32) -> bool {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return false;
    };

    cell_entities.iter().any(|&ent| self.world.satisfies::<&Solid>(ent))
  }

  pub fn push_player_action(&mut self, action_kind: ActionKind) {
    let Some(entity) = self.player_entity else {
      return;
    };

    if let Ok(mut action_queue) = self.world.get::<&mut ActionQueue>(entity) {
      action_queue.push_back(action_kind);
    }
  }

  pub fn move_entity(&mut self, entity: hecs::Entity, opts: MoveOptions) -> bool {
    if !self.world.satisfies::<(&Movable, &OnGrid)>(entity) {
      return false;
    }

    let Ok(new_pos) = self
      .world
      .get::<&Position>(entity)
      .map(|pos| utils::advance_pos_in_direction(pos.into_inner(), opts.dir))
    else {
      return false;
    };

    if opts.push {
      self.push_entities_if_any(new_pos.x, new_pos.y, opts.dir);
    }

    self.move_entity_to_pos(entity, new_pos.x, new_pos.y)
  }

  pub fn interact(&mut self, entity: hecs::Entity, dir: Direction) {
    let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
      return;
    };

    let target_pos = utils::advance_pos_in_direction(pos, dir);

    let Some(cell_entities) = self.grid.get_cell(target_pos.x, target_pos.y) else {
      return;
    };

    let interactable_entities: Vec<(InteractableHandlerKind, _, _)> = cell_entities
      .iter()
      .filter_map(|&entity| {
        let interactable = self.world.get::<&Interactable>(entity).ok()?;

        Some((interactable.handler_kind, entity, interactable.linked_entity))
      })
      .collect();

    for (handler_kind, entity, linked_entity) in interactable_entities {
      handler_kind.to_fn()(self, entity, linked_entity);
    }
  }

  fn push_entities_if_any(&mut self, x: u32, y: u32, dir: Direction) {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return;
    };

    let pushable_entities: Vec<hecs::Entity> = cell_entities
      .iter()
      .filter(|&&ent| self.world.satisfies::<(&Movable, &Pushable)>(ent))
      .cloned()
      .collect();

    if pushable_entities.is_empty() {
      return;
    }

    pushable_entities.into_iter().for_each(|ent| {
      self.move_entity(ent, MoveOptions::new(dir));
    });
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) -> bool {
    let is_out_of_bounds = self.grid.get_cell(x, y).is_none();

    if is_out_of_bounds || self.has_anything_solid_at(x, y) {
      return false;
    }

    let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    else {
      return false;
    };

    self.grid.remove_from_cell(entity, entity_pos.x, entity_pos.y);
    self.grid.add_to_cell(entity, x, y);

    let start = *entity_pos;
    {
      entity_pos.x = x;
      entity_pos.y = y;

      let _ = entity_pos;
    }
    let end = Position(uvec2(x, y));

    let _ = self.world.insert_one(entity, Animation::new(AnimationKind::Move { start, end }));

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
