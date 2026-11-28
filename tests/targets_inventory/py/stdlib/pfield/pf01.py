from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

    def save(self, text):
        return self.path.write_text(text)

def local():
    p = Path('x')
    return p.exists()
