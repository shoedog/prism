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

class D(typing.Generic[int],  # a comment among the bases
        (Other)[str]):
    @property
    def path(self):
        return Other()
