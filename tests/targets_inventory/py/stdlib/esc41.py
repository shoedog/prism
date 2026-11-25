from pathlib import Path
def make(flag) -> Path | None:
    if flag:
        return None
    return None
def read(flag):
    p = make(flag)
    return p.read_text()
