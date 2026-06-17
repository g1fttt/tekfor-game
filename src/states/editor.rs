use crate::components::*;
use crate::core::{CELL_SIZE, Direction, Game, WorldGrid};
use crate::lock_picking::LockKind;
use crate::resources::{AssetManager, SpriteID};
use crate::serialize::{self, WorldInfo};
use crate::states::PlannedGameState;
use crate::systems::draw::draw_sprites;
use crate::utils;

use egui_macroquad::egui;
use strum::IntoEnumIterator;

use macroquad::logging as log;
use macroquad::prelude::*;

use std::collections::HashSet;

pub struct Editor {
  asset_manager: AssetManager,
  level_path: String,
  world_grid: WorldGrid,
  world_info: WorldInfo,
  cursor_pos: UVec2,
  selected_entity: Option<hecs::Entity>,
  should_capture_keyboard: bool,
  should_return_to_menu: bool,
  is_in_linkage_mode: bool,
  entity_info: EntityInfo,
}

impl Editor {
  pub fn new(asset_manager: AssetManager) -> Self {
    Self {
      asset_manager,
      level_path: String::new(),
      world_grid: WorldGrid::default(),
      world_info: WorldInfo::default(),
      cursor_pos: UVec2::ZERO,
      selected_entity: None,
      should_capture_keyboard: false,
      should_return_to_menu: false,
      is_in_linkage_mode: false,
      entity_info: EntityInfo::default(),
    }
  }

  pub fn draw(&self, state: &Game) {
    state.with_camera(None, || {
      draw_sprites(&self.world_grid, &self.asset_manager);

      self.draw_cursor();
    });
  }

  pub fn draw_ui(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Level editor").resizable(false).show(egui_ctx, |ui| {
      if ui.button("Return to main menu").clicked() {
        self.should_return_to_menu = true;
      }

      ui.separator();

      let save_load_result = ui.horizontal(|ui| {
        let resp = egui::TextEdit::singleline(&mut self.level_path)
          .hint_text("Level path")
          .show(ui)
          .response;

        self.should_capture_keyboard = resp.clicked() || resp.changed();

        if ui.button("Save").clicked() {
          serialize::save_world(&self.level_path, &self.world_info, &self.world_grid)?;
        }

        if ui.button("Load").clicked() {
          let (info, world) = serialize::load_world(&self.level_path)?;

          self.world_grid = WorldGrid::new(&info, world);
          self.world_info = info;
        }
        Ok::<(), anyhow::Error>(())
      });

      if let Err(err) = save_load_result.inner {
        log::error!("{}", err);
      }

      let is_world_width_changed =
        draw_drag_value_ui("World width", &mut self.world_info.width, ui).changed();
      let is_world_height_changed =
        draw_drag_value_ui("World height", &mut self.world_info.height, ui).changed();

      let should_resize_grid = is_world_width_changed || is_world_height_changed;

      if should_resize_grid {
        self.world_grid.resize(self.world_info.width, self.world_info.height);
      }

      ui.separator();

      let is_in_bounds = self.world_grid.get_cell(self.cursor_pos.x, self.cursor_pos.y).is_some();

      if is_in_bounds {
        self.draw_current_entity_ui(ui);
        self.draw_sprite_ui(ui);

        ui.separator();
      }

      ui.label(format!("Position: x: {}, y: {}", self.cursor_pos.x, self.cursor_pos.y));
    });
  }

  pub fn planned(&self) -> Option<PlannedGameState> {
    if self.should_return_to_menu {
      return Some(PlannedGameState::Menu);
    }

    None
  }

  pub fn update(&mut self, ui_wants_input: bool) {
    if ui_wants_input && self.should_capture_keyboard {
      return;
    }

    self.update_input();
  }

  fn update_input(&mut self) {
    let Some(key_pressed) = get_last_key_pressed() else {
      return;
    };

    if key_pressed == KeyCode::Backspace {
      self.try_despawn_selected_entity()
    }

    let dir = match key_pressed {
      KeyCode::W => Direction::North,
      KeyCode::A => Direction::West,
      KeyCode::S => Direction::South,
      KeyCode::D => Direction::East,
      _ => return,
    };

    self.cursor_pos = crate::utils::advance_pos_in_direction(self.cursor_pos, dir);
    self.selected_entity = self.last_entity_under_cursor();
  }

  fn last_entity_under_cursor(&self) -> Option<hecs::Entity> {
    let cell_entities = self.world_grid.get_cell(self.cursor_pos.x, self.cursor_pos.y)?;
    cell_entities.last().copied()
  }

  fn try_despawn_selected_entity(&mut self) {
    if let Some(entity) = self.selected_entity {
      let _ = self.world_grid.despawn_entity(entity);

      self.selected_entity = self.last_entity_under_cursor();
    }
  }

  fn draw_cursor(&self) {
    let x = self.cursor_pos.x as f32 * CELL_SIZE;
    let y = self.cursor_pos.y as f32 * CELL_SIZE;

    let color = if self.is_in_linkage_mode { GREEN } else { WHITE };

    draw_rectangle_lines(x, y, CELL_SIZE, CELL_SIZE, 2.0, color);
  }

  fn draw_current_entity_ui(&mut self, ui: &mut egui::Ui) {
    let Some(cell_entities) = self.world_grid.get_cell(self.cursor_pos.x, self.cursor_pos.y) else {
      return;
    };

    let selected_text: &'static str = self
      .selected_entity
      .and_then(|entity| utils::entity_sprite_text(&self.world_grid, entity))
      .unwrap_or("...");

    egui::ComboBox::from_label("Current entity").selected_text(selected_text).show_ui(ui, |ui| {
      for &entity in cell_entities {
        let Some(sprite_text) = utils::entity_sprite_text(&self.world_grid, entity) else {
          continue;
        };

        ui.selectable_value(&mut self.selected_entity, Some(entity), sprite_text);
      }
    });
  }

  fn draw_sprite_ui(&mut self, ui: &mut egui::Ui) {
    let selected_text: &'static str = self.entity_info.sprite_id.map(Into::into).unwrap_or("...");

    egui::ComboBox::from_label("Asset").selected_text(selected_text).show_ui(ui, |ui| {
      for sprite_id in SpriteID::iter() {
        let text: &'static str = sprite_id.into();

        ui.selectable_value(&mut self.entity_info.sprite_id, Some(sprite_id), text);
      }
    });

    self.draw_sprite_param_ui(ui);

    if self.selected_entity.is_some() && ui.button("Despawn entity").clicked() {
      self.try_despawn_selected_entity();
    }

    ui.separator();

    ui.checkbox(&mut self.is_in_linkage_mode, "Linkage mode");

    ui.separator();

    ui.label("Linked entities:");

    egui::ScrollArea::vertical().show(ui, |ui| {
      for &entity in self.entity_info.linked_entities.iter() {
        let Some(sprite_text) = utils::entity_sprite_text(&self.world_grid, entity) else {
          continue;
        };

        let Ok(pos) = self.world_grid.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
          continue;
        };

        ui.label(format!("{} (x: {}, y: {})", sprite_text, pos.x, pos.y));
      }
    });

    let has_linked_entities = !self.entity_info.linked_entities.is_empty();

    if self.is_in_linkage_mode || has_linked_entities {
      ui.separator();
    }

    ui.horizontal(|ui| {
      if let Some(entity) = self.selected_entity
        && self.is_in_linkage_mode
        && ui.button("Link").clicked()
      {
        self.entity_info.linked_entities.insert(entity);
      }

      if has_linked_entities && ui.button("Clear linked").clicked() {
        self.entity_info.linked_entities.clear();
      }
    });
  }

  #[rustfmt::skip]
  fn draw_sprite_param_ui(&mut self, ui: &mut egui::Ui) {
    let Some(sprite_id) = self.entity_info.sprite_id else {
      return;
    };

    ui.separator();

    let spawned_entity = match sprite_id {
      wall_sprite_id @ (SpriteID::WallHorizontal
      | SpriteID::WallHorizontalLeftEdge
      | SpriteID::WallHorizontalRightEdge
      | SpriteID::WallLeftLowerCorner
      | SpriteID::WallLeftUpperCorner
      | SpriteID::WallRightLowerCorner
      | SpriteID::WallRightUpperCorner
      | SpriteID::WallVertical
      | SpriteID::WallVerticalLeftSplit
      | SpriteID::WallVerticalRightSplit
      | SpriteID::WallHorizontalUpperSplit
      | SpriteID::WallHorizontalLowerSplit
      | SpriteID::WallVerticalTopEdge
      | SpriteID::WallVerticalBottomEdge) => self.draw_plain_sprite_ui(ui, |this| {
        Some(this.world_grid.spawn_entity(wall_template(this.cursor_pos, wall_sprite_id)))
      }),
      SpriteID::Crate => self.draw_plain_sprite_ui(ui, |this| {
        Some(this.world_grid.spawn_entity(crate_template(this.cursor_pos)))
      }),
      SpriteID::Player => self.draw_plain_sprite_ui(ui, |this| {
        Some(this.world_grid.spawn_entity(player_template(this.cursor_pos)))
      }),
      SpriteID::DoorUnlocked => self.draw_plain_sprite_ui(ui, |this| {
        Some(this.world_grid.spawn_entity(door_template(this.cursor_pos, false)))
      }),
      downstairs_sprite_id @ SpriteID::DownstairsHorizontalUpper => self.draw_plain_sprite_ui(ui, |this| {
        Some(this.world_grid.spawn_entity(downstairs_template(this.cursor_pos, downstairs_sprite_id)))
      }),
      SpriteID::DoorLocked => self.draw_door_locked_ui(ui),
      SpriteID::PressurePlate => self.draw_pressure_plate_ui(ui),
      SpriteID::Saw => self.draw_saw_ui(ui),
      SpriteID::Fireball => self.draw_fireball_ui(ui),
      SpriteID::FireballThrower => self.draw_fireball_thrower_ui(ui),
    };

    if let Some(entity) = spawned_entity {
      self.selected_entity.replace(entity);
    }
  }

  fn draw_door_locked_ui(&mut self, ui: &mut egui::Ui) -> Option<hecs::Entity> {
    let selected_text: &'static str = self.entity_info.lock_kind.map(Into::into).unwrap_or("...");

    egui::ComboBox::from_label("Lock kind").selected_text(selected_text).show_ui(ui, |ui| {
      for lock_kind in LockKind::iter() {
        let text: &'static str = lock_kind.into();

        ui.selectable_value(&mut self.entity_info.lock_kind, Some(lock_kind), text);
      }
    });

    let door_locked_entity = self.draw_plain_sprite_ui(ui, |this| {
      Some(this.world_grid.spawn_entity(door_template(this.cursor_pos, true)))
    });

    if let Some(entity) = door_locked_entity
      && let Some(lock_kind) = self.entity_info.lock_kind
    {
      let _ = self.world_grid.insert_one(entity, Locked(lock_kind));
    }
    door_locked_entity
  }

  fn draw_fireball_ui(&mut self, ui: &mut egui::Ui) -> Option<hecs::Entity> {
    draw_direction_ui("Direction", &mut self.entity_info.direction_to, ui);

    self.draw_plain_sprite_ui(ui, |this| {
      let fireball_entity = this
        .world_grid
        .spawn_entity(fireball_template(this.cursor_pos, this.entity_info.direction_to?));
      Some(fireball_entity)
    })
  }

  fn draw_fireball_thrower_ui(&mut self, ui: &mut egui::Ui) -> Option<hecs::Entity> {
    draw_direction_ui("Facing", &mut self.entity_info.direction_to, ui);

    self.draw_plain_sprite_ui(ui, |this| {
      let firebal_thrower_entity = this
        .world_grid
        .spawn_entity(fireball_thrower_template(this.cursor_pos, this.entity_info.direction_to?));
      Some(firebal_thrower_entity)
    })
  }

  fn draw_pressure_plate_ui(&mut self, ui: &mut egui::Ui) -> Option<hecs::Entity> {
    if self.entity_info.linked_entities.is_empty() {
      draw_entity_linkage_warn(ui);
    }

    self.draw_plain_sprite_ui(ui, |this| {
      let pressure_plate_entity =
        this.world_grid.spawn_entity(pressure_plate_template(this.cursor_pos));

      let linked_entities = &this.entity_info.linked_entities;

      if !linked_entities.is_empty() {
        let _ = this
          .world_grid
          .insert_one(pressure_plate_entity, LinkedEntities::new(linked_entities.clone()));
      }
      Some(pressure_plate_entity)
    })
  }

  fn draw_saw_ui(&mut self, ui: &mut egui::Ui) -> Option<hecs::Entity> {
    ui.horizontal(|ui| {
      draw_direction_ui("From", &mut self.entity_info.direction_from, ui);
      draw_direction_ui("To", &mut self.entity_info.direction_to, ui);
    });

    self.draw_plain_sprite_ui(ui, |this| {
      let (from, to) = this.entity_info.direction_from.zip(this.entity_info.direction_to)?;

      Some(this.world_grid.spawn_entity(saw_template(this.cursor_pos, from, to)))
    })
  }

  fn draw_plain_sprite_ui(
    &mut self,
    ui: &mut egui::Ui,
    f: impl Fn(&mut Self) -> Option<hecs::Entity>,
  ) -> Option<hecs::Entity> {
    if ui.button("Spawn entity").clicked() { f(self) } else { None }
  }
}

fn draw_direction_ui(label: &str, dir: &mut Option<Direction>, ui: &mut egui::Ui) {
  let selected_text: &'static str = dir.map(Into::into).unwrap_or("...");

  egui::ComboBox::from_label(label).selected_text(selected_text).show_ui(ui, |ui| {
    for curr_dir in Direction::iter() {
      let text: &'static str = curr_dir.into();

      ui.selectable_value(dir, Some(curr_dir), text);
    }
  });
}

fn draw_drag_value_ui<N>(label: &str, value: &mut N, ui: &mut egui::Ui) -> egui::Response
where
  N: egui::emath::Numeric,
{
  ui.horizontal(|ui| {
    let resp = ui.add(egui::DragValue::new(value));
    {
      ui.label(label);
    }
    resp
  })
  .inner
}

fn draw_entity_linkage_warn(ui: &mut egui::Ui) {
  const YELLOW_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 255, 0);

  ui.colored_label(YELLOW_COLOR, "Has soft entity-linkage requirement")
    .on_hover_text(
      "This asset still can be spawned, but will bear no logic and will act like a decoration",
    )
    .on_hover_cursor(egui::CursorIcon::Default);
}

#[derive(Default)]
pub struct EntityInfo {
  sprite_id: Option<SpriteID>,
  linked_entities: HashSet<hecs::Entity>,
  direction_from: Option<Direction>,
  direction_to: Option<Direction>,
  lock_kind: Option<LockKind>,
}
