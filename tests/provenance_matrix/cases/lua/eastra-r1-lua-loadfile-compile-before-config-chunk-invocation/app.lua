local function edition()
  local chunk = assert(loadfile("settings.lua"))
  local options = chunk()
  local title = options.title
  return chunk, title
end
