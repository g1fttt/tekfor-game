local M = {}

---@param world_grid WorldGrid
---@param saw Entity
function M.update(world_grid, _, saw)
  local pos = world_grid:get_pos(saw)
  local bouncing = world_grid:get_bouncing(saw)

  if pos == nil or bouncing == nil then
    return
  end

  local new_pos = require("utils").advance_pos_in_direction(pos, bouncing.to)

  if world_grid:has_obstacle_at(new_pos) then
    local new_bouncing = world_grid:switch_bouncing_dir(saw)

    if new_bouncing ~= nil then
      bouncing = new_bouncing
    else
      return
    end
  end

  world_grid:add_action(saw, {
    type = ActionType.Move,
    data = { dir = bouncing.to },
  })
end

return M
