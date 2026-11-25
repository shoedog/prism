from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    return [client.get(u) for client in (make(),)]
