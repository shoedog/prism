__builtins__ = {"open": lambda path: path}
def read(path):
    return open(path)
