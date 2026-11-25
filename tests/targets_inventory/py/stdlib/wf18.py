import builtins as b
def read(p):
    delattr(b, "open")
    return open(p)
