local function run(mode)
  local outer = os.getenv('MODE')
  local function inner(mode)
    print(mode)
  end
  inner(outer)
end
