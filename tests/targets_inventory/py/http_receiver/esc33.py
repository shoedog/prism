from requests import Session
def make() -> Session:
    v = Session()
    x = v
    return v
def fetch(req):
    s = make()
    return s.send(req)
