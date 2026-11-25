local function values()
  local first, second = select(2, io.read("*l"), os.getenv("EDITION"), "sample")
  return first, second
end
