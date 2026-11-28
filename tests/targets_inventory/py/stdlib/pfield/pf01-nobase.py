from pathlib import Path
class C:
    def __init__(self, path: Path):
        self.path = path

    def is_new(self):
        return self.path.exists()
