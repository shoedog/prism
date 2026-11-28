from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        self.path = path
        del self

    def is_new(self):
        return self.path.exists()
