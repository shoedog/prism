from pathlib import Path
def check(path: Path):
    path = Path(path)
    return path.is_file()
