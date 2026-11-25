import requests
class C:
    def __init__(self):
        self.s = requests.Session()
    def f(self):
        return self.s.get("u")
    def g(self, value):
        self.__dict__["s"] = value
