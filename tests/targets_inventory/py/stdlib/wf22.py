def read(p, replacement):
    __builtins__["open"] = replacement
    return open(p)
