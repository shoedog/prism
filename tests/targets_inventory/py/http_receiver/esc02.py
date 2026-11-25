from requests import Session
def make() -> Session:
    return {}
def fetch(u):
    s = make()
    return s.get(u)
