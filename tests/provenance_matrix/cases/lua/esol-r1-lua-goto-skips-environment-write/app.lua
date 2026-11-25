local function load()
  local mode = "fixed"
  goto ready
  mode = os.getenv("MODE")
  ::ready::
  return mode
end
