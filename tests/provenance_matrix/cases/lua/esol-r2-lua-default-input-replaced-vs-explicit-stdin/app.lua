local function load()
    local previous = io.input()
    local fixed = assert(io.tmpfile())
    fixed:write("fixed\n")
    fixed:seek("set", 0)
    io.input(fixed)
    local local_line = io.read("*l")
    io.input(previous)
    fixed:close()
    local incoming = io.stdin:read("*l")
    return local_line, incoming
end
