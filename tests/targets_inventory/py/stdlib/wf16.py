import builtins
def read(p):
    del builtins.open
    return open(p)
