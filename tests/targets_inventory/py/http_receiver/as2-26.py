from requests import Session
class API:
    def __init__(self, raw):
        self.client: Session = raw
    def fetch(self, u):
        return self.client.get(u)
