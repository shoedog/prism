from requests import Session
def make() -> Session:
    tmp = Session()
    return tmp
def fetch(u):
    s = make()
    return s.get(u)
