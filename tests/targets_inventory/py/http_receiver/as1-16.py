from httpx import Client
class API:
    from requests import Session as Client
    def fetch(self, raw, u):
        client: Client = raw
        return client.get(u)
