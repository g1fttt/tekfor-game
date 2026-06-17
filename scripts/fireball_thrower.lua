local M = {}

---@param world_grid WorldGrid
---@param fireball_thrower Entity
function M.update(world_grid, _, fireball_thrower)
  local pos = world_grid:get_pos(fireball_thrower)
  local facing_dir = world_grid:get_facing_dir(fireball_thrower)

  if pos == nil or facing_dir == nil then
    return
  end

  local new_pos = require("utils").advance_pos_in_direction(pos, facing_dir)

  if not world_grid:has_obstacle_at(new_pos) then
    local fireball = world_grid:spawn_fireball(new_pos, facing_dir)

    world_grid:add_action(fireball, {
      type = ActionType.Move,
      data = { dir = facing_dir },
    })
  end
end

return M
