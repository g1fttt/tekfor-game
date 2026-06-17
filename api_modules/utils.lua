local M = {}

---@param n number
---@return number
function saturating_sub(n, rhs)
  local r = n - rhs
  return r >= 0 and r or 0
end

---@param pos UVec2
---@param dir Direction
---@return UVec2
function M.advance_pos_in_direction(pos, dir)
  local new_pos = pos

  if dir == Direction.North then
    new_pos.y = saturating_sub(pos.y, 1)
  elseif dir == Direction.East then
    new_pos.x = pos.x + 1
  elseif dir == Direction.South then
    new_pos.y = pos.y + 1
  elseif dir == Direction.West then
    new_pos.x = saturating_sub(pos.x, 1)
  end

  return new_pos
end

return M
