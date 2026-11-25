from requests import Session
def fetch(u, factory=Session):
    client = factory()
    return client.get(u)
