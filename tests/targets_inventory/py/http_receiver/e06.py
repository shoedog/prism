import requests


class Client:
    def __init__(self):
        self.s = requests.Session()

    def reset(self, other):
        self.s, self.t = other, None

    def fetch(self, u):
        return self.s.get(u)
