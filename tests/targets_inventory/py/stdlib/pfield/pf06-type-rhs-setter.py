from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

def g(c):
    type(c).other = setattr(c, "path", Other())

class Unrelated:
    def __init__(self, q: Path):
        self.q = q

    def is_new(self):
        return self.q.exists()


def local():
    return Path("x").exists()
