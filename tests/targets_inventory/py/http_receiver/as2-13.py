from httpx import Client
def fetch(u):
    with Client() as client:
        pass
    return client.get(u)
