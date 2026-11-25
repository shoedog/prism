import requests
class C:
    def __init__(self):
        self.session = requests.Session()
    def f(self, u):
        return self.session.get(u)
    def update(self, value):
        setattr(self, "cache", value)
