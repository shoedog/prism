from requests import Session
def make() -> Session:
    return Session()
make.__annotations__["return"] = object
def fetch(u):
    client = make()
    return client.get(u)
