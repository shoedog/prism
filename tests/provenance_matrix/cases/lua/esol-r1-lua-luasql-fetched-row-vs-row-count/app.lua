local function load(dsn)
  local driver = require("luasql.postgres").postgres()
  local conn = assert(driver:connect(dsn))
  local cursor = assert(conn:execute("select 'desk' as label"))
  local count = cursor:numrows()
  local record = cursor:fetch({}, "a")
  cursor:close()
  conn:close()
  driver:close()
  return record, count
end
