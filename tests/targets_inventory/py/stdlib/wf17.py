import builtins as b
def read(p, replacement):
    setattr(b, "open", replacement)
    return open(p)
