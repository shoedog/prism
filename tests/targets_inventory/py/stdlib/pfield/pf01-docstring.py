from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        """store a path"""
        super().__init__()
        pass
        self.path = path

    def is_new(self):
        return self.path.exists()
