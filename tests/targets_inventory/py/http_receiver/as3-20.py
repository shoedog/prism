from httpx import Client
def fetch(u):
    with Client() as client, Client() as client:
        return client.get(u)
