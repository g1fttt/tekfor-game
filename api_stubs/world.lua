---@meta

---@class WorldGrid
WorldGrid = {}

---@param pos UVec2
---@return boolean
function WorldGrid:has_obstacle_at(pos) end

---@param entity Entity
---@return boolean
function WorldGrid:is_player(entity) end

---@param entity Entity
---@return boolean
function WorldGrid:is_solid(entity) end

---@param pos UVec2
---@param facing_dir Direction
---@return Entity
function WorldGrid:spawn_fireball(pos, facing_dir) end

---@param pos UVec2
---@return Entity
function WorldGrid:spawn_unlocked_door(pos) end

---@param entity Entity
---@return Bouncing?
function WorldGrid:switch_bouncing_dir(entity) end

---@param entity Entity
---@param action Action
function WorldGrid:add_action(entity, action) end

---@param entity Entity
---@return UVec2?
function WorldGrid:get_pos(entity) end

---@param entity Entity
---@return Direction?
function WorldGrid:get_facing_dir(entity) end

---@param entity Entity
---@return LockKind?
function WorldGrid:get_lock_kind(entity) end

---@param entity Entity
---@return Bouncing?
function WorldGrid:get_bouncing(entity) end

---@param entity Entity
---@return Entity[]?
function WorldGrid:get_linked_entities(entity) end

---@param pos UVec2
---@return Entity[]
function WorldGrid:get_cell(pos) end
