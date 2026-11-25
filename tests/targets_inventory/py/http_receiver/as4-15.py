from requests import Session
class API:
    def fetch(self, other, u):
        self.client = Session()
        alias = self
        alias.client = other
        return self.client.get(u)
