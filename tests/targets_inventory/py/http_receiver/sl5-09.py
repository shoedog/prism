import requests
class Client:
    session = requests.Session()
    def f(self, u):
        return self.session.get(u)
