import builtins
def read(p, replacement):
    builtins.open = replacement
    return open(p)
