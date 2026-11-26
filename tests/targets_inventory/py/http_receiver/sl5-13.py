import requests
class Client:
    def __init__(self):
        self.session = requests.Session()
    def f(self, u):
        self.session: object
        return self.session.get(u)
