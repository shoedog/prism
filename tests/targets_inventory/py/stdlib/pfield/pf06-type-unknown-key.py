from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

def g(c):
    type(c).__dict__["caf\N{LATIN SMALL LETTER E WITH ACUTE}"] = 1
