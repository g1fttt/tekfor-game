use crate::asset::{AssetID, AssetManager};
use crate::world::Grid;
use crate::{Settings, utils};

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};

use macroquad::experimental::camera::mouse::Camera;
use macroquad::prelude::*;

use std::collections::VecDeque;

pub struct State {
  pub grid: Grid,
  pub world: hecs::World,
  pub camera: Camera,
  asset_manager: AssetManager,
  player_entity: Option<hecs::Entity>,
}

impl State {
  pub async fn with_grid_size(width: u32, height: u32) -> Result<Self, macroquad::Error> {
    let grid = Grid::new(width, height);
    let world = hecs::World::new();

    let camera = Camera::new(Vec2::ZERO, 0.005);
    let asset_manager = AssetManager::load_all().await?;

    Ok(Self { grid, world, camera, asset_manager, player_entity: None })
  }

  pub fn spawn_entity(&mut self, components: impl hecs::DynamicBundle) -> hecs::Entity {
    let entity = self.world.spawn(components);

    if let Ok((pos, _)) = self.world.query_one::<(&Position, &OnGrid)>(entity).get() {
      self.grid.add_to_cell(entity, pos.x, pos.y);
    }
    entity
  }

  pub fn spawn_player_at(&mut self, pos: UVec2) -> hecs::Entity {
    let entity = self.spawn_entity((
      Sprite(AssetID::Player),
      ZIndex(1),
      Solid,
      Movable,
      OnGrid,
      Player,
      Position(pos),
      ActionQueue::default(),
    ));

    self.player_entity.replace(entity);

    entity
  }

  pub fn spawn_horizontal_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontal)
  }

  pub fn spawn_horizontal_left_edge_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallHorizontalLeftEdge)
  }

  pub fn spawn_right_lower_corner_wall_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_wall_at(pos, AssetID::WallRightLowerCorner)
  }

  fn spawn_wall_at(&mut self, pos: UVec2, id: AssetID) -> hecs::Entity {
    self.spawn_entity((Sprite(id), OnGrid, Solid, Position(pos)))
  }

  pub fn spawn_crate_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((Sprite(AssetID::Crate), OnGrid, Solid, Movable, Pushable, Position(pos)))
  }

  pub fn spawn_fireball_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Dummy),
      Movable,
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::Fireball,
      }),
    ))
  }

  pub fn spawn_fireball_thrower_at(&mut self, pos: UVec2, dir: Direction) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::Dummy),
      OnGrid,
      Position(pos),
      Facing(dir),
      Tickable(Interactable {
        linked_entity: None,
        handler_kind: InteractableHandlerKind::FireballThrower,
      }),
    ))
  }

  pub fn spawn_pressure_plate(
    &mut self,
    pos: UVec2,
    linked_entity: Option<hecs::Entity>,
  ) -> hecs::Entity {
    self.spawn_entity((
      Sprite(AssetID::PressurePlate),
      OnGrid,
      Position(pos),
      Tickable(Interactable {
        linked_entity,
        handler_kind: InteractableHandlerKind::PressurePlate,
      }),
    ))
  }

  pub fn spawn_door_at(&mut self, pos: UVec2) -> hecs::Entity {
    self.spawn_entity((
      StatefulObjectKind::Door,
      Sprite(AssetID::DoorClosed),
      OnGrid,
      Closed,
      Solid,
      Position(pos),
      Interactable { linked_entity: None, handler_kind: InteractableHandlerKind::Door },
    ))
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

    pushable_entities.into_iter().for_each(|ent| {
      self.move_entity(ent, MoveOptions::new(dir));
    });
  }

  fn move_entity_to_pos(&mut self, entity: hecs::Entity, x: u32, y: u32) -> bool {
    let is_out_of_bounds = self.grid.get_cell(x, y).is_none();

    if is_out_of_bounds || self.has_anything_solid_at(x, y) {
      return false;
    }

    let Ok((entity_pos, _, _)) =
      self.world.query_one_mut::<(&mut Position, &Movable, &OnGrid)>(entity)
    else {
      return false;
    };

    self.grid.remove_from_cell(entity, entity_pos.x, entity_pos.y);
    self.grid.add_to_cell(entity, x, y);

    let start = *entity_pos;
    {
      entity_pos.x = x;
      entity_pos.y = y;

      let _ = entity_pos;
    }
    let end = Position(uvec2(x, y));

    let _ = self.world.insert_one(entity, Animation::new(AnimationKind::Move { start, end }));

    true
  }

  fn move_entity(&mut self, entity: hecs::Entity, opts: MoveOptions) -> bool {
    if !self.world.satisfies::<(&Movable, &OnGrid)>(entity) {
      return false;
    }

    let Ok(new_pos) = self
      .world
      .get::<&Position>(entity)
      .map(|pos| advance_pos_in_direction(pos.into_inner(), opts.dir))
    else {
      return false;
    };

    if opts.push {
      self.push_entities_if_any(new_pos.x, new_pos.y, opts.dir);
    }

    self.move_entity_to_pos(entity, new_pos.x, new_pos.y)
  }

  fn interact(&mut self, entity: hecs::Entity, dir: Direction) {
    let Ok(pos) = self.world.get::<&Position>(entity).map(|pos| pos.into_inner()) else {
      return;
    };

    let target_pos = advance_pos_in_direction(pos, dir);

    let Some(cell_entities) = self.grid.get_cell(target_pos.x, target_pos.y) else {
      return;
    };

    let interactable_entities: Vec<(InteractableHandlerKind, _, _)> = cell_entities
      .iter()
      .filter_map(|&entity| {
        let interactable = self.world.get::<&Interactable>(entity).ok()?;

        Some((interactable.handler_kind, entity, interactable.linked_entity))
      })
      .collect();

    for (handler_kind, entity, linked_entity) in interactable_entities {
      handler_kind.to_fn()(self, entity, linked_entity);
    }
  }
}

impl State {
  pub fn do_tick(&mut self) {
    self.update_sprites();
    self.update_animations();

    let is_any_action_started = self.process_actions();

    if is_any_action_started {
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

    let mut render_queue = Vec::<(u32, Vec2, Sprite)>::new();

    for (pos, sprite, entity) in self.world.query::<(&Position, &Sprite, hecs::Entity)>().iter() {
      let global_pos = if let Ok(anim) = self.world.get::<&Animation>(entity)
        && let AnimationKind::Move { start, end } = anim.kind
      {
        interpolated_pos(&anim, start.global(), end.global())
      } else {
        pos.global()
      };

      let z_index = self.world.get::<&ZIndex>(entity).map(|z| z.0).unwrap_or(0);

      render_queue.push((z_index, global_pos, *sprite));
    }

    render_queue.sort_by_key(|&(z, _, _)| z);

    for (_, global_pos, sprite) in render_queue.into_iter() {
      let texture = self.asset_manager.get(sprite.into_inner());

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
        ActionKind::Move(dir) => {
          self.move_entity(entity, MoveOptions { dir, push: true });
        }
        ActionKind::Interact(dir) => self.interact(entity, dir),
        ActionKind::NoOp => (),
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

      *sprite = Sprite(asset_id);
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
  fn do_logical_tick(&mut self) {
    // Тут можно (и нужно) обновлять логическое состояние мира:
    // * Нажимные плиты
    // * Враги
    // * И т.д.

    self.update_tickable();
  }

  fn update_tickable(&mut self) {
    let tickable: Vec<(InteractableHandlerKind, _, _)> = self
      .world
      .query::<(&Tickable, hecs::Entity)>()
      .iter()
      .map(|(tickable, entity)| (tickable.handler_kind, entity, tickable.linked_entity))
      .collect();

    for (handler_kind, entity, linked_entity) in tickable {
      handler_kind.to_fn()(self, entity, linked_entity);
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
}

fn fireball_handler(state: &mut State, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  let Ok(facing_dir) = state.world.get::<&Facing>(this_entity).map(|facing| facing.0) else {
    return;
  };

  // NOTE: Сущность не удалится если она движется в сторону левой верхней границы (0, 0).
  //       Пока не знаю как это починить.
  if !state.move_entity(this_entity, MoveOptions::new(facing_dir)) {
    let _ = state.world.despawn(this_entity);
  }
}

fn fireball_thrower_handler(state: &mut State, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  let Ok((this_pos, facing_dir)) = state
    .world
    .query_one::<(&Position, &Facing)>(this_entity)
    .get()
    .map(|(pos, Facing(dir))| (pos.into_inner(), *dir))
  else {
    return;
  };

  let new_pos = advance_pos_in_direction(this_pos, facing_dir);

  if state.has_anything_solid_at(new_pos.x, new_pos.y) {
    return;
  }

  state.spawn_fireball_at(new_pos, facing_dir);
}

fn pressure_plate_handler(
  state: &mut State,
  this_entity: hecs::Entity,
  linked_entity: Option<hecs::Entity>,
) {
  let Ok(this_pos) = state.world.get::<&Position>(this_entity).map(|pos| pos.into_inner()) else {
    return;
  };

  let Some(this_cell_entities) = state.grid.get_cell(this_pos.x, this_pos.y) else {
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

fn door_handler(state: &mut State, this_entity: hecs::Entity, _: Option<hecs::Entity>) {
  if let Err(hecs::ComponentError::MissingComponent(_)) =
    state.world.remove::<(Closed, Solid)>(this_entity)
  {
    state.world.insert(this_entity, (Closed, Solid)).unwrap();
  }
}

fn advance_pos_in_direction(pos: UVec2, dir: Direction) -> UVec2 {
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

struct MoveOptions {
  dir: Direction,
  push: bool,
}

impl MoveOptions {
  fn new(dir: Direction) -> Self {
    Self { dir, push: false }
  }
}

#[derive(Serialize, Deserialize, EnumIter, IntoStaticStr, Clone, Copy, PartialEq)]
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

#[derive(Serialize, Deserialize)]
enum AnimationKind {
  Move { start: Position, end: Position },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Animation {
  kind: AnimationKind,
  elapsed: f32,
  duration: f32,
}

impl Animation {
  fn new(kind: AnimationKind) -> Self {
    Self { kind, elapsed: 0.0, duration: 0.5 }
  }
}

#[derive(Serialize, Deserialize)]
pub enum ActionKind {
  Move(Direction),
  Interact(Direction),
  NoOp,
}

#[derive(Serialize, Deserialize, Default)]
pub(super) struct ActionQueue(VecDeque<ActionKind>);

// Перечисление объектов, которые в зависимости от состояния - могут иметь разные спрайты.
#[derive(Serialize, Deserialize)]
pub(super) enum StatefulObjectKind {
  Door,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(super) struct Position(#[serde(with = "super::serialize::uvec2_serde")] UVec2);

impl Position {
  fn global(self) -> Vec2 {
    utils::global_pos(self.into_inner())
  }
}

#[derive(Serialize, Deserialize)]
pub(super) struct ZIndex(u32);

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(super) struct Sprite(AssetID);

type InteractableHandler =
  fn(&mut State, this_entity: hecs::Entity, linked_entity: Option<hecs::Entity>);

#[derive(Serialize, Deserialize, Clone, Copy)]
enum InteractableHandlerKind {
  Fireball,
  FireballThrower,
  PressurePlate,
  Door,
}

impl InteractableHandlerKind {
  fn to_fn(self) -> InteractableHandler {
    match self {
      InteractableHandlerKind::Fireball => fireball_handler,
      InteractableHandlerKind::FireballThrower => fireball_thrower_handler,
      InteractableHandlerKind::PressurePlate => pressure_plate_handler,
      InteractableHandlerKind::Door => door_handler,
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Interactable {
  linked_entity: Option<hecs::Entity>,
  handler_kind: InteractableHandlerKind,
}

#[derive(Serialize, Deserialize, Clone)]
pub(super) struct Tickable(Interactable);

#[derive(Serialize, Deserialize)]
pub(super) struct Facing(Direction);

#[derive(Serialize, Deserialize)]
pub(super) struct Closed;

#[derive(Serialize, Deserialize)]
pub(super) struct Movable;

#[derive(Serialize, Deserialize)]
pub(super) struct Pushable;

#[derive(Serialize, Deserialize)]
pub(super) struct OnGrid;

#[derive(Serialize, Deserialize)]
pub(super) struct Solid;

#[derive(Serialize, Deserialize)]
pub(super) struct Player;

deref_component!(Position, UVec2);
deref_component!(Sprite, AssetID);
deref_component!(ActionQueue, VecDeque<ActionKind>);
deref_component!(Tickable, Interactable);
