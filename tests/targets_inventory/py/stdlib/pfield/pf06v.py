from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path
        setattr(self, "caf\N{LATIN SMALL LETTER E WITH ACUTE}", v)

    def is_new(self):
        return self.path.exists()
