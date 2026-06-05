use crate::resources::Settings;
use crate::serialize::*;
use crate::{GameState, utils};

use crate::states::editor::Editor;
use crate::states::gameplay::Gameplay;

use egui_macroquad::egui;

use macroquad::logging as log;
use macroquad::miniquad::window::order_quit;

use std::fs;

#[derive(Default)]
pub struct Menu {
  should_draw_window: ShouldDrawWindow,
  chosen_level: Option<String>,
  should_start_level: bool,
}

impl Menu {
  pub fn draw_ui(&mut self, egui_ctx: &egui::Context) -> Option<GameState> {
    let inner_response = egui::Window::new("Menu")
      .resizable(false)
      .movable(false)
      .collapsible(false)
      .title_bar(false)
      .show(egui_ctx, |ui| {
        let result = ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
          if ui.button("Choose level").clicked() {
            self.should_draw_window.choose_level = true;
          }

          if ui.button("Editor").clicked() {
            let editor = Box::new(Editor::new());

            return Some(GameState::Editor(editor));
          }

          if ui.button("Settings").clicked() {
            self.should_draw_window.settings = true;
          }

          if ui.button("Quit").clicked() {
            order_quit();
          }

          None
        });

        result.inner
      })
      .unwrap();

    self.draw_choose_level_window(egui_ctx);
    self.draw_settings_window(egui_ctx);

    if let Some(ref level_path) = self.chosen_level
      && self.should_start_level
    {
      let bytes = fs::read(level_path).unwrap();
      let (info, world) = deserialize_world_info(&bytes).unwrap();

      let gameplay = Box::new(Gameplay::new(info, world));

      return Some(GameState::Gameplay(gameplay));
    }

    // Изменилось ли глобальное-игровое состояние?
    //
    // Если да, то даем основному циклу узнать об этом и предпринять соответствующие меры.
    inner_response.inner.unwrap()
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

        if let Some(ref level_path) = self.chosen_level
          && ui.button("Start").clicked()
        {
          log::debug!("Level (\"{}\") was chosen", level_path);

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
}

#[derive(Default)]
struct ShouldDrawWindow {
  choose_level: bool,
  settings: bool,
}
