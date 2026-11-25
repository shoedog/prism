from requests import Session
def make() -> Session:
    return Session()
client = make()
def fetch(u):
    return client.get(u)
