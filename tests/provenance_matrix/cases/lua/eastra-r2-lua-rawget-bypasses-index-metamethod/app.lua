local editions = setmetatable({}, {
  __index = function(_, key)
    return os.getenv(key)
  end
})
local inherited = editions.EDITION
local direct = rawget(editions, "EDITION")
return inherited, direct
