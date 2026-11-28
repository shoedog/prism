from pathlib import Path
class Other:
    def exists(self):
        return True

class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        type(self).other = (self := Other())
        return self.path.exists()
