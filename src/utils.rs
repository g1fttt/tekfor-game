use crate::components::Sprite;
use crate::core::{CELL_SIZE, Direction};

use macroquad::math::*;

use std::path::Path;
use std::{fs, io};

pub fn global_pos(pos: UVec2) -> Vec2 {
  vec2(pos.x as f32, pos.y as f32) * CELL_SIZE
}

pub fn entity_sprite_text_default(world: &hecs::World, entity: hecs::Entity) -> &'static str {
  entity_sprite_text(world, entity).unwrap_or("? (Unknown)")
}

pub fn entity_sprite_text(world: &hecs::World, entity: hecs::Entity) -> Option<&'static str> {
  world.get::<&Sprite>(entity).map(|sprite| sprite.into_inner().into()).ok()
}

pub fn advance_pos_in_direction(pos: UVec2, dir: Direction) -> UVec2 {
  let (dest_x, dest_y) = match dir {
    Direction::North => (None, Some(pos.y.saturating_sub(1))),
    Direction::East => (Some(pos.x + 1), None),
    Direction::South => (None, Some(pos.y + 1)),
    Direction::West => (Some(pos.x.saturating_sub(1)), None),
  };

  let new_pos_x = dest_x.unwrap_or(pos.x);
  let new_pos_y = dest_y.unwrap_or(pos.y);

  uvec2(new_pos_x, new_pos_y)
}

pub fn with_entries_in<P, F>(target_path: P, mut f: F) -> io::Result<()>
where
  P: AsRef<Path>,
  F: FnMut(String, &str),
{
  for entry in fs::read_dir(target_path)? {
    let entry = entry?;

    let path = entry.path();
    if !path.is_file() {
      continue;
    }

    let path_string = path.to_str().map(|path| path.to_owned()).unwrap();
    let filename = path.file_name().and_then(|filename| filename.to_str()).unwrap();

    f(path_string, filename);
  }
  Ok(())
}

pub fn type_name_str<T>() -> &'static str {
  std::any::type_name::<T>().split("::").last().unwrap()
}

#[test]
fn type_name_str_test() {
  struct TestOuter;

  mod test_mod {
    pub struct TestInner;
  }

  assert_eq!(type_name_str::<TestOuter>(), "TestOuter");
  assert_eq!(type_name_str::<test_mod::TestInner>(), "TestInner");
}
