from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

C = deco(C)

@deco
class Decorated:
    def __init__(self, path: Path):
        self.decorated_path = path

    def is_new(self):
        return self.decorated_path.exists()
