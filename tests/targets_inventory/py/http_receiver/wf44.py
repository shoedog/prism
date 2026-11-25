from httpx import Client
def fetch(client: Client, u, replacement):
    setattr(client, "get", replacement)
    return client.get(u)
