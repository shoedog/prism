local function label()
    local box = setmetatable({}, {
        __newindex = function() end,
        __index = { edition = "preview" }
    })
    box.edition = os.getenv("EDITION")
    local value = box.edition
    return value
end
return label
