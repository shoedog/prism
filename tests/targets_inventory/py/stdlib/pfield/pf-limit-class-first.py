from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

    def __new__(self):
        return self.path.exists()

    def __init_subclass__(self):
        return self.path.exists()

    def __class_getitem__(self, item):
        return self.path.exists()
