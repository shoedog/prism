local entries = {os.getenv("EDITION"), io.read("*l"), "preview"}
table.move(entries, 1, 2, 2)
local second = entries[2]
local third = entries[3]
return second, third
