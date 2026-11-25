local standard_string = string
local _ENV = {
  os = { getenv = function(key) return standard_string.lower(key) end }
}
local text = os.getenv("PREVIEW")
return text
