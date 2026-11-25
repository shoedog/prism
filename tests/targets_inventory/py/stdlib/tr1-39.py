import pathlib
def f():
    p = pathlib.Path("x")
    return p.symlink_to()
