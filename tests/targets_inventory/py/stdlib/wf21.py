def read(p, replacement):
    globals()["open"] = replacement
    return open(p)
