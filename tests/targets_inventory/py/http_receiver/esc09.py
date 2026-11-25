from requests import Session
Session.__getattribute__ = lambda self, key: {}.get
def make() -> Session:
    return Session()
def fetch(u):
    s = make()
    return s.get(u)
