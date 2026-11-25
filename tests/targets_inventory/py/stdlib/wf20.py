import builtins as b
def read(p, replacement):
    vars(b)["open"] = replacement
    return open(p)
