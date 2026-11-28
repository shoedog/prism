from pathlib import Path
class C(dict):
    def __init__(self: Path):
        super().__init__()
        self.path = self

    def is_new(self):
        return self.path.exists()
