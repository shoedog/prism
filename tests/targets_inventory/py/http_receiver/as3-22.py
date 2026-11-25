from httpx import Client
def Session() -> Client:
    return Client()
def fetch(u):
    client = Session()
    return client.get(u)
