from requests import Session
def make() -> object:
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
