from pathlib import Path
class Other:
    def exists(self):
        return True

def f(path: Path):
    type(path).other = (path := Other())
    return path.exists()
