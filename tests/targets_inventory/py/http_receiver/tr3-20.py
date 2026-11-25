import requests
class C:
    def __init__(self):
        self.s = requests.Session()
    def f(self):
        return self.s.get("u")
    def g(self, name, value):
        setattr(self, name, value)
