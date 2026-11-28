import typing
from pathlib import Path
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

class Other(dict):
    pass

class D(typing.cast(type, C)):
    @property
    def path(self):
        return Other()
