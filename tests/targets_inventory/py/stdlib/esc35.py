from pathlib import Path, PosixPath
PosixPath.read_text = lambda self: "replacement"
def read(path):
    return Path(path).read_text()
