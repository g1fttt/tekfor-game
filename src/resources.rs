use directories::BaseDirs;
use mlua::Lua;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::audio::{Sound, load_sound};
use macroquad::experimental::collections::storage;
use macroquad::logging as log;
use macroquad::prelude::*;

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{env, fs, io};

#[derive(Serialize, Deserialize, IntoStaticStr, EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpriteID {
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
  DownstairsHorizontalUpper,
}

#[derive(PartialEq, Eq, Hash)]
pub enum MaterialID {
  CRT,
}

#[derive(PartialEq, Eq, Hash)]
pub enum SoundID {
  Lock,
  Unlock,
  DoorOpen,
  LevelFinished,
  Death,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ScriptID {
  Fireball,
  FireballThrower,
  PressurePlate,
  Door,
  Saw,
  Downstairs,
}

pub struct AssetManager(Rc<AssetManagerInner>);

impl AssetManager {
  pub async fn load_all(lua: &Lua) -> anyhow::Result<Self> {
    Ok(Self(Rc::new(AssetManagerInner::load_all(lua).await?)))
  }

  pub fn strong_clone(&self) -> Self {
    Self(Rc::clone(&self.0))
  }
}

impl TextureProvider for AssetManager {
  fn get_texture(&self, id: SpriteID) -> &Texture2D {
    self.0.textures.get(&id).unwrap()
  }
}

impl SoundProvider for AssetManager {
  fn get_sound(&self, id: SoundID) -> &Sound {
    self.0.sounds.get(&id).unwrap()
  }
}

impl MaterialProvider for AssetManager {
  fn get_material(&self, id: MaterialID) -> &Material {
    self.0.materials.get(&id).unwrap()
  }
}

impl ScriptProvider for AssetManager {
  fn get_script(&self, id: ScriptID) -> &[u8] {
    self.0.scripts.get(&id).map(|bytes| bytes.as_slice()).unwrap()
  }
}

pub trait TextureProvider {
  fn get_texture(&self, id: SpriteID) -> &Texture2D;
}

pub trait SoundProvider {
  fn get_sound(&self, id: SoundID) -> &Sound;
}

pub trait MaterialProvider {
  fn get_material(&self, id: MaterialID) -> &Material;
}

pub trait ScriptProvider {
  fn get_script(&self, id: ScriptID) -> &[u8];
}

type Textures = HashMap<SpriteID, Texture2D>;
type Materials = HashMap<MaterialID, Material>;
type Sounds = HashMap<SoundID, Sound>;
type Scripts = HashMap<ScriptID, Vec<u8>>;

struct AssetManagerInner {
  textures: Textures,
  sounds: Sounds,
  materials: Materials,
  scripts: Scripts,
}

impl AssetManagerInner {
  async fn load_all(lua: &Lua) -> anyhow::Result<Self> {
    Ok(Self {
      textures: Self::load_textures().await?,
      sounds: Self::load_sounds().await?,
      materials: Self::load_materials()?,
      scripts: Self::load_scritps(lua)?,
    })
  }

  #[rustfmt::skip]
  async fn load_textures() -> Result<Textures, macroquad::Error> {
    let mut textures = Textures::new();

    textures.insert(SpriteID::Player, load_texture("textures/player.png").await?);
    textures.insert(SpriteID::DoorLocked, load_texture("textures/door-locked.png").await?);
    textures.insert(SpriteID::DoorUnlocked, load_texture("textures/door-unlocked.png").await?);
    textures.insert(SpriteID::WallHorizontal, load_texture("textures/wall-horizontal.png").await?);
    textures.insert(SpriteID::WallHorizontalLeftEdge, load_texture("textures/wall-horizontal-left-edge.png").await?);
    textures.insert(SpriteID::WallHorizontalRightEdge, load_texture("textures/wall-horizontal-right-edge.png").await?);
    textures.insert(SpriteID::WallLeftLowerCorner, load_texture("textures/wall-left-lower-corner.png").await?);
    textures.insert(SpriteID::WallLeftUpperCorner, load_texture("textures/wall-left-upper-corner.png").await?);
    textures.insert(SpriteID::WallRightLowerCorner, load_texture("textures/wall-right-lower-corner.png").await?);
    textures.insert(SpriteID::WallRightUpperCorner, load_texture("textures/wall-right-upper-corner.png").await?);
    textures.insert(SpriteID::WallVertical, load_texture("textures/wall-vertical.png").await?);
    textures.insert(SpriteID::PressurePlate, load_texture("textures/pressure-plate.png").await?);
    textures.insert(SpriteID::Crate, load_texture("textures/crate.png").await?);
    textures.insert(SpriteID::Saw, load_texture("textures/saw.png").await?);
    textures.insert(SpriteID::Fireball, load_texture("textures/fireball.png").await?);
    textures.insert(SpriteID::FireballThrower, load_texture("textures/translucent.png").await?);
    textures.insert(SpriteID::WallVerticalLeftSplit, load_texture("textures/wall-vertical-left-split.png").await?);
    textures.insert(SpriteID::WallVerticalRightSplit, load_texture("textures/wall-vertical-right-split.png").await?);
    textures.insert(SpriteID::WallHorizontalUpperSplit, load_texture("textures/wall-horizontal-upper-split.png").await?);
    textures.insert(SpriteID::WallHorizontalLowerSplit, load_texture("textures/wall-horizontal-lower-split.png").await?);
    textures.insert(SpriteID::WallVerticalTopEdge, load_texture("textures/wall-vertical-top-edge.png").await?);
    textures.insert(SpriteID::WallVerticalBottomEdge, load_texture("textures/wall-vertical-bottom-edge.png").await?);
    textures.insert(SpriteID::DownstairsHorizontalUpper, load_texture("textures/downstairs-horizontal-upper.png").await?);

    Ok(textures)
  }

  async fn load_sounds() -> Result<Sounds, macroquad::Error> {
    let mut sounds = Sounds::new();

    sounds.insert(SoundID::Lock, load_sound("sounds/lock.wav").await?);
    sounds.insert(SoundID::Unlock, load_sound("sounds/unlock.wav").await?);
    sounds.insert(SoundID::DoorOpen, load_sound("sounds/door-open.wav").await?);
    sounds.insert(SoundID::LevelFinished, load_sound("sounds/level-finished.wav").await?);
    sounds.insert(SoundID::Death, load_sound("sounds/death.wav").await?);

    Ok(sounds)
  }

  fn load_materials() -> Result<Materials, macroquad::Error> {
    let mut materials = Materials::new();

    materials.insert(
      MaterialID::CRT,
      load_material(
        ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: CRT_SHADER },
        MaterialParams {
          uniforms: vec![
            UniformDesc::new("Resolution", UniformType::Float2),
            UniformDesc::new("Intensity", UniformType::Float1),
            UniformDesc::new("CrtIntensity", UniformType::Float1),
          ],
          ..Default::default()
        },
      )?,
    );

    Ok(materials)
  }

  fn load_scritps(lua: &Lua) -> anyhow::Result<Scripts> {
    fn load_bytecode(lua: &Lua, path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
      Ok(lua.load(fs::read(path)?).into_function()?.dump(false))
    }

    let mut scripts = Scripts::new();

    scripts.insert(ScriptID::Fireball, load_bytecode(lua, "scripts/fireball.lua")?);
    scripts.insert(ScriptID::FireballThrower, load_bytecode(lua, "scripts/fireball_thrower.lua")?);
    scripts.insert(ScriptID::PressurePlate, load_bytecode(lua, "scripts/pressure_plate.lua")?);
    scripts.insert(ScriptID::Saw, load_bytecode(lua, "scripts/saw.lua")?);
    scripts.insert(ScriptID::Downstairs, load_bytecode(lua, "scripts/downstairs.lua")?);
    scripts.insert(ScriptID::Door, load_bytecode(lua, "scripts/door.lua")?);

    Ok(scripts)
  }
}

const VERTEX_SHADER: &str = include_str!("materials/vertex.glsl");
const CRT_SHADER: &str = include_str!("materials/crt.glsl");

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
  pub animation_speed_multiplier: f32,
  pub ui_scale_factor: f32,
  pub show_frames_per_second: bool,
  pub crt_intensity: f32,
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
      crt_intensity: 0.3,
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
