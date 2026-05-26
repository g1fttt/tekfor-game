---@meta

---@enum Direction
Direction = {
  North = "north",
  East = "east",
  South = "south",
  West = "West",
}

--- Moves the player towards the given direction.
---@param dir Direction
function move_player(dir) end

--- Interacts with an entity in the given direction
---@param dir Direction
function interact(dir) end
