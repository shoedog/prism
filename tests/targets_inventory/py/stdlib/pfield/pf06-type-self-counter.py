from pathlib import Path
class C:
    def __init__(self):
        type(self).counter = 0

    def run(self, path: Path):
        return path.exists()

def g(path: Path):
    return path.exists()
