use crate::Settings;
use crate::asset::{AssetID, AssetManager};
use crate::game::Grid;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use std::collections::VecDeque;

pub struct State {
  pub grid: Grid,
  pub world: hecs::World,
  asset_manager: AssetManager,
  player_entity: Option<hecs::Entity>,
}

impl State {
  pub async fn with_grid_size(width: u32, height: u32) -> Result<Self, macroquad::Error> {
    let grid = Grid::new(width, height);
    let world = hecs::World::new();

    let asset_manager = AssetManager::load_all().await?;

    Ok(Self { grid, world, asset_manager, player_entity: None })
  }

  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x as u32, pos.y as u32);
    }
    entity
  }

  pub fn spawn_player_at(&mut self, pos: Vec2) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::Player)),
      ZIndex(1),
      Solid,
      Movable,
      OnGrid,
      PlayerTag,
      Position(pos),
      ActionQueue::default(),
    ));

    self.player_entity.replace(entity);

    entity
  }

  pub fn spawn_horizontal_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontal)
  }

  pub fn spawn_horizontal_left_edge_wall_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontalLeftEdge)
  }

  pub fn spawn_crate_at(&mut self, pos: Vec2) -> hecs::Entity {
    self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::Crate)),
      OnGrid,
      Solid,
      Movable,
      Pushable,
      Position(pos),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: Vec2,
    linked_entity: Option<hecs::Entity>,
  ) -> hecs::Entity {
    fn handler(state: &mut State, this_entity: hecs::Entity, linked_entity: Option<hecs::Entity>) {
      let Ok(this_pos) = state.world.query_one::<&Position>(this_entity).get().cloned() else {
        return;
      };

      let Some(this_cell_entities) = state.grid.get_cell(this_pos.x as u32, this_pos.y as u32)
      else {
        return;
      };

      let is_anything_standing_on_plate = this_cell_entities.iter().any(|&ent| {
        state.world.satisfies::<hecs::Without<&Solid, &Animation>>(ent) && ent != this_entity
      });

      let Some(linked_entity) = linked_entity else {
        return;
      };

      if is_anything_standing_on_plate {
        let _ = state.world.remove::<(Closed, Solid)>(linked_entity);
      } else {
        let _ = state.world.insert(linked_entity, (Closed, Solid));
      }
    }

    self.spawn_entity((
      Sprite(self.asset_manager.get(AssetID::PressurePlate)),
      OnGrid,
      Position(pos),
      Pressable(Interactable { linked_entity, handler }),
    ))
  }

  pub fn spawn_door_at(&mut self, pos: Vec2) -> hecs::Entity {
    fn handler(state: &mut State, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
      if let Err(hecs::ComponentError::MissingComponent(_)) =
        state.world.remove::<(Closed, Solid)>(this_entity)
      {
        state.world.insert(this_entity, (Closed, Solid)).unwrap();
      }
    }

    self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(self.asset_manager.get(AssetID::DoorClosed)),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable { linked_entity: None, handler },
    ))
  }

  fn spawn_wall_at(&mut self, pos: Vec2, id: AssetID) -> hecs::Entity {
    self.spawn_entity((Sprite(self.asset_manager.get(id)), OnGrid, Solid, Position(pos)))
  }

  fn has_anything_solid_at(&self, x: u32, y: u32) -> bool {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return false;
    };

    cell_entities.iter().any(|&ent| self.world.satisfies::<&Solid>(ent))
  }

  fn push_entities_if_any(&mut self, x: u32, y: u32, dir: Direction) {
    let Some(cell_entities) = self.grid.get_cell(x, y) else {
      return;
    };

    let pushable_entities: Vec<hecs::Entity> = cell_entities
      .iter()
      .filter(|&&ent| self.world.satisfies::<(&Movable, &Pushable)>(ent))
      .cloned()
      .collect();

    if pushable_entities.is_empty() {
      return;
    }

    let (new_x, new_y) = advance_pos_in_direction((x, y), dir);

    pushable_entities.into_iter().for_each(|ent| self.move_entity_to_pos(ent, new_x, new_y));
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) {
    let is_out_of_bounds = self.grid.get_cell(x, y).is_none();

    if is_out_of_bounds || self.has_anything_solid_at(x, y) {
      return;
    }

    let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    else {
      return;
    };

    self.grid.remove_from_cell(entity, entity_pos.x as u32, entity_pos.y as u32);
    self.grid.add_to_cell(entity, x, y);

    let x = x as f32;
    let y = y as f32;

    let start = *entity_pos;
    {
      entity_pos.x = x;
      entity_pos.y = y;

      let _ = entity_pos;
    }
    let end = Position(vec2(x, y));

    let _ = self.world.insert_one(entity, Animation::new(AnimationKind::Move { start, end }));
  }

  fn move_entity(&mut self, entity: hecs::Entity, dir: Direction) {
    if !self.world.satisfies::<(&Movable, &OnGrid)>(entity) {
      return;
    }

    let Ok((new_pos_x, new_pos_y)) = self
      .world
      .get::<&Position>(entity)
      .map(|pos| advance_pos_in_direction((pos.x as u32, pos.y as u32), dir))
    else {
      return;
    };

    self.push_entities_if_any(new_pos_x, new_pos_y, dir);
    self.move_entity_to_pos(entity, new_pos_x, new_pos_y);
  }

  fn interact(&mut self, entity: hecs::Entity, dir: Direction) {
    let Ok((pos_x, pos_y)) =
      self.world.query_one::<&Position>(entity).get().map(|pos| (pos.x as u32, pos.y as u32))
    else {
      return;
    };

    let (target_pos_x, target_pos_y) = advance_pos_in_direction((pos_x, pos_y), dir);

    let Some(cell_entities) = self.grid.get_cell(target_pos_x, target_pos_y) else {
      return;
    };

    let interactable_entities: Vec<(Interactable, hecs::Entity)> = cell_entities
      .iter()
      .filter_map(|&ent| {
        let inter = self.world.get::<&Interactable>(ent).ok()?;
        let inter = (*inter).clone();

        Some((inter, ent))
      })
      .collect();

    for (inter, entity) in interactable_entities {
      (inter.handler)(self, entity, inter.linked_entity);
    }
  }
}

impl State {
  pub fn do_tick(&mut self) {
    self.update_sprites();

    let is_any_animation_finished = self.update_animations();
    let is_any_action_started = self.process_actions();

    if is_any_animation_finished || is_any_action_started {
      self.do_logical_tick();
    }
  }

  pub fn draw_sprites(&self) {
    fn interpolated_pos(anim: &Animation, start: Vec2, end: Vec2) -> Vec2 {
      let ease_out_quart = |n: f32| 1.0 - (1.0 - n).powi(4);

      let progress = ease_out_quart((anim.elapsed / anim.duration).clamp(0.0, 1.0));

      let x = start.x + (end.x - start.x) * progress;
      let y = start.y + (end.y - start.y) * progress;

      vec2(x, y)
    }

    let mut render_queue = Vec::<(u32, Vec2, Texture2D)>::new();

    for (pos, sprite, entity) in self.world.query::<(&Position, &Sprite, hecs::Entity)>().iter() {
      let global_pos = if let Ok(anim) = self.world.get::<&Animation>(entity)
        && let AnimationKind::Move { start, end } = anim.kind
      {
        interpolated_pos(&anim, start.global(), end.global())
      } else {
        pos.global()
      };

      let z_index = self.world.get::<&ZIndex>(entity).map(|z| z.0).unwrap_or(0);

      render_queue.push((z_index, global_pos, sprite.weak_clone()));
    }

    render_queue.sort_by_key(|&(z, _, _)| z);

    for (_, global_pos, texture) in render_queue.into_iter() {
      draw_texture(&texture, global_pos.x, global_pos.y, WHITE);
    }
  }

  fn process_actions(&mut self) -> bool {
    let mut actions = Vec::new();

    for (action_queue, entity) in self.world.query::<(&mut ActionQueue, hecs::Entity)>().iter() {
      if self.world.satisfies::<&Animation>(entity) {
        continue;
      }

      if let Some(action_kind) = action_queue.pop_front() {
        actions.push((action_kind, entity));
      }
    }

    if actions.is_empty() {
      return false;
    }

    for (action_kind, entity) in actions {
      match action_kind {
        ActionKind::Move(dir) => self.move_entity(entity, dir),
        ActionKind::Interact(dir) => self.interact(entity, dir),
      }
    }
    true
  }

  fn update_sprites(&mut self) {
    let mut stateful_sprited_objects =
      self.world.query::<(&StatefulObjectKind, &mut Sprite, hecs::Entity)>();

    for (kind, sprite, entity) in stateful_sprited_objects.iter() {
      let asset_id = match (kind, self.world.satisfies::<&Closed>(entity)) {
        (StatefulObjectKind::Door, true) => AssetID::DoorClosed,
        (StatefulObjectKind::Door, false) => AssetID::DoorOpen,
      };

      let new_texture = self.asset_manager.get(asset_id);

      if sprite.0.raw_miniquad_id() != new_texture.raw_miniquad_id() {
        sprite.0 = new_texture;
      }
    }
  }

  fn update_animations(&mut self) -> bool {
    let mut finished_entities = Vec::new();

    for (animation, entity) in self.world.query_mut::<(&mut Animation, hecs::Entity)>() {
      animation.elapsed += get_frame_time() * Settings::get().animation_speed_multiplier;

      if animation.elapsed >= animation.duration {
        finished_entities.push(entity);
      }
    }

    if finished_entities.is_empty() {
      return false;
    }

    for entity in finished_entities {
      let _ = self.world.remove_one::<Animation>(entity);
    }
    true
  }
}

impl State {
  /// Вызывается после каждого действия, а также после окончания самой последней анимации.
  ///
  /// Если действий было 4, то эта функция вызовется 5 раз.
  pub fn do_logical_tick(&mut self) {
    // Тут можно (и нужно) обновлять логическое состояние мира:
    // * Нажимные плиты
    // * Враги
    // * И т.д.

    self.update_pressure_plates();
  }

  fn update_pressure_plates(&mut self) {
    let pressable: Vec<(Pressable, hecs::Entity)> = self
      .world
      .query::<(&Pressable, hecs::Entity)>()
      .iter()
      .map(|(p, e)| (p.clone(), e))
      .collect();

    for (Pressable(interactable), entity) in pressable {
      (interactable.handler)(self, entity, interactable.linked_entity)
    }
  }
}

// API
impl State {
  pub fn push_player_action(&mut self, action_kind: ActionKind) {
    let Some(entity) = self.player_entity else {
      return;
    };

    if let Ok(mut action_queue) = self.world.get::<&mut ActionQueue>(entity) {
      action_queue.push_back(action_kind);
    }
  }

  // pub fn player_pos(&mut self) -> Option<(u32, u32)> {
  //   let mut query = self.world.query_one::<&Position>(self.player_entity?);

  //   query.get().map(|pos| (pos.x as u32, pos.y as u32)).ok()
  // }
}

fn advance_pos_in_direction((pos_x, pos_y): (u32, u32), dir: Direction) -> (u32, u32) {
  let (dest_x, dest_y) = match dir {
    Direction::North => (None, Some(pos_y.saturating_sub(1))),
    Direction::East => (Some(pos_x + 1), None),
    Direction::South => (None, Some(pos_y + 1)),
    Direction::West => (Some(pos_x.saturating_sub(1)), None),
  };

  let new_pos_x = dest_x.unwrap_or(pos_x);
  let new_pos_y = dest_y.unwrap_or(pos_y);

  (new_pos_x, new_pos_y)
}

#[derive(Serialize, Deserialize, EnumIter, IntoStaticStr, Clone, Copy, Debug, PartialEq)]
pub enum Direction {
  North,
  East,
  South,
  West,
}

#[macro_export]
macro_rules! deref_component {
  ($from:ty, $into:ty) => {
    impl std::ops::DerefMut for $from {
      fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
      }
    }

    impl std::ops::Deref for $from {
      type Target = $into;

      fn deref(&self) -> &Self::Target {
        &self.0
      }
    }

    impl $from {
      #[allow(dead_code)]
      pub fn into_inner(self) -> $into {
        self.0
      }
    }
  };
}

enum AnimationKind {
  Move { start: Position, end: Position },
}

struct Animation {
  kind: AnimationKind,
  elapsed: f32,
  duration: f32,
}

impl Animation {
  fn new(kind: AnimationKind) -> Self {
    Self { kind, elapsed: 0.0, duration: 0.5 }
  }
}

pub enum ActionKind {
  Move(Direction),
  Interact(Direction),
}

#[derive(Default)]
struct ActionQueue(VecDeque<ActionKind>);

// Перечисление объектов, которые в зависимости от состояния - могут иметь разные спрайты.
enum StatefulObjectKind {
  Door,
}

#[derive(Clone, Copy)]
pub struct Position(pub Vec2);

impl Position {
  pub fn global(self) -> Vec2 {
    vec2(self.0.x * Grid::CELL_SIZE, self.0.y * Grid::CELL_SIZE)
  }
}

struct ZIndex(u32);

#[derive(Clone)]
struct Sprite(Texture2D);

#[derive(Clone)]
struct Interactable {
  linked_entity: Option<hecs::Entity>,
  handler: fn(&mut State, hecs::Entity, Option<hecs::Entity>),
}

#[derive(Clone)]
struct Pressable(Interactable);

struct Closed;
struct Movable;
struct Pushable;
struct OnGrid;
struct Solid;

struct PlayerTag;

deref_component!(Position, Vec2);
deref_component!(Sprite, Texture2D);
deref_component!(ActionQueue, VecDeque<ActionKind>);
