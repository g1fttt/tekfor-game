pub mod editor;
pub mod gameplay;
pub mod menu;

use crate::serialize::WorldInfo;

use editor::Editor;
use gameplay::Gameplay;
use menu::Menu;

use hecs::World;

pub enum GameState {
  Menu(Menu),
  Editor(Box<Editor>),
  Gameplay(Box<Gameplay>),
}

impl Default for GameState {
  fn default() -> Self {
    Self::Menu(Menu::default())
  }
}

pub enum PlannedGameState {
  Menu,
  Editor,
  Gameplay(Box<(WorldInfo, World)>),
}
