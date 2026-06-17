---@meta

---@enum Direction
Direction = {
  North = "North",
  East = "East",
  South = "South",
  West = "West",
}

---@class UVec2
---@field x number
---@field y number
UVec2 = {}

---@enum ActionType
ActionType = {
  Move = "Move",
}

---@class MoveOptions
---@field dir Direction
---@field can_push boolean?
---@field despawn_if_collided boolean?
MoveOptions = {}

---@class Action
---@field type ActionType
---@field data MoveOptions
Action = {}
