from requests import Session
class Client:
    def __init__(self):
        self.s = Session()
    def replace(self, other):
        setattr(self, "ｓ", other)
    def fetch(self, u):
        return self.s.get(u)
