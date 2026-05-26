-- Trying to open the door
interact(Direction.South)

-- No success? Planning a path in order to solve a primitive puzzle
local path = { Direction.East, Direction.West, Direction.South, Direction.South }

-- Moving the player in the defined route
for _, dir in pairs(path) do
  move_player(dir)
end
