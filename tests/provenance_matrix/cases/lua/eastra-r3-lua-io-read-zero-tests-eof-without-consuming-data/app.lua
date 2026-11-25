local function label()
    local probe = io.read(0)
    if probe == nil then return end
    local word = io.read("*l")
    return probe, word
end
return label
