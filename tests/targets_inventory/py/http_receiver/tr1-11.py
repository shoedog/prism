import requests
class C:
    def __init__(self):
        self.s = requests.Session()
    def f(self):
        del self.s
        return self.s.get("u")
