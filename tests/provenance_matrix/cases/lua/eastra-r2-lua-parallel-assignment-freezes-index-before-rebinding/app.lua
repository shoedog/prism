local slot = "left"
local entries = {left = "old", right = "preview"}
local edition = os.getenv("EDITION")
if edition == nil then return end
slot, entries[slot] = "right", edition
local updated, untouched = entries.left, entries.right
return updated, untouched
