use crate::asset::AssetID;
use crate::serialize::{ComponentID, deserialize_from_binary, serialize_as_binary};
use crate::world::*;

use egui_macroquad::egui;
use strum::IntoEnumIterator;

use macroquad::logging as log;
use macroquad::prelude::*;

use std::fs;
use std::ops::{Deref, DerefMut};

type ComponentAdder = Box<dyn Fn(&mut hecs::EntityBuilder)>;

pub struct LevelEditor {
  level_path: String,
  component_id: ComponentID,
  component_adders: Vec<ComponentAdder>,
  should_add_component: bool,
  inner_state: InnerState,
}

pub struct InnerState {
  cursor_pos: UVec2,
  entities_under_cursor: Vec<hecs::Entity>,
  facing_dir: Direction,
  asset_id: AssetID,
  z_index: u32,
  stateful_object_kind: StatefulObjectKind,
}

impl Default for InnerState {
  fn default() -> Self {
    Self {
      cursor_pos: UVec2::ZERO,
      entities_under_cursor: Vec::new(),
      facing_dir: Direction::North,
      asset_id: AssetID::Dummy,
      z_index: 0,
      stateful_object_kind: StatefulObjectKind::Door,
    }
  }
}

impl Deref for LevelEditor {
  type Target = InnerState;

  fn deref(&self) -> &Self::Target {
    &self.inner_state
  }
}

impl DerefMut for LevelEditor {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner_state
  }
}

impl LevelEditor {
  pub fn new() -> Self {
    Self {
      level_path: String::new(),
      component_id: ComponentID::Sprite,
      component_adders: Vec::new(),
      should_add_component: false,
      inner_state: InnerState::default(),
    }
  }

  pub fn cursor_pos(&self) -> UVec2 {
    self.cursor_pos
  }

  pub fn update_ui(&mut self, state: &mut State, egui_ctx: &egui::Context) {
    egui::Window::new("Level editor").resizable(false).show(egui_ctx, |ui| {
      let save_load_result = ui.horizontal(|ui| {
        egui::TextEdit::singleline(&mut self.level_path).hint_text("Level path").show(ui);

        if ui.button("Save").clicked() {
          let bytes = serialize_as_binary(&state.world)?;

          fs::write(&self.level_path, bytes)?;
        }

        if ui.button("Load").clicked() {
          let bytes = fs::read(&self.level_path)?;

          state.world = deserialize_from_binary(&bytes)?;
        }
        Ok::<(), anyhow::Error>(())
      });

      if let Err(err) = save_load_result.inner {
        log::error!("{}", err);
      }

      for &entity in self.entities_under_cursor.iter() {
        ui.label(format!("{:?}", entity));
      }

      let selected_text: &'static str = self.component_id.into();

      let available_components =
        ComponentID::iter().filter(|comp_id| is_available_in_level_editor(*comp_id));

      ui.horizontal(|ui| {
        // TODO: Поддержка Interactable & Tickable
        egui::ComboBox::from_label("Component").selected_text(selected_text).show_ui(ui, |ui| {
          for comp_id in available_components {
            let text: &'static str = comp_id.into();

            ui.selectable_value(&mut self.component_id, comp_id, text);
          }
        });

        self.should_add_component = ui.button("Add").clicked();
      });

      match self.component_id {
        ComponentID::Sprite => self.sprite_ui(ui),
        ComponentID::StatefulObjectKind => self.stateful_ui(ui),
        ComponentID::ZIndex => self.z_index_ui(ui),
        ComponentID::Facing => self.facing_ui(ui),
        ComponentID::Pushable => self.try_add_component(Pushable),
        ComponentID::Movable => self.try_add_component(Movable),
        ComponentID::Closed => self.try_add_component(Closed),
        ComponentID::Solid => self.try_add_component(Solid),
        _ => (),
      };

      if !self.component_adders.is_empty() && ui.button("Spawn entity").clicked() {
        let mut builder = hecs::EntityBuilder::new();

        for adder in self.component_adders.iter() {
          adder(&mut builder);
        }

        builder.add_bundle((Position(self.cursor_pos), OnGrid));

        let entity = state.spawn_entity(builder.build());

        log::debug!("Spawned entity via level editor: {:?}", entity);

        self.component_adders.clear();
      }
    });
  }

  pub fn update(&mut self, state: &mut State, ui_wants_pointer_input: bool) {
    if !ui_wants_pointer_input {
      self.update_input();
    }

    if let Some(cell_entities) = state.grid.get_cell(self.cursor_pos.x, self.cursor_pos.y) {
      self.entities_under_cursor.clear();

      for &entity in cell_entities {
        self.entities_under_cursor.push(entity);
      }
    }
  }

  fn update_input(&mut self) {
    let Some(key_pressed) = get_last_key_pressed() else {
      return;
    };

    let dir = match key_pressed {
      KeyCode::W => Direction::North,
      KeyCode::A => Direction::West,
      KeyCode::S => Direction::South,
      KeyCode::D => Direction::East,
      _ => return,
    };

    self.cursor_pos = advance_pos_in_direction(self.cursor_pos, dir);
  }

  fn sprite_ui(&mut self, ui: &mut egui::Ui) {
    let selected_text: &'static str = self.asset_id.into();

    egui::ComboBox::from_label("Asset ID").selected_text(selected_text).show_ui(ui, |ui| {
      for id in AssetID::iter() {
        let text: &'static str = id.into();

        ui.selectable_value(&mut self.asset_id, id, text);
      }
    });

    self.try_add_component(Sprite(self.asset_id));
  }

  fn stateful_ui(&mut self, ui: &mut egui::Ui) {
    let selected_text: &'static str = self.asset_id.into();

    egui::ComboBox::from_label("Stateful object kind").selected_text(selected_text).show_ui(
      ui,
      |ui| {
        for stateful in StatefulObjectKind::iter() {
          let text: &'static str = stateful.into();

          ui.selectable_value(&mut self.stateful_object_kind, stateful, text);
        }
      },
    );

    self.try_add_component(self.stateful_object_kind);
  }

  fn z_index_ui(&mut self, ui: &mut egui::Ui) {
    ui.add(egui::Slider::new(&mut self.z_index, 0..=100));

    self.try_add_component(ZIndex(self.z_index));
  }

  fn facing_ui(&mut self, ui: &mut egui::Ui) {
    let selected_text: &'static str = self.facing_dir.into();

    egui::ComboBox::from_label("Direction").selected_text(selected_text).show_ui(ui, |ui| {
      for dir in Direction::iter() {
        let text: &'static str = dir.into();

        ui.selectable_value(&mut self.facing_dir, dir, text);
      }
    });

    self.try_add_component(Facing(self.facing_dir));
  }

  fn try_add_component<C: hecs::Component + Clone>(&mut self, component: C) {
    if !self.should_add_component {
      return;
    }

    self.component_adders.push(Box::new(move |builder| {
      builder.add(component.clone());
    }));
  }
}

fn is_available_in_level_editor(comp_id: ComponentID) -> bool {
  matches!(
    comp_id,
    ComponentID::Closed
      | ComponentID::Facing
      | ComponentID::Movable
      | ComponentID::Interactable
      | ComponentID::Player
      | ComponentID::Pushable
      | ComponentID::Solid
      | ComponentID::Sprite
      | ComponentID::Tickable
      | ComponentID::StatefulObjectKind
      | ComponentID::ZIndex
  )
}
