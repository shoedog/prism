import pathlib
def f():
    p = pathlib.Path("x")
    return p.write_bytes()
