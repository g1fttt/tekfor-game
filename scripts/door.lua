local M = {}

---@param world_grid WorldGrid
---@param game_events GameEventManager
---@param door Entity
function M.interact(world_grid, game_events, door)
  if world_grid:get_lock_kind(door) == LockKind.Basic then
    -- TODO: Какая-то логика
    return
  else
    game_events:add({
      type = GameEventType.DoorOpen,
      data = door,
    })
  end
end

return M
