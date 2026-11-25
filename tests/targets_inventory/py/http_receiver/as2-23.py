from requests import Session
def make():  # type: () -> Session
    return Session()
def fetch(u):
    client = make()
    return client.get(u)
