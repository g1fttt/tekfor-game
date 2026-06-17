---@meta

---@enum GameEventType
GameEventType = {
  DoorLock = "DoorLock",
  DoorUnlock = "DoorUnlock",
  DoorOpen = "DoorOpen",
  EntityWentDownstairs = "EntityWentDowntairs",
  EntityDeath = "EntityDeath",
}

---@class GameEvent
---@field type GameEventType
---@field data Entity | { target: Entity, attacker: Entity } | nil
GameEvent = {}

---@class GameEventManager
GameEventManager = {}

---@param event GameEvent
function GameEventManager:add(event) end

---@return GameEvent[]
function GameEventManager:iter() end
