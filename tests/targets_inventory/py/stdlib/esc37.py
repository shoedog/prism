import pathlib as paths
paths.PosixPath.read_text = lambda self: "replacement"
def make(path) -> paths.Path:
    return paths.Path(path)
def read(path):
    p = make(path)
    return p.read_text()
