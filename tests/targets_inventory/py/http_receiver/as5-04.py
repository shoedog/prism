from requests import Session
class API:
    def fetch(self, replacement, u):
        local = self.client = Session()
        self.client = replacement
        local.get(u)
        return self.client.get(u)
