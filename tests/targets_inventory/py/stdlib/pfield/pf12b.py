from requests import Session
class C(dict):
    def __init__(self, path):
        super().__init__()
        self.path = Session()

    def is_new(self):
        return self.path.get(url)
