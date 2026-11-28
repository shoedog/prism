import pathlib as pl
class C(dict):
    def __init__(self, path: pl.Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()
