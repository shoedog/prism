from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    match make():
        case client:
            return client.get(u)
