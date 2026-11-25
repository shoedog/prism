import requests
class Client:
    def __init__(self):
        self.session = None
    def __init__(self):
        self.session = requests.Session()
    def f(self, u):
        return self.session.get(u)
