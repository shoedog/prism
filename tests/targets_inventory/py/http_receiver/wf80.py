from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    ((client := make()), None)
    return client.get(u)
