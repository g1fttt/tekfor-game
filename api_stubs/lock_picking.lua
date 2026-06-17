---@meta

---@enum LockKind
LockKind = {
  Basic = "basic",
}

---@alias LockData string

---@param kind LockKind
---@param data LockData
---@return boolean
function on_lock_pick(kind, data) end
