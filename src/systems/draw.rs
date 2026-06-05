use crate::components::*;
use crate::resources::{AssetManager, SpriteID};

use macroquad::prelude::*;

pub fn draw_sprites(world: &hecs::World, asset_manager: &AssetManager) {
  let mut render_queue = Vec::<(u32, Vec2, Sprite)>::new();

  for (pos, sprite, entity) in world.query::<(&Position, &Sprite, hecs::Entity)>().iter() {
    let global_pos = if let Ok(anim) = world.get::<&Animation>(entity)
      && let AnimationKind::Move { start, end } = anim.kind()
    {
      interpolated_pos(&anim, start.global(), end.global())
    } else {
      pos.global()
    };

    let z_index = world.get::<&ZIndex>(entity).map(|z| z.0).unwrap_or(0);

    render_queue.push((z_index, global_pos, *sprite));
  }

  render_queue.sort_by_key(|&(z, _, _)| z);

  for (_, global_pos, sprite) in render_queue.into_iter() {
    let sprite_id = sprite.into_inner();
    let texture = asset_manager.get_texture(sprite_id);

    draw_texture(texture, global_pos.x, global_pos.y, WHITE);
  }
}

pub fn update_sprites(world: &hecs::World) {
  let mut stateful_sprited_objects =
    world.query::<(&StatefulObjectKind, &mut Sprite, hecs::Entity)>();

  for (kind, sprite, entity) in stateful_sprited_objects.iter() {
    let sprite_id = match (kind, world.satisfies::<&Locked>(entity)) {
      (StatefulObjectKind::Door, true) => SpriteID::DoorLocked,
      (StatefulObjectKind::Door, false) => SpriteID::DoorUnlocked,
    };

    *sprite = Sprite(sprite_id);
  }
}

pub fn update_animations(world: &mut hecs::World) {
  let mut finished_entities = Vec::new();

  for (anim, entity) in world.query_mut::<(&mut Animation, hecs::Entity)>() {
    if anim.update(get_frame_time()) {
      finished_entities.push(entity);
    }
  }

  for entity in finished_entities {
    let _ = world.remove_one::<Animation>(entity);
  }
}

pub fn is_any_animation_active(world: &hecs::World) -> bool {
  world.query::<&Animation>().iter().any(|anim| !anim.is_finished())
}

fn interpolated_pos(anim: &Animation, start: Vec2, end: Vec2) -> Vec2 {
  let ease_out_quart = |n: f32| 1.0 - (1.0 - n).powi(4);

  let progress = ease_out_quart(anim.progress());

  let x = start.x + (end.x - start.x) * progress;
  let y = start.y + (end.y - start.y) * progress;

  vec2(x, y)
}
