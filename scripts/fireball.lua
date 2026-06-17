local M = {}

---@param world_grid WorldGrid
---@param fireball Entity
function M.update(world_grid, _, fireball)
  local facing_dir = world_grid:get_facing_dir(fireball)
  if facing_dir == nil then
    return
  end

  -- NOTE: Сущность не удалится если она движется в сторону левой или верхней границы.
  --       Пока не знаю как это починить.
  world_grid:add_action(fireball, {
    type = ActionType.Move,
    data = {
      dir = facing_dir,
      despawn_if_collided = true,
    },
  })
end

return M
