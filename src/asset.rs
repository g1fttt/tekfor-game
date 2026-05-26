use macroquad::prelude::*;

use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash)]
pub enum AssetID {
  Player,
  DoorClosed,
  DoorOpen,
  WallHorizontal,
  WallHorizontalLeftEdge,
  PressurePlate,
  Crate,
}

pub struct AssetManager {
  textures: HashMap<AssetID, Texture2D>,
}

impl AssetManager {
  #[rustfmt::skip]
  pub async fn load_all() -> Result<Self, macroquad::Error> {
    let mut textures = HashMap::new();

    textures.insert(AssetID::Player, load_texture("assets/textures/player.png").await?);
    textures.insert(AssetID::DoorClosed, load_texture("assets/textures/door-closed.png").await?);
    textures.insert(AssetID::DoorOpen, load_texture("assets/textures/door-open.png").await?);
    textures.insert(AssetID::WallHorizontal, load_texture("assets/textures/wall-horizontal.png").await?);
    textures.insert(AssetID::WallHorizontalLeftEdge, load_texture("assets/textures/wall-horizontal-left-edge.png").await?);
    textures.insert(AssetID::PressurePlate, load_texture("assets/textures/pressure-plate.png").await?);
    textures.insert(AssetID::Crate, load_texture("assets/textures/crate.png").await?);

    textures.values().for_each(|tex| tex.set_filter(FilterMode::Nearest));

    Ok(Self { textures })
  }

  pub fn get(&self, id: AssetID) -> Texture2D {
    self.textures.get(&id).expect("Unknown asset id").clone()
  }
}
