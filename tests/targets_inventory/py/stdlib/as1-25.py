import builtins
from os import open
def read(path, raw):
    if raw:
        return open(path, 0)
    return builtins.open(path)
