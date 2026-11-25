from requests import Session
def make():
    return Session()
def fetch(u):
    with make() as client:
        return client.get(u)
