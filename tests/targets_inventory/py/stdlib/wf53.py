import pathlib
def fetch(p, replacement):
    cls = pathlib.Path
    cls.read_text = replacement
    path = pathlib.Path(p)
    return path.read_text()
