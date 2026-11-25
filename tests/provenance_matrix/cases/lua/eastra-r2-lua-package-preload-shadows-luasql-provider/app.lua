package.loaded["luasql.sqlite3"] = nil
package.preload["luasql.sqlite3"] = function()
  return { sqlite3 = function()
    return { connect = function()
      return { execute = function() return "preview" end }
    end }
  end }
end
local sql = require("luasql.sqlite3")
local conn = sql.sqlite3():connect("catalog")
local result = conn:execute("select title from books")
return result
