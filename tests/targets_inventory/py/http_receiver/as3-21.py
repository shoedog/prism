from requests import Session
class API:
    @classmethod
    def configure(cls):
        cls.client = Session()
    def fetch(self, u):
        return self.client.get(u)
