from pathlib import Path, PosixPath
PosixPath.write_text = lambda self, text: 0
def read(path):
    with Path(path) as p:
        return p.read_text()
