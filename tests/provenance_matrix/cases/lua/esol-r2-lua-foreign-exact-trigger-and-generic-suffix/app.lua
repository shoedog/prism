local function getenv(key)
    return string.upper(key)
end
local function fetchCaption(key)
    return string.lower(key)
end
local function load()
    local upper = getenv("Catalog")
    local lower = fetchCaption("Catalog")
    return upper, lower
end
