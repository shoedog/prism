from requests import Session
class Base:
    def __init__(self):
        self.client = Session()
    def fetch(self, u):
        return self.client.get(u)
