from __future__ import annotations
from pathlib import Path as P
class C(dict):
    def __init__(self, path: ((P))):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()
