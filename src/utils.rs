use crate::components::Sprite;
use crate::core::{CELL_SIZE, Direction};

use hecs::{Entity, World};
use macroquad::math::*;

use std::borrow::Cow;
use std::path::Path;
use std::{any, fs, io};

pub fn global_pos(pos: UVec2) -> Vec2 {
  pos.as_vec2() * CELL_SIZE
}

pub fn entity_sprite_text_default(world: &World, entity: Entity) -> Cow<'static, str> {
  entity_sprite_text(world, entity).unwrap_or(Cow::Borrowed("Unknown"))
}

pub fn entity_sprite_text(world: &World, entity: Entity) -> Option<Cow<'static, str>> {
  world.get::<&Sprite>(entity).map(|sprite| sprite.into_inner().humanize()).ok()
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

pub fn pascal_to_sentence_ascii<'a>(string: &'a str) -> Option<Cow<'a, str>> {
  let is_sentence = string.trim().contains(|ch: char| ch.is_ascii_whitespace());
  if is_sentence {
    return Some(Cow::Borrowed(string));
  }

  let is_uppercase = string.chars().all(|ch| ch.is_ascii_uppercase());
  if is_uppercase {
    return None;
  }

  let mut owned_string = string.to_owned();
  let mut ws_counter = 0;

  for (i, char) in string.chars().enumerate() {
    if i == 0 {
      continue;
    }

    let i = i + ws_counter;

    if char.is_ascii_uppercase() {
      owned_string.remove(i);
      owned_string.insert(i, char.to_ascii_lowercase());
      owned_string.insert(i, ' ');

      ws_counter += 1;
    }
  }
  Some(Cow::Owned(owned_string))
}

pub trait HumanString<'a> {
  fn humanize(self) -> Cow<'a, str>;
}

impl<'a: 'static, T> HumanString<'a> for T
where
  T: Into<&'a str> + 'a,
  &'a str: From<&'a T>,
{
  fn humanize(self) -> Cow<'a, str> {
    pascal_to_sentence_ascii(self.into()).unwrap()
  }
}

pub fn type_name_str<T>() -> Option<&'static str> {
  let type_name = any::type_name::<T>();
  if type_name.contains(['<', '>']) {
    return None;
  }
  type_name.split("::").last()
}

#[test]
fn pascal_to_sentence_ascii_test() {
  assert_eq!(pascal_to_sentence_ascii("PascalCase"), Some(Cow::Borrowed("Pascal case")));
  assert_eq!(pascal_to_sentence_ascii("UPPER"), None);
}

#[test]
fn type_name_str_test() {
  struct TestOuter;

  mod test_mod {
    pub struct TestInner;
  }

  struct TestGeneric<T>(std::marker::PhantomData<T>);

  assert_eq!(type_name_str::<TestOuter>(), Some("TestOuter"));
  assert_eq!(type_name_str::<test_mod::TestInner>(), Some("TestInner"));
  assert_eq!(type_name_str::<TestGeneric<i32>>(), None);
}
