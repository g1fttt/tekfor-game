use directories::BaseDirs;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use macroquad::experimental::collections::storage;
use macroquad::logging as log;

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::{env, fs, io};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
  pub animation_speed_multiplier: f32,
  #[serde(skip)]
  settings_file_path: PathBuf,
}

impl Default for Settings {
  fn default() -> Self {
    Self { animation_speed_multiplier: 1.0, settings_file_path: Default::default() }
  }
}

impl Settings {
  pub fn init_or_load() -> io::Result<()> {
    let config_dir_path = Self::config_dir_path()?;

    let settings_file_path = config_dir_path.join("tekfor-game/settings.ron");

    let settings = if settings_file_path.exists() {
      log::info!("Loading settings from {:?}", &settings_file_path);

      let string = fs::read_to_string(&settings_file_path)?;
      let mut settings = ron::from_str::<Self>(&string).expect("Settings file is corrupted");

      settings.settings_file_path = settings_file_path;
      settings
    } else {
      log::info!("Creating defaulted settings file at {:?}", &settings_file_path);

      let settings = Self { settings_file_path, ..Default::default() };
      settings.save()?;
      settings
    };

    storage::store(settings);

    Ok(())
  }

  /// # Safety
  /// Вызовет панику если преждевременно не проинциализировать `Settings` структуру и `macroquad` хранилище.
  #[inline(always)]
  pub fn get() -> impl Deref<Target = Self> {
    Self::get_mut()
  }

  /// # Safety
  /// Вызовет панику если преждевременно не проинциализировать `Settings` структуру и `macroquad` хранилище.
  #[inline(always)]
  pub fn get_mut() -> impl DerefMut<Target = Self> {
    storage::get_mut()
  }

  pub fn save(&self) -> io::Result<()> {
    let parent_path = self.settings_file_path.parent().unwrap();
    fs::create_dir_all(parent_path)?;

    let string = ron::ser::to_string_pretty(self, PrettyConfig::default()).unwrap();
    fs::write(&self.settings_file_path, string)
  }

  fn config_dir_path() -> io::Result<PathBuf> {
    let path = BaseDirs::new()
      .map(|base_dirs| base_dirs.config_dir().to_owned())
      .unwrap_or(env::current_dir()?);
    Ok(path)
  }
}
