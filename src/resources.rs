use directories::BaseDirs;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::experimental::collections::storage;
use macroquad::logging as log;
use macroquad::prelude::*;

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::{env, fs, io};

#[derive(Serialize, Deserialize, IntoStaticStr, EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetID {
  Player,
  DoorLocked,
  DoorUnlocked,
  WallHorizontal,
  WallHorizontalLeftEdge,
  WallHorizontalRightEdge,
  WallLeftLowerCorner,
  WallLeftUpperCorner,
  WallRightLowerCorner,
  WallRightUpperCorner,
  WallVertical,
  PressurePlate,
  Crate,
  Saw,
  Fireball,
  FireballThrower,
  WallVerticalLeftSplit,
  WallVerticalRightSplit,
  WallHorizontalUpperSplit,
  WallHorizontalLowerSplit,
  WallVerticalTopEdge,
  WallVerticalBottomEdge,
}

pub struct AssetManager {
  textures: HashMap<AssetID, Texture2D>,
}

impl AssetManager {
  #[rustfmt::skip]
  pub async fn load_all() -> Result<Self, macroquad::Error> {
    let mut textures = HashMap::new();

    textures.insert(AssetID::Player, load_texture("textures/player.png").await?);
    textures.insert(AssetID::DoorLocked, load_texture("textures/door-locked.png").await?);
    textures.insert(AssetID::DoorUnlocked, load_texture("textures/door-unlocked.png").await?);
    textures.insert(AssetID::WallHorizontal, load_texture("textures/wall-horizontal.png").await?);
    textures.insert(AssetID::WallHorizontalLeftEdge, load_texture("textures/wall-horizontal-left-edge.png").await?);
    textures.insert(AssetID::WallHorizontalRightEdge, load_texture("textures/wall-horizontal-right-edge.png").await?);
    textures.insert(AssetID::WallLeftLowerCorner, load_texture("textures/wall-left-lower-corner.png").await?);
    textures.insert(AssetID::WallLeftUpperCorner, load_texture("textures/wall-left-upper-corner.png").await?);
    textures.insert(AssetID::WallRightLowerCorner, load_texture("textures/wall-right-lower-corner.png").await?);
    textures.insert(AssetID::WallRightUpperCorner, load_texture("textures/wall-right-upper-corner.png").await?);
    textures.insert(AssetID::WallVertical, load_texture("textures/wall-vertical.png").await?);
    textures.insert(AssetID::PressurePlate, load_texture("textures/pressure-plate.png").await?);
    textures.insert(AssetID::Crate, load_texture("textures/crate.png").await?);
    textures.insert(AssetID::Saw, load_texture("textures/saw.png").await?);
    textures.insert(AssetID::Fireball, load_texture("textures/fireball.png").await?);
    textures.insert(AssetID::FireballThrower, load_texture("textures/translucent.png").await?);
    textures.insert(AssetID::WallVerticalLeftSplit, load_texture("textures/wall-vertical-left-split.png").await?);
    textures.insert(AssetID::WallVerticalRightSplit, load_texture("textures/wall-vertical-right-split.png").await?);
    textures.insert(AssetID::WallHorizontalUpperSplit, load_texture("textures/wall-horizontal-upper-split.png").await?);
    textures.insert(AssetID::WallHorizontalLowerSplit, load_texture("textures/wall-horizontal-lower-split.png").await?);
    textures.insert(AssetID::WallVerticalTopEdge, load_texture("textures/wall-vertical-top-edge.png").await?);
    textures.insert(AssetID::WallVerticalBottomEdge, load_texture("textures/wall-vertical-bottom-edge.png").await?);

    Ok(Self { textures })
  }

  pub fn get(&self, id: AssetID) -> Texture2D {
    self.textures.get(&id).expect("Unknown asset id").clone()
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
  pub animation_speed_multiplier: f32,
  pub ui_scale_factor: f32,
  pub show_frames_per_second: bool,
  #[serde(skip)]
  settings_file_path: PathBuf,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      animation_speed_multiplier: 2.0,
      ui_scale_factor: 1.25,
      show_frames_per_second: false,
      settings_file_path: Default::default(),
    }
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
