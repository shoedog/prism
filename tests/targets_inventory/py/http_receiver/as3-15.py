from requests import Session
def make() -> Session:
    if False:
        yield
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
