import pathlib
delattr(pathlib.Path, "read_text")
def direct(p):
    return pathlib.Path.read_text(p)
def receiver(p):
    path = pathlib.Path(p)
    return path.read_text()
def annotated(path: pathlib.Path):
    return path.read_text()
def make(p) -> pathlib.Path:
    return pathlib.Path(p)
def helper(p):
    path = make(p)
    return path.read_text()
