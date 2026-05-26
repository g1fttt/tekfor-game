local path = { Direction.East, Direction.West, Direction.South, Direction.South }

for _, dir in pairs(path) do
  move_player(dir)
end
