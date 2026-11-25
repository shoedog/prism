import builtins
def read(p, replacement):
    builtins.__dict__["open"] = replacement
    return open(p)
