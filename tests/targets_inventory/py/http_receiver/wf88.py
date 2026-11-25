from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    class client:
        seed = make()
    return client.get(u)
