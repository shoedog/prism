from pathlib import Path
from typing import TypeVar
T = TypeVar("T")
class C(dict):
    def __init__(self, path: Path):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()

class Other(dict):
    pass

class D(Other[T][int]):
    @property
    def path(self):
        return Other()
