from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path
        alias = self.path
        alias.exists = replacement

    def is_new(self):
        return self.path.exists()

    def save(self, text):
        return self.path.write_text(text)
