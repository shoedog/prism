from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        if path:
            self.path = path

    def is_new(self):
        return self.path.exists()
