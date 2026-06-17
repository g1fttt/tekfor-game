local M = {}

---@param world_grid WorldGrid
---@param game_events GameEventManager
---@param pressure_plate Entity
function M.update(world_grid, game_events, pressure_plate)
  local pos = world_grid:get_pos(pressure_plate)
  local linked_entities = world_grid:get_linked_entities(pressure_plate)

  if pos == nil or linked_entities == nil then
    return
  end

  local cell = world_grid:get_cell(pos)
  if cell == nil then
    return
  end

  local is_anything_standing_on_plate = false

  for _, entity in pairs(cell) do
    if world_grid:is_solid(entity) then
      is_anything_standing_on_plate = true
      break
    end
  end

  for _, entity in pairs(linked_entities) do
    local lock_kind = world_grid:get_lock_kind(entity)
    local is_locked = lock_kind ~= nil

    if is_anything_standing_on_plate and is_locked then
      game_events:add({ type = GameEventType.DoorUnlock, data = entity })
    elseif not is_anything_standing_on_plate and not is_locked then
      game_events:add({ type = GameEventType.DoorLock, data = entity })
    end
  end
end

return M
