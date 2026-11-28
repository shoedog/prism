from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        if done:
            return
        self.path = path

    def is_new(self):
        return self.path.exists()
