from requests import Session
def make() -> Session:
    return Session()
def fetch(u):
    s: Session = make()
    return s.get(u)
