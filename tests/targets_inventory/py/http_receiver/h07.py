import requests


class Client:
    def __init__(self):
        self.s = requests.Session()

    def fetch(self, u):
        return self.s.get(u)
