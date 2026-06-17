local M = {}

---@param world_grid WorldGrid
---@param game_events GameEventManager
---@param downstairs Entity
function M.update(world_grid, game_events, downstairs)
  local pos = world_grid:get_pos(downstairs)
  if pos == nil then
    return
  end

  for _, entity in pairs(world_grid:get_cell(pos)) do
    if world_grid:is_player(entity) then
      game_events:add({
        type = GameEventType.EntityWentDownstairs,
        data = entity,
      })
    end
  end
end

return M
