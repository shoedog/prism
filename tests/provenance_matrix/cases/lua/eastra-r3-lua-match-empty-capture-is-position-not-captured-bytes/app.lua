local function label()
    local line = io.read("*l")
    if not line then return end
    local start, word = line:match("()(%a+)")
    if not word then return end
    return start, word
end
return label
