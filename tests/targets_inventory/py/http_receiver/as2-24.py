from httpx import Client
def make() -> Client:
    from requests import Session as Client
    return Client()
def fetch(u):
    client = make()
    return client.get(u)
