from pathlib import Path

class Conditional:
    if True:
        def __init__(self, path: Path):
            self.path = path

    def is_new(self):
        return self.path.exists()

class Direct:
    def __init__(self, path: Path):
        self.direct_path = path

    def is_new(self):
        return self.direct_path.exists()
