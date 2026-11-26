import requests
class C:
    def __init__(self):
        self.session = requests.Session()
    def fetch(self, headers, u):
        self.session.headers = headers
        return self.session.get(u)
