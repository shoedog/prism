local function first_title(dsn)
  local env = require("luasql.postgres").postgres()
  local conn = assert(env:connect(dsn))
  local cursor = assert(conn:execute("select title from editions limit 1"))
  local target = {}
  if cursor:fetch(target, "a") == nil then
    cursor:close(); conn:close(); env:close()
    return
  end
  local title = target.title
  cursor:close(); conn:close(); env:close()
  return title
end
