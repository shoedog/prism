from pathlib import PosixPath
def read(path):
    return PosixPath(path).read_text()
