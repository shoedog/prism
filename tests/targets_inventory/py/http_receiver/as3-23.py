from httpx import Client
def fetch(u):
    return Client().__enter__().get(u)
