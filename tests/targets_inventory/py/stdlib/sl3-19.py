def install(fn):
    global open
    [(open := fn) for _ in (0,)]
def read(p):
    return open(p)
