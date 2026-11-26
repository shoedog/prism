import requests


class Client:
    def __init__(self):
        self.s = requests.Session()

    def set_value(self, name, x):
        setattr(self, name, x)

    def fetch(self, u):
        return self.s.get(u)
