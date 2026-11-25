from .provider import client
def fetch(u):
    return client.get(u)
