from pathlib import Path
def make() -> Path:
    return Path("x")
def f():
    p = make()
    return p.read_text()
