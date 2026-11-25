from requests import Session
class API:
    def __init__(self):
        self.client = Session()
    def clear(self):
        delattr(self, "client")
    def fetch(self, u):
        return self.client.get(u)
