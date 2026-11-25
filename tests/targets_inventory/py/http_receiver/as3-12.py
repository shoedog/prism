from requests import Session
from httpx import Client
class API:
    def fetch(self, u):
        self.client = Session()
        return self.client.get(u)
class API:
    def fetch(self, u):
        self.client = Client()
        return self.client.get(u)
