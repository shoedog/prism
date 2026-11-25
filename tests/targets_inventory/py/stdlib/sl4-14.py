from pathlib import Path
def f(src: Path):
    return int(src.stat().st_mtime)
