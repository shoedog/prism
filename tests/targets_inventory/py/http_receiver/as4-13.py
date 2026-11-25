from requests import Session
class API:
    @property
    def client(self):
        return Session()
    def fetch(self, u):
        return self.client.get(u)
