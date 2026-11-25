import builtins as b
def install(replacement):
    b.open = replacement
def read(p):
    b.open(p)
    return open(p)
