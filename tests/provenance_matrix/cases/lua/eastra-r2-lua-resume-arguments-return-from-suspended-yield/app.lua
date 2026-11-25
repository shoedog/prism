local thread = coroutine.create(function()
  local resumed = coroutine.yield(os.getenv("EDITION"))
  return resumed
end)
local ok, first = coroutine.resume(thread)
if not ok then return end
local done, second = coroutine.resume(thread, "preview")
if not done then return end
return first, second
