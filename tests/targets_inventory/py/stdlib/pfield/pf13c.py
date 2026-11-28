from pathlib import Path
class C(dict, metaclass=Meta):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()
