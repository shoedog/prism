from pathlib import Path
class Outer:
    class C(dict):
        def __init__(self, path: Path):
            super().__init__()
            self.path = path

        def is_new(self):
            return self.path.exists()

class Other:
    class X:
        pass

class D(Other.X):

    @property
    def path(self):
        return object()
