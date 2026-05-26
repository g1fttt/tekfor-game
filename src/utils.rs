use crate::game::Grid;

use macroquad::math::Vec2;

#[inline(always)]
pub fn global_pos(pos: Vec2) -> Vec2 {
  pos * Grid::CELL_SIZE
}
