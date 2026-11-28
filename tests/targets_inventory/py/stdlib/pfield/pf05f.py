from pathlib import Path
class C(dict):
    def __init__(self, path: Path = default):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()
