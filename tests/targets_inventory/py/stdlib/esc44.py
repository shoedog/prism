from pathlib import Path
def make(load) -> Path | None:
    return load()
def read(load):
    p = make(load)
    return p.read_text()
