use crate::world::Grid;

use macroquad::math::{UVec2, Vec2, vec2};

pub fn global_pos(pos: UVec2) -> Vec2 {
  vec2(pos.x as f32, pos.y as f32) * Grid::CELL_SIZE
}
