import os
def remove_path():
    delattr(os, "path")
def inspect(p):
    os.stat(p)
    return os.path.getmtime(p)
