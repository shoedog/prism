import pathlib
def f():
    p = pathlib.Path("x")
    return p.read_bytes()
