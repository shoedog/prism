local function caption()
  local config <close> = assert(io.open("preview.conf", "r"))
  local title = config:read("*l")
  local state = io.type(config)
  return title, state
end
return caption
