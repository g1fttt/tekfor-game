use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, IntoStaticStr, EnumIter, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetID {
  Player,
  DoorClosed,
  DoorOpen,
  WallHorizontal,
  WallHorizontalLeftEdge,
  WallRightLowerCorner,
  PressurePlate,
  Crate,
  Dummy,
}

pub struct AssetManager {
  textures: HashMap<AssetID, Texture2D>,
}

impl AssetManager {
  #[rustfmt::skip]
  pub async fn load_all() -> Result<Self, macroquad::Error> {
    let mut textures = HashMap::new();

    textures.insert(AssetID::Player, load_texture("textures/player.png").await?);
    textures.insert(AssetID::DoorClosed, load_texture("textures/door-closed.png").await?);
    textures.insert(AssetID::DoorOpen, load_texture("textures/door-open.png").await?);
    textures.insert(AssetID::WallHorizontal, load_texture("textures/wall-horizontal.png").await?);
    textures.insert(AssetID::WallHorizontalLeftEdge, load_texture("textures/wall-horizontal-left-edge.png").await?);
    textures.insert(AssetID::WallRightLowerCorner, load_texture("textures/wall-right-lower-corner.png").await?);
    textures.insert(AssetID::PressurePlate, load_texture("textures/pressure-plate.png").await?);
    textures.insert(AssetID::Crate, load_texture("textures/crate.png").await?);
    textures.insert(AssetID::Dummy, Texture2D::empty());

    textures.values().for_each(|tex| tex.set_filter(FilterMode::Nearest));

    Ok(Self { textures })
  }

  pub fn get(&self, id: AssetID) -> Texture2D {
    self.textures.get(&id).expect("Unknown asset id").clone()
  }
}
