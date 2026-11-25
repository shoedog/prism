import requests
class C:
    def __init__(self):
        self.s = requests.Session()
    def f(self):
        setattr(self, "s", None)
        return self.s.get("u")
