local function next_line()
  local ok, value = pcall(io.read, "*l")
  if not ok then return end
  if value == nil then return end
  return value
end
