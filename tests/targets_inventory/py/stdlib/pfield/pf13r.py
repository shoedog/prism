from pathlib import PurePath
class C(dict):
    def __init__(self, path: PurePath):
        super().__init__()
        self.path = path

    def is_new(self):
        return self.path.exists()
