use crate::resources::Settings;
use crate::states::PlannedGameState;
use crate::{serialize, utils};

use egui_macroquad::egui;

use macroquad::logging as log;
use macroquad::miniquad::window::order_quit;

#[derive(Default)]
pub struct Menu {
  should_draw_window: ShouldDrawWindow,
  chosen_level: Option<String>,
  should_start_level: bool,
  should_start_editor: bool,
}

impl Menu {
  pub fn draw_ui(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Menu")
      .resizable(false)
      .movable(false)
      .collapsible(false)
      .title_bar(false)
      .show(egui_ctx, |ui| {
        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
          if ui.button("Choose level").clicked() {
            self.should_draw_window.choose_level = true;
          }

          if ui.button("Editor").clicked() {
            self.should_start_editor = true;
          }

          if ui.button("Settings").clicked() {
            self.should_draw_window.settings = true;
          }

          if ui.button("Quit").clicked() {
            order_quit();
          }
        });
      });

    self.draw_choose_level_window(egui_ctx);
    self.draw_settings_window(egui_ctx);
  }

  fn draw_choose_level_window(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Choose level")
      .resizable(false)
      .open(&mut self.should_draw_window.choose_level)
      .show(egui_ctx, |ui| {
        let selected_text = self.chosen_level.clone().unwrap_or_else(|| "...".to_owned());

        egui::ComboBox::from_label("Levels").selected_text(selected_text).show_ui(ui, |ui| {
          utils::with_entries_in("levels/", |path, filename| {
            ui.selectable_value(&mut self.chosen_level, Some(path), filename);
          })
        });

        if ui.button("Start").clicked() {
          self.should_start_level = true;
        }
      });
  }

  fn draw_settings_window(&mut self, egui_ctx: &egui::Context) {
    egui::Window::new("Settings")
      .resizable(false)
      .open(&mut self.should_draw_window.settings)
      .show(egui_ctx, |ui| {
        let mut settings = Settings::get_mut();

        ui.add(
          egui::Slider::new(&mut settings.animation_speed_multiplier, 1.0..=5.0)
            .text("Animation speed multiplier"),
        );
        ui.add(egui::Slider::new(&mut settings.ui_scale_factor, 1.0..=3.0).text("UI scale factor"));
        ui.checkbox(&mut settings.show_frames_per_second, "Show FPS");
        ui.add(egui::Slider::new(&mut settings.crt_intensity, 0.0..=1.0).text("CRT intensity"));

        if ui.button("Save").clicked() {
          let _ = settings.save();
        }
      });
  }

  pub fn planned(&self) -> Option<PlannedGameState> {
    if let Some(ref level_path) = self.chosen_level
      && self.should_start_level
    {
      match serialize::load_world(level_path).map(Box::new) {
        Ok(world_info) => Some(PlannedGameState::Gameplay(world_info)),
        Err(err) => {
          log::error!("Failed to read level at: {}. Due to: {}", level_path, err);
          None
        }
      }
    } else if self.should_start_editor {
      Some(PlannedGameState::Editor)
    } else {
      None
    }
  }
}

#[derive(Default)]
struct ShouldDrawWindow {
  choose_level: bool,
  settings: bool,
}
